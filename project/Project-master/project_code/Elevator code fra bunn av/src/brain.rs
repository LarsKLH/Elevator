use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::thread;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;
use crate::memory::{self as mem, Call, CallType};
use crate::elevator_interface::{self as elevint, Direction};
use driver_rust::elevio::{self, elev::{self, Elevator}};

// (Todo) clean up references and clones

// The main elevator logic. Determines where to go next and sends commands to the elevator interface
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, brain_stop_direct_link: Sender<mem::State>) -> () {

    let mut prev_direction = elevint::Direction::Down; // Store previous direction of elevator, default Down
    loop {
        //println!("Brain: Requesting memory");
        //thread::sleep(Duration::from_millis(100));
        memory_request_tx.send(mem::MemoryMessage::Request).expect("Error requesting memory");
        let memory = memory_recieve_rx.recv().expect("Error receiving memory");
        let mut my_state = memory.state_list.get(&memory.my_id).expect("Error getting own state").clone();
        let my_movementstate = my_state.move_state;
        match my_movementstate {

            elevint::MovementState::Moving(dirn) => {
                prev_direction = dirn;
                cbc::select! { 
                    recv(floor_sensor_rx) -> a => {
                        if should_i_stop(a.expect("Error reading from floor sensor"), &my_state) {
                            //println!("Brain: Stopping and opening door");
                            my_state.last_floor = a.expect("Error reading from floor sensor");
                            my_state.move_state = elevint::MovementState::StopAndOpen;
                            brain_stop_direct_link.send(my_state).expect("Error sending stop and open to brain");
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopAndOpen)).expect("Error sending stop and open to memory");
                            //println!("Brain: Stopped elevator with door open");
                            
                        }
                        else {
                            //println!("Brain: Continuing in same direction");
                            // If we should continue, send the current movement state to memory
                            // Jens : is this neccescary, if we want to continue in the same direction do we send the same back aggain?
                            //memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(dirn))).expect("Error sending movement state to memory");
                        }
                    }
                    default(Duration::from_millis(1000)) => {
                    }
                }
            } 
            elevint::MovementState::StopDoorClosed => {
                let going = should_i_go(&mut prev_direction, &memory_request_tx ,&my_state);
                if going {
                    //println!("Brain: Moving again after stoped with closed door");
                }
            }
            elevint::MovementState::StopAndOpen => {
                thread::sleep(Duration::from_secs(3));
                clear_call(&mut my_state,  &memory_request_tx, prev_direction);    
                let going = should_i_go(&mut prev_direction, &memory_request_tx ,&my_state);
                if going {
                    //println!("Brain: Moving again after stoped with open door");
                }
            }
            elevint::MovementState::Obstructed => {
                //println!("Brain: Elevator is obstructed");
                let going = should_i_go(&mut prev_direction, &memory_request_tx ,&my_state);
                if going {
                    //println!("Brain: Moving again after obstruction");
                }  
            }
            _ => {}
        }
    }
}

// Check if the elevator should stop or not | Todo: Maybe turn if into a match statement
fn should_i_stop(floor: u8, my_state: &mem::State) -> bool {
    let my_direction = match my_state.move_state {
        elevint::MovementState::Moving(dir) => dir,
        _ => return false, // Not moving, shouldn't be checking
    };

    let mut has_confirmed_call = false;
    let mut has_call_ahead = false;

    for (call, state) in &my_state.call_list {
        if *state == mem::CallState::Confirmed {
            if call.floor == floor {
                match call.call_type {
                    CallType::Cab => {
                        println!("Brain: Stopping at cab call at floor {}", floor);
                        return true; // Confirmed call at current floor
                    }
                    CallType::Hall(dir) => {
                        if dir == my_direction {
                            println!("Brain: Stopping at hall call {:?} at floor {}", my_direction, floor);
                            return true; // Confirmed call at current floor
                        }

                    }
                    _ => {}
                    
                }
            }
            if (my_direction == elevint::Direction::Up && call.floor > floor)
                || (my_direction == elevint::Direction::Down && call.floor < floor)
            {
                has_call_ahead = true;
            }
        }
    }

    !has_call_ahead // Stop if no confirmed calls in the current direction
}

