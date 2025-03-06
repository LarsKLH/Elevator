use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;


use crate::memory as mem;
use crate::elevator_interface::{self as elevint, Direction};

use driver_rust::elevio::{self, elev::{self, Elevator}};

// The symbol # is used where the code is not yet implemented and needs to be done later, or i have questions about the code


// The main elevator logic. Determines where to go next and sends commands to the motor controller
// # (Todo) clean up references, clones and copies
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>) -> () {

    let mut prev_direction = elevint::Direction::Up; // Store the previous direction of the elevator, currently set to Up
    // Infinite loop checking for memory messages
    loop {

        memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
        let memory = memory_recieve_rx.recv().unwrap();
        let my_state = memory.state_list.get(&memory.my_id).unwrap();
        let my_movementstate = my_state.move_state;
        match my_movementstate {

            elevint::MovementState::Moving(dirn) => {
                prev_direction = dirn;
                // If the elevator is moving, we should check if we should stop using the floor sensor
                cbc::select! { 
                    recv(floor_sensor_rx) -> a => {
                        // Update the last floor in memory
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnFloor(a.unwrap())).unwrap();

                        //println!("New floor received, checking whether or not to stop");
                        if should_i_stop(a.unwrap(), my_state) {
                            // Send StopAndOpen to memory to stop the elevator and open the door
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopAndOpen)).unwrap();
                        }
                        else {
                            // If we should continue, send the current movement state to memory
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(dirn))).unwrap();
                        }
                    }
                    recv(cbc::after(Duration::from_millis(100))) -> _a => {

                        //println!("No new floor received, refreshing");
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            } 
            elevint::MovementState::StopDoorClosed => {
                //println!("Stopping and closing door");
                let going = should_i_go(my_state.clone(), prev_direction, memory_request_tx.clone());
                if going {
                    println!("Moving again");
                }
            }
            elevint::MovementState::StopAndOpen => {
                //println!("Stopping and opening door");
                clear_call(my_state.clone(),  memory_request_tx.clone(), prev_direction);    
                let going = should_i_go(my_state.clone(), prev_direction, memory_request_tx.clone());
                if going {
                    println!("Moving again");
                }
            }
            elevint::MovementState::Obstructed => {
                println!("Elevator is obstructed");
                let going = should_i_go(my_state.clone(), prev_direction, memory_request_tx.clone());
                if going {
                    println!("Moving again"); // dont allow for ANY movement until the obstruction is removed
                }       
                
            }
        }
    }
}

// Check if the elevator should stop or not
fn should_i_stop(new_floor: u8, my_state: &mem::State) -> bool {

    let calls: Vec<_> = my_state.call_list.clone().into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = new_floor;
    let my_direction: elevint::Direction = match my_state.move_state {
        elevint::MovementState::Moving(dirn) => dirn,
        _ => {                                                            // This should never happen
            //println!("Error: Elevator is not moving. Defaulting to Up."); 
            elevint::Direction::Up                                        // Provide a fallback value
        }
    };


    // Check if my current floor is confirmed using filter -> stop
    let my_call_is_confirmed = calls.iter()
        .any(|(call, state)| *state == mem::CallState::Confirmed && call.floor == my_floor);
    
    if my_call_is_confirmed {
        return true;
    }

    
    // Check if there are no confirmed floors in the direction of the elevator -> stop
    let no_confirmed_calls_in_direction = calls.iter()
        .filter(|(call, state)| *state == mem::CallState::Confirmed) // Keep only confirmed calls
        .all(|(call, _)| match my_direction {                   // #should maybe use .any() instead of .all() here
            elevint::Direction::Up => call.floor <= my_floor,
            elevint::Direction::Down => call.floor >= my_floor,
        });
    
    if no_confirmed_calls_in_direction {
        return true;                    
    }

    // Else continue moving in current direction
    return false;

}

