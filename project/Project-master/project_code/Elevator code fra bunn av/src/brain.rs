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


// The main elevator logic. Determines where to go next and sends commands to the elevator interface
// # (Todo) clean up references, clones and copies
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>) -> () {

    let mut prev_direction = elevint::Direction::Down; // Store the previous direction of the elevator, currently set to Down
    // Infinite loop checking for memory messages

    loop {

        memory_request_tx.send(mem::MemoryMessage::Request).expect("Error requesting memory");
        let memory = memory_recieve_rx.recv().expect("Error receiving memory");
        let my_state = memory.state_list.get(&memory.my_id).expect("Error getting own state");
        let my_movementstate = my_state.move_state;
        match my_movementstate {

            elevint::MovementState::Moving(dirn) => {
                prev_direction = dirn;
                // If the elevator is moving, we should check if we should stop using the floor sensor
                cbc::select! { 
                    recv(floor_sensor_rx) -> a => {
                        println!("\nBrain: Floor sensor detected, checking whether or not to stop");
                        // Update the last floor in memory
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnFloor(a.expect("Error reading from floor sensor"))).expect("Error updating floor");

                        //println!("New floor received, checking whether or not to stop");
                        if should_i_stop(a.expect("Error reading from floor sensor"), my_state) {
                            println!("Brain: Stopping and opening door");
                            // Send StopAndOpen to memory to stop the elevator and open the door
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopAndOpen)).expect("Error sending stop and open to memory");
                            println!("Brain: Stopped elevator with door open");
                        }
                        else {
                            println!("Brain: Continuing in same direction");
                            // If we should continue, send the current movement state to memory
                            // Jens : is this neccescary, if we want to continue in the same direction do we send the same back aggain?
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(dirn))).expect("Error sending movement state to memory");
                        }
                    }
                    recv(cbc::after(Duration::from_millis(100))) -> _a => {

                        // Jens: We do not need to spam the terminal with messages that does not communicate any information
                        // println!("Brain: No floor sensor detected, refreshing");
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            } 
            elevint::MovementState::StopDoorClosed => {
                //println!("Stopping and closing door");
                let going = should_i_go(my_state.clone(), prev_direction, memory_request_tx.clone());
                if going {
                    println!("Brain: Moving again after stoped with closed door");
                    thread::sleep(Duration::from_millis(100));

                }
            }
            elevint::MovementState::StopAndOpen => {
                //println!("Stopping and opening door");
                clear_call(my_state.clone(),  memory_request_tx.clone(), prev_direction);    
                let going = should_i_go(my_state.clone(), prev_direction, memory_request_tx.clone());
                if going {
                    println!("Brain: Moving again after stoped with open door");
                }
            }
            elevint::MovementState::Obstructed => {
                println!("Brain: Elevator is obstructed");
                thread::sleep(Duration::from_millis(100));

                let going = should_i_go(my_state.clone(), prev_direction, memory_request_tx.clone());
                if going {
                    println!("Brain: Moving again after obstruction"); // dont allow for ANY movement until the obstruction is removed
                    thread::sleep(Duration::from_millis(100));

                }  
                 
                
            }
        }
    }
}

// Check if the elevator should stop or not
fn should_i_stop(floor_to_consider_stopping_at: u8, my_state: &mem::State) -> bool {

    let calls: Vec<_> = my_state.call_list.clone().into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = floor_to_consider_stopping_at;
    let my_direction: elevint::Direction = match my_state.move_state {
        elevint::MovementState::Moving(dirn) => dirn,
        _ => {                                                            // This should never happen
            //println!("Error: Elevator is not moving. Defaulting to Up."); 
            elevint::Direction::Up                                        // Provide a fallback value

            // Jens: in this case shouldnt we just crash here? as something, somewere is wery wrong if we arrive at at a floor without moving
        }
    };


    // Check if my current floor is confirmed using filter, if so we should stop -> return true
    let my_call_is_confirmed = calls.iter()
        .any(|(call, state)| *state == mem::CallState::Confirmed && call.floor == my_floor);
    
    if my_call_is_confirmed {
        println!("Brain: There is a confirmed order on this floor, currently at floor: {} , stopping", my_floor);
        return true;
    }

    
    // Check if there are no confirmed floors in the direction of the elevator,
    // if there is not we should not continue in that direction and we should stop -> return true
    let confirmed_calls_in_direction = calls.iter()
        .any(|(call, state)| *state == mem::CallState::Confirmed && match my_direction {                   // #should maybe use .any() instead of .all() here
            elevint::Direction::Up => call.floor >= my_floor,
            elevint::Direction::Down => call.floor <= my_floor,
        });

    
    if !confirmed_calls_in_direction {
        println!("Brain: There is no more confiremed orders beyond this floor, currently at floor {}, stopping", my_floor);
        return true;                    
    }

    // Else continue moving in current direction
    println!("Brain: There is more confiremed orders beyond this floor, currently at floor {}, continuuing ", my_floor);
    return false;

}