// Clear the call from the memory
fn clear_call(my_state: &mut mem::State, memory_request_tx: &Sender<mem::MemoryMessage>, prev_dir: Direction) {
    let floor = my_state.last_floor;

    let cab_call = mem::Call { call_type: mem::CallType::Cab, floor };
    let hall_call = mem::Call { call_type: mem::CallType::Hall(prev_dir), floor };

    for &call in &[cab_call, hall_call] {
        if my_state.call_list.get(&call) == Some(&mem::CallState::Confirmed) {
            println!("Brain: Clearing call {:?} at floor {}", call, floor);
            memory_request_tx
                .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
                .expect("Error sending call update");
            *my_state.call_list.get_mut(&call).expect("Could not get mutable call to change") = mem::CallState::PendingRemoval;
        }
    }
}

// Check if the elevator should go or not
fn should_i_go(current_dir: &mut Direction, memory_request_tx: &Sender<mem::MemoryMessage>, my_state: &mem::State) -> bool {
    if my_state.move_state == elevint::MovementState::Obstructed {
        return false;
    }

    let my_floor = my_state.last_floor;
    let mut has_calls_ahead = false;
    let mut has_any_calls = false;

    for (call, state) in &my_state.call_list {
        match *state {
            mem::CallState::Confirmed => {
                has_any_calls = true;
                if (matches!(current_dir, elevint::Direction::Up) && call.floor > my_floor)
                    || (matches!(current_dir, elevint::Direction::Down) && call.floor < my_floor)
                        {
                            has_calls_ahead = true;
                        }
            },
            mem::CallState::PendingRemoval => {
                if call.floor == my_floor && (call.call_type == CallType::Cab || call.call_type == CallType::Hall(*current_dir)) {
                    //there is a call that has not been removed yet, cannot leave untill that is the case
                    return  false;
                }
            },
            _ => {}
        }  
    }

    match (has_any_calls, has_calls_ahead) {
        (false, _) => {
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).unwrap();
            false
        }
        (true, true) => {
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(*current_dir))).unwrap();
            true
        }
        (true, false) => {
            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopAndOpen)).unwrap();
            *current_dir = match *current_dir {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
            };
            true
        }
    }
}


// This function checks if the current elevator is the best one to respond to a call based on its state and the call's properties.
fn am_i_best_elevator_to_respond(call: mem::Call, memory: mem::Memory, current_dir: Direction) -> bool {
    let my_id = memory.my_id;
    let my_state = memory.state_list.get(&my_id).unwrap();
    let my_floor = my_state.last_floor;
    let my_calls = my_state.call_list.len();
    
    let call_floor = call.floor;
    let is_moving_towards = match my_state.move_state {
        elevint::MovementState::Moving(Direction::Up) => call_floor >= my_floor,
        elevint::MovementState::Moving(Direction::Down) => call_floor <= my_floor,
        _ => false,
    };

    // Compute a simple heuristic score
    let my_score = (call_floor as i32 - my_floor as i32).abs() as u32 // Distance weight
        + if is_moving_towards { 0 } else { 10 } // Favor elevators already moving in the right direction
        + (my_calls as u32 * 2); // Load balancing: Prefer elevators with fewer calls

    // Compare against all other elevators
    for (elev_id, elev_state) in &memory.state_list {
        if elev_state.move_state == elevint::MovementState::Obstructed || elev_state.timed_out == true {
            continue; // Skip obstructed or timed out elevators
        }
        if *elev_id == my_id {
            continue; // Skip self
        }

        let other_floor = elev_state.last_floor;
        let other_calls = elev_state.call_list.len();
        let other_is_moving_towards = match elev_state.move_state {
            elevint::MovementState::Moving(Direction::Up) => call_floor >= other_floor,
            elevint::MovementState::Moving(Direction::Down) => call_floor <= other_floor,
            _ => false,
        };

        let other_score = (call_floor as i32 - other_floor as i32).abs() as u32
            + if other_is_moving_towards { 0 } else { 10 }
            + (other_calls as u32 * 2);

        if other_score < my_score {
            return false; // Another elevator is a better choice
        }
    }

    true
}


/*fn restart(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {
    // TODO
    println!("Restarting elevator");
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();
    let my_state = memory.state_list.get(&memory.my_id).unwrap();
}*/