// Check if the elevator should continue moving or not
fn should_i_go(my_state: mem::State, mut prev_dir: Direction, memory_request_tx: Sender<mem::MemoryMessage> ) -> bool {

    // This function check both cab calls and hall calls for determining the next movement of the elevator
    // # (Todo) Also needs to check if another elevator is closer to the call than this elevator
    //          May need to use the distance function from the memory.rs file ??
    // # (Todo) Also needs to tidy up if statements to match statements and/or clean up number of cab_calls and hall_calls variables

    //println!("Checking if I should go");
    let calls: Vec<_> = my_state.call_list.clone().into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = my_state.last_floor;

    // Check if elevator holds any cab or hall calls
    let cab_calls = calls.iter()
        .any(|(call, state)| call.call_type == mem::CallType::Cab && *state == mem::CallState::Confirmed);

    let cab_calls_in_prev_dir = calls.iter()
        .any(|(call, state)| call.call_type == mem::CallType::Cab && *state == mem::CallState::Confirmed && (call.floor > my_floor && prev_dir == Direction::Up) || (call.floor < my_floor && prev_dir == Direction::Down));

    let hall_calls = calls.iter()
        .any(|(call, state)| (call.call_type == mem::CallType::Hall(Direction::Up) || call.call_type == mem::CallType::Hall(Direction::Down)) && *state == mem::CallState::Confirmed);

    let hall_calls_in_prev_dir = calls.iter()
        .any(|(call, state)| (call.call_type == mem::CallType::Hall(Direction::Up) || call.call_type == mem::CallType::Hall(Direction::Down)) && *state == mem::CallState::Confirmed && (call.floor > my_floor && prev_dir == Direction::Up) || (call.floor < my_floor && prev_dir == Direction::Down));

    if cab_calls {
        // If there are cab calls, we should maybe start moving
        // Move in the direction of previous call
        if cab_calls_in_prev_dir {
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).unwrap();
            println!("Moving in same direction");
            return true;
        }
        else {
            // Move in the direction of the other cab call (turning around) and switch the privious direction
            match prev_dir {
                Direction::Up => prev_dir = Direction::Down,
                Direction::Down => prev_dir = Direction::Up,
            }
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).unwrap();
            println!("Turning around");
            return true;
        }
    }

    // We might add logic for checking if another elevator is closer to the call than this elevator. But do it later
    else if hall_calls {
        // If there are hall calls and no cab calls, we should maybe start moving in same direction as before
        if hall_calls_in_prev_dir {
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).unwrap();
            println!("Moving in same direction");
            return true;
        }
        else {
            // Move in the direction of the other hall call (turning around) and switch the privious direction
            match prev_dir {
                Direction::Up => prev_dir = Direction::Down,
                Direction::Down => prev_dir = Direction::Up,
            }
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).unwrap();
            println!("Turning around");
            return true;
        }

    } else {
        // If there are no calls, we should do nothing
        return false;
        };

}

// Clear the call from the memory
fn clear_call(my_state: mem::State,  memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) -> () {
    use std::collections::HashMap;

let confirmed_calls_on_my_floor_with_same_direction: HashMap<mem::Call, mem::CallState> = my_state.call_list.clone()
    .into_iter()
    .filter(|(call, state)| {
        call.floor == my_state.last_floor &&
        *state == mem::CallState::Confirmed &&
        (call.call_type == mem::CallType::Hall(prev_dir) || call.call_type == mem::CallType::Cab)
    })
    .collect(); // Collect into a HashMap

// Change CallState of each call to PendingRemoval
for (call, _) in confirmed_calls_on_my_floor_with_same_direction {
    memory_request_tx
        .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
        .unwrap();
}

    // Wait 3 seconds
    thread::sleep(Duration::from_secs(3));              // Figure out how to do this without sleeping
    // Update MoveState to StopDoorClosed
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).unwrap();
}



/*fn restart(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {
    // TODO
    println!("Restarting elevator");
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();
    let my_state = memory.state_list.get(&memory.my_id).unwrap();
}*/