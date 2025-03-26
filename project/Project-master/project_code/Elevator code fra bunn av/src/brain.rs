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
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, brain_stop_direct_link: Sender<mem::State>) -> () {

    let mut prev_direction = elevint::Direction::Down; // Store the previous direction of the elevator, currently set to Down
    // Infinite loop checking for memory messages

    loop {

        memory_request_tx.send(mem::MemoryMessage::Request).expect("Error requesting memory");
        let memory = memory_recieve_rx.recv().expect("Error receiving memory");
        let mut my_state = memory.state_list.get(&memory.my_id).expect("Error getting own state").clone();
        let my_movementstate = my_state.move_state;
        match my_movementstate {

            elevint::MovementState::Moving(dirn) => {
                prev_direction = dirn;
                println!("Brain: Moving in direction {:?} and updating previous direction to {:?}", dirn, prev_direction);
                // If the elevator is moving, we should check if we should stop using the floor sensor
                cbc::select! { 
                    recv(floor_sensor_rx) -> a => {

                        //println!("New floor received, checking whether or not to stop");
                        if should_i_stop(a.expect("Error reading from floor sensor"), my_state.clone()) {
                            println!("Brain: Stopping and opening door");
                            my_state.last_floor = a.expect("Error reading from floor sensor");
                            my_state.move_state = elevint::MovementState::StopAndOpen;
                            brain_stop_direct_link.send(my_state).expect("Error sending stop and open to brain");
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
                }
            } 
            elevint::MovementState::StopDoorClosed => {
                //println!("Stopping and closing door");
                clear_call(my_state.clone(),  memory_request_tx.clone(), prev_direction);
                let going = should_i_go(prev_direction, memory_request_tx.clone(),my_state.clone());
                if going {
                    println!("Brain: Moving again after stoped with closed door");
                }
            }
            elevint::MovementState::StopAndOpen => {
                //println!("Stopping and opening door");
                thread::sleep(Duration::from_secs(3));
                clear_call(my_state.clone(),  memory_request_tx.clone(), prev_direction);    
                let going = should_i_go(prev_direction, memory_request_tx.clone(),my_state.clone());
                if going {
                    println!("Brain: Moving again after stoped with open door");
                }
            }
            elevint::MovementState::Obstructed => {
                println!("Brain: Elevator is obstructed");
                let going = should_i_go(prev_direction, memory_request_tx.clone(),my_state.clone());
                if going {
                    println!("Brain: Moving again after obstruction"); // dont allow for ANY movement until the obstruction is removed
                }  
                 
                
            }
            _ => {}
        }
    }
}

// Check if the elevator should stop or not
fn should_i_stop(floor_to_consider_stopping_at: u8, my_state: mem::State) -> bool {

    let calls: Vec<_> = my_state.call_list.into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = floor_to_consider_stopping_at;

    let my_direction: elevint::Direction = match my_state.move_state {
        elevint::MovementState::Moving(dirn) => dirn,
        _ => elevint::Direction::Up
            // This should never happen
            panic!("Error: Elevator is not moving, should not be checking if it should stop"),
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
        .any(|(call, state)| *state == mem::CallState::Confirmed && match my_direction {
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



// Clear the call from the memory
fn clear_call(my_state: mem::State,  memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) -> () {
    use std::collections::HashMap;

    // Jens: this seems like incredably overkill, isnt the only applicable calls in last_floor
    let confirmed_calls_on_my_floor_with_same_direction: HashMap<mem::Call, mem::CallState> =
    my_state.call_list.into_iter()
        .filter(|(call, state)| {
            //println!("Brain: Checking call {:?} at floor {} w/ state {:?}", call, my_state.last_floor, state);

            call.floor == my_state.last_floor &&
            *state == mem::CallState::Confirmed &&
            (matches!(call.call_type, mem::CallType::Hall(d) if d == prev_dir) || call.call_type == mem::CallType::Cab)
        })
        .collect(); // Collect into a HashMap

    if (my_state.last_floor == call floor && prev_dir == call_dir) {
        memory_request_tx
        .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
        .expect("Error sending call to memory");
    } else if (my floor == cab call floor) {
        memory_request_tx
        .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
        .expect("Error sending call to memory");
    } else {
        // do nothing
    }
                            
    println!("Brain: Want to clear all calls at my floor and in my direction, currently at floor {} with direction {:?}, calls to clear: {:?}", my_state.last_floor, prev_dir, confirmed_calls_on_my_floor_with_same_direction.clone());
                            

    
    // Update MoveState to StopDoorClosed
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).expect("Error sending movement state to memory");
    
}

fn should_i_go(current_dir: Direction, memory_request_tx: Sender<mem::MemoryMessage>, my_state: mem::State) -> bool {
    match my_state.move_state {
        elevint::MovementState::Obstructed => {return false;}
        _ => {
            let calls: Vec<_> = my_state.call_list.into_iter()
                .collect();
            let my_floor = my_state.last_floor;
            let confirmed_calls: Vec<_> = calls.iter()
                .filter(|(call, state)| *state == mem::CallState::Confirmed)
                .collect();
            let calls_in_current_direction: Vec<_> = calls.iter()
                .filter(|(call, state)| *state == mem::CallState::Confirmed && match current_dir {
                    elevint::Direction::Up => call.floor > my_floor,
                    elevint::Direction::Down => call.floor < my_floor,
                    })
                .collect();
            let calls_in_opposite_direction: Vec<_> = calls.iter()
                .filter(|(call, state)| *state == mem::CallState::Confirmed && match current_dir {
                    elevint::Direction::Up => call.floor < my_floor,
                    elevint::Direction::Down => call.floor > my_floor,
                    })
                .collect();
            
            match confirmed_calls.is_empty() {
                true => {
                    //memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).expect("Error sending movement state to memory");
                    return false;
                }
                _ => ()
            }
            match calls_in_current_direction.is_empty() {
                false => {
                    println!("Brain: There are more calls in my current direction {:?} from before I stopped, continuing to move in that direction", current_dir);
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(current_dir))).expect("Error sending movement state to memory");
                    return true;
                }
                _ => ()
            }

            match calls_in_opposite_direction.is_empty() {
                false => {
                    println!("Brain: There are no more hall calls in my previous direction {:?} from before I stopped but there are calls in the other direction, turning around to move in other direction", current_dir);
                    let current_dir = match current_dir {
                        Direction::Up => Direction::Down,
                        Direction::Down => Direction::Up,
                    };
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(current_dir))).expect("Error sending movement state to memory");
                    clear_call(my_state.clone(),  memory_request_tx.clone(), current_dir);
                    return true;
                }
                _ => {}
            }

            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).expect("Error sending movement state to memory");
            return false;

        }
    }
}

