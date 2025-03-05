use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;


use crate::memory as mem;
use crate::elevator_interface as elevint;

use driver_rust::elevio::{self, elev::{self, Elevator}};


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
                //  Determine next direction using should_i_go and send to memory
            }
            elevint::MovementState::StopAndOpen => {
                println!("Stopping and opening door");
                // Change callstate to PendingRemoval in memory
                // Determine next direction using should_i_go and send to memory
            }

            // Need to add a case for the Obstructed state that 
            // dont allow for ANY movement until the obstruction is removed
            // Questions for later: Should the elevator remember the last direction it was moving in; Can obstructions occur both when
            // in motion and when standing still (with or without open doors);
            // See elevint file for clariity??

        }
    }
}

// Check whether we should stop or not
fn should_i_stop(new_floor: u8, my_state: &mem::State) -> bool {

    match my_state.move_state {
        elevint::MovementState::Moving(dirn) => {
            let check_call = mem::Call {
                call_type: mem::CallType::Hall(dirn),
                floor: new_floor
            };
        }
        _ => ()
    }
    // Move the if-statements bellow INSIDE the match statement if you can NOT access the check_call variable outside of the match statement
    // elsewise you can keep it as it is
    // If the state of our current floor is confirmed, we should stop
    if *my_state.call_list.get(&check_call).unwrap() == mem::CallState::Confirmed {
        return true;
    }
    // if there are no more floors below us, we should stop
    else if !lower_calls(new_floor, my_state.clone()) {
        return true;
    }
    else {
        return false;
    }
}

// Fix my_state.direction as it was done in the function above, or pass it as an argument
fn lower_calls(new_floor: u8, my_state: mem::State) -> bool {

    match my_state.move_state {
        elevio::elev::DIRN_UP => {
            for call in my_state.call_list {
                if call.0.floor > new_floor && call.1 == mem::CallState::Confirmed {
                    return false;
                }
            }
        }
        elevio::elev::DIRN_DOWN => {
            for call in my_state.call_list {
                if call.0.floor < new_floor && call.1 == mem::CallState::Confirmed {
                    return false;
                }
            }
        }
        0_u8|2_u8..=254_u8 => {
            println!("Error: Direction not valid");
        }
    }
    return true
}

// should_i_go, checks if the elevator should go up or down or if another elevator should take the call
fn should_i_go(my_state: mem::State) -> () {
    println!("Checking if I should go");
    // This function needs to check both cab_calls and call_list (I advice cab_calls take president over call_list) 
    // for determining the next direction of the elevator
    // Also needs to check if another elevator is closer to the call than this elevator
    // May need to use the distance function from the memory.rs file ??
    // May need to take the direction of the elevator into account when checking if another elevator is closer and if 
    // the elevator is moving or not (and which floor is more advantageous to go to)

}


/*fn restart(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {
    // TODO
    println!("Restarting elevator");
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();
    let my_state = memory.state_list.get(&memory.my_id).unwrap();
}*/