// Check if the elevator should continue moving or not
fn should_i_go(my_state: mem::State, mut prev_dir: Direction, memory_request_tx: Sender<mem::MemoryMessage> ) -> bool {

    // This function check both cab calls and hall calls for determining the next movement of the elevator
    // # (Todo) Also needs to check if another elevator is closer to the call than this elevator
    //          May need to use the distance function from the memory.rs file ??
    // # (Todo) Also needs to tidy up if statements to match statements and/or clean up number of cab_calls and hall_calls variables

    let calls: Vec<_> = my_state.call_list.clone().into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = my_state.last_floor;

    match my_state.move_state {
        elevint::MovementState::Obstructed => {
            // If the elevator is obstructed, we should not move
            return false;
        }
        
        /*
        elevint::MovementState::StopAndOpen => {
            let any_calls_on_my_floor_pending_removal = calls.iter().any(|(call, state)| call.floor == my_floor)
        }
        */

        _ => {

            //println!("Checking if I should go");

            // Check if elevator holds any cab or hall calls
            let cab_calls = calls.iter()
                .any(|(call, state)| call.call_type == mem::CallType::Cab && *state == mem::CallState::Confirmed);

            let cab_calls_in_prev_dir = calls.iter()
                .any(|(call, state)| call.call_type == mem::CallType::Cab && *state == mem::CallState::Confirmed && ((call.floor > my_floor && prev_dir == Direction::Up) || (call.floor < my_floor && prev_dir == Direction::Down)));

            let hall_calls = calls.iter()
                .any(|(call, state)| (call.call_type == mem::CallType::Hall(Direction::Up) || call.call_type == mem::CallType::Hall(Direction::Down)) && *state == mem::CallState::Confirmed);

            let hall_calls_in_prev_dir = calls.iter()
                .any(|(call, state)| (call.call_type == mem::CallType::Hall(Direction::Up) || call.call_type == mem::CallType::Hall(Direction::Down)) && *state == mem::CallState::Confirmed && ((call.floor > my_floor && prev_dir == Direction::Up) || (call.floor < my_floor && prev_dir == Direction::Down)));

            if cab_calls {
                // If there are cab calls, we should maybe start moving
                // Move in the direction of previous call
                if cab_calls_in_prev_dir {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).expect("Error sending movement state to memory");
                    println!("Brain: I have cab calls in my prev direction {:?} from before I stopped, contimuuing to move in that direction", prev_dir);
                    thread::sleep(Duration::from_millis(100));

                    return true;
                }
                else {
                    // Move in the direction of the other cab call (turning around) and switch the privious direction
                    match prev_dir {
                        Direction::Up => prev_dir = Direction::Down,
                        Direction::Down => prev_dir = Direction::Up,
                    }
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).expect("Error sending movement state to memory");
                    println!("Brain: I have no more cab calls in my prev direction {:?} from before I stopped but I have cab calls in the other direction, turning around to move in that direction", prev_dir);
                    thread::sleep(Duration::from_millis(100));

                    return true;
                }
            }

            // We might add logic for checking if another elevator is closer to the call than this elevator. But do it later
            else if hall_calls {
                // If there are hall calls and no cab calls, we should maybe start moving in same direction as before
                if hall_calls_in_prev_dir {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).expect("Error sending movement state to memory");
                    println!("Brain: Ther are hall calls in my prev direction {:?} from before I stopped, contiuing to move in that direction", prev_dir);
                    thread::sleep(Duration::from_millis(100));
                    clear_call(my_state.clone(),  memory_request_tx.clone(), prev_dir);
                    return true;
                }
                else {
                    // Move in the direction of the other hall call (turning around) and switch the privious direction
                    match prev_dir {
                        Direction::Up => prev_dir = Direction::Down,
                        Direction::Down => prev_dir = Direction::Up,
                    }
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).expect("Error sending movement state to memory");
                    println!("Brain: There are no more hall calls in my prev direction {:?} from before I stopped but there are hall calls in the other direction, turning around to move in that direction", prev_dir);
                    thread::sleep(Duration::from_millis(100));
                    clear_call(my_state.clone(),  memory_request_tx.clone(), prev_dir);
                    return true;
                }

            } else {
                // If there are no confrimed calls, we should do nothing, but first set state to stopAndCloseDoor
                memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).expect("Error sending movement state to memory");
                
                // Jens: This one spammed the terminal with the same message, commenting it out
                //println!("Brain: There are no more calls to take, stoping with door open at floor {}", my_floor);
                
                thread::sleep(Duration::from_millis(100));
                return false;
                };/* 
                let has_calls = calls.iter().any(|(call, state)| *state == mem::CallState::Confirmed || *state == mem::CallState::PendingRemoval || *state == mem::CallState::New);
                if has_calls {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(prev_dir))).expect("Error sending movement state to memory");
                    };
                has_calls;*/

        }    

    }
}

// Clear the call from the memory
fn clear_call(my_state: mem::State,  memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) -> () {
    use std::collections::HashMap;

    // Jens: this seems like incredably overkill, isnt the only applicable calls in last_floor
    let confirmed_calls_on_my_floor_with_same_direction: HashMap<mem::Call, mem::CallState>
            = my_state.call_list.clone()
                                .into_iter()
                                .filter(|(call, state)| {
                                    call.floor == my_state.last_floor &&
                                    *state == mem::CallState::Confirmed &&
                                    (call.call_type == mem::CallType::Hall(prev_dir) || call.call_type == mem::CallType::Cab)
                                })
                                .collect(); // Collect into a HashMap
                            
    println!("Brain: Want to clear all calls at my floor and in my direction, currently at floor {} with direction {:?}, calls to clear: {:?}", my_state.last_floor, prev_dir, confirmed_calls_on_my_floor_with_same_direction.clone());
                            
    // Change CallState of each call to PendingRemoval
    for (call, _) in confirmed_calls_on_my_floor_with_same_direction {
        memory_request_tx
        .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
        .expect("Error sending call to memory");
                        }

    // Wait 3 seconds
    thread::sleep(Duration::from_secs(3));              // Figure out how to do this without sleeping
    // Jens: We should take note at the current time, and chack back and confirm that we have been stopped for long enough 
    
    // Update MoveState to StopDoorClosed
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).expect("Error sending movement state to memory");
    
}



/*fn restart(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {
    // TODO
    println!("Restarting elevator");
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();
    let my_state = memory.state_list.get(&memory.my_id).unwrap();
}*/