fn am_i_best_elevator_to_respond(call: mem::Call, memory: mem::Memory, current_dir: Direction) -> bool {
    let my_id = memory.my_id;
    let my_floor = memory.state_list.get(&my_id).unwrap().last_floor;
    let current_dir = memory.state_list.get(&my_id).unwrap().move_state;
    let call_floor = call.floor;
    if (current_dir == elevint::MovementState::Moving(Direction::Up) && call_floor < my_floor)
        || (current_dir == elevint::MovementState::Moving(Direction::Down) && call_floor > my_floor)
        
    {
        return false;
    }

    return memory.am_i_closest(my_id, call_floor);
}

fn clear_confirmed_calls_on_floor_matching_direction(my_state: mem::State,  memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) -> () {
    
    let confirmed_calls_on_my_floor_with_same_direction: HashMap<mem::Call, mem::CallState> =
        my_state.call_list
            .into_iter()
            .filter(|(call, state)| {
                //println!("Checking call {:?} at floor {}", call, my_state.last_floor);

                call.floor == my_state.last_floor &&
                *state == mem::CallState::Confirmed &&
                (matches!(call.call_type, mem::CallType::Hall(d) if d == prev_dir) || call.call_type == mem::CallType::Cab)
            })
            .collect(); // Collect into a HashMap

            
    println!("Brain: Want to clear all calls at my floor and in my direction, currently at floor {} with direction {:?}, calls to clear: {:?}", my_state.last_floor, prev_dir, confirmed_calls_on_my_floor_with_same_direction.clone());
                        
    // Change CallState of each call to PendingRemoval
    for (call, _) in confirmed_calls_on_my_floor_with_same_direction {

        memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval)).expect("Error sending call to memory");
        
        }

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