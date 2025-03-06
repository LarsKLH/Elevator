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
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>) -> () {

    // Infinite loop checking for memory messages
    loop {

        memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
        let memory = memory_recieve_rx.recv().unwrap();
        let my_state = memory.state_list.get(&memory.my_id).unwrap();
        let my_movementstate = my_state.move_state;
        match my_movementstate {

            elevint::MovementState::Moving(dirn) => {

                // If the elevator is moving, we should check if we should stop using the floor sensor
                cbc::select! { 

                    recv(floor_sensor_rx) -> a => {

                        println!("New floor received, checking whether or not to stop");
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

                        println!("No new floor received, refreshing");
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            } 
            elevint::MovementState::StopDoorClosed => {
                println!("Stopping and closing door");
                //  #Determine next direction using should_i_go and send to memory
            }
            elevint::MovementState::StopAndOpen => {
                println!("Stopping and opening door");
                // #Change callstate to PendingRemoval in memory
                // #Determine next direction using should_i_go and send to memory
            }

            elevint::MovementState::Obstructed => {
                println!("Elevator is obstructed");
                // #Determine next direction using should_i_go and send to memory         
                // dont allow for ANY movement until the obstruction is removed

            }

            // #Questions for later: Should the elevator remember the last direction it was moving in?
            // #Can obstructions occur both when in motion and when standing still (with or without open doors)?

        }
    }
}

// Check whether we should stop or not
fn should_i_stop(new_floor: u8, my_state: &mem::State) -> bool {

    let calls: Vec<_> = my_state.call_list.clone().into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = new_floor;
    let my_direction: elevint::Direction = match my_state.move_state {
        elevint::MovementState::Moving(dirn) => dirn,
        _ => {                                                            // This should never happen
            println!("Error: Elevator is not moving. Defaulting to Up."); 
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
        return true; // Stop the elevator
    }

    // Else continue moving in current direction
    return false;

}


// should_i_go, checks if the elevator should go up or down or if another elevator should take the call
fn should_i_go(my_state: mem::State) -> () {

    // #This function needs to check both cab_calls and call_list (I advice cab_calls take president over call_list) 
    // #for determining the next direction of the elevator
    // #Also needs to check if another elevator is closer to the call than this elevator
    // #May need to use the distance function from the memory.rs file ??
    // #May need to take the direction of the elevator into account when checking if another elevator is closer and if 
    // #the elevator is moving or not (and which floor is more advantageous to go to)

    println!("Checking if I should go");
    let calls: Vec<_> = my_state.call_list.clone().into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = my_state.last_floor;
    let my_movementstate = my_state.move_state;
    let my_direction = match my_movementstate {
        elevint::MovementState::Moving(dirn) => Some(dirn),
        _ => {
            println!("Error: Elevator is not moving.");
            None
        }
    };

    // Check if elevator is obstructed, 
    //#or maybe this should be done in the elevator_logic function so that the elevator does not move at all
    let is_obstructed = my_state.obstructed;

    // Check if elevator holds any cab or hall calls
    let cab_calls = calls.iter()
        .any(|(call, state)| call.call_type == mem::CallType::Cab && *state == mem::CallState::Confirmed);

    let hall_calls = calls.iter()
        .any(|(call, state)| call.call_type == mem::CallType::Hall && *state == mem::CallState::Confirmed);

    if cab_calls {
        // If there are cab calls, we should maybe start moving
        // Move in the direction of most advantageous cab call
    }

    if hall_calls {
        // If there are hall calls, we should maybe start moving
        // Move in the direction of most advantageous hall call
    }

    

    
}


/*fn restart(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {
    // TODO
    println!("Restarting elevator");
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();
    let my_state = memory.state_list.get(&memory.my_id).unwrap();
}*/