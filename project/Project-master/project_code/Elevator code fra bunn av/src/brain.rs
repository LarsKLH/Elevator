use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::thread;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;
use crate::memory as mem;
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
                        if should_i_stop(a.expect("Error reading from floor sensor"), my_state.clone()) {
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
                let going = should_i_go(&mut prev_direction, memory_request_tx.clone(),my_state.clone());
                if going {
                    //println!("Brain: Moving again after stoped with closed door");
                }
            }
            elevint::MovementState::StopAndOpen => {
                thread::sleep(Duration::from_secs(3));
                clear_call(my_state.clone(),  memory_request_tx.clone(), prev_direction);    
                let going = should_i_go(&mut prev_direction, memory_request_tx.clone(),my_state.clone());
                if going {
                    //println!("Brain: Moving again after stoped with open door");
                }
            }
            elevint::MovementState::Obstructed => {
                //println!("Brain: Elevator is obstructed");
                let going = should_i_go(&mut prev_direction, memory_request_tx.clone(),my_state.clone());
                if going {
                    //println!("Brain: Moving again after obstruction");
                }  
            }
            _ => {}
        }
    }
}

// Check if the elevator should stop or not | Todo: Maybe turn if into a match statement
fn should_i_stop(floor_to_consider_stopping_at: u8, my_state: mem::State) -> bool {

    let calls: Vec<_> = my_state.call_list.into_iter().collect(); // Store call_list as a vec for future filtering    
    let my_floor = floor_to_consider_stopping_at;

    let my_direction: elevint::Direction = match my_state.move_state {
        elevint::MovementState::Moving(dirn) => dirn,
        _ => panic!("Error: Elevator is not moving, should not be checking if it should stop"),
    };

    // Check if my current floor is confirmed, if so we should stop -> return true
    let my_call_is_confirmed = calls.iter()
        .any(|(call, state)| *state == mem::CallState::Confirmed && call.floor == my_floor);
    
    if my_call_is_confirmed {
        //println!("Brain: There is a confirmed order on this floor, currently at floor: {} , stopping", my_floor);
        return true;
    }

    /* Check if there are no confirmed floors in the direction of the elevator,
     if there is not we should not continue in that direction and we should stop -> return true*/
    let confirmed_calls_in_direction = calls.iter()
        .any(|(call, state)| *state == mem::CallState::Confirmed && match my_direction {
            elevint::Direction::Up => call.floor >= my_floor,
            elevint::Direction::Down => call.floor <= my_floor,
        });

    if !confirmed_calls_in_direction {
        //println!("Brain: There is no more confirmed orders beyond this floor, currently at floor {}, stopping", my_floor);
        return true;                    
    }

    // If no conditions are met, we should continue -> return false
    //println!("Brain: There is more confirmed orders beyond this floor, currently at floor {}, continuing ", my_floor);
    return false;

}
// Potential improvement of should_i_stop:
/*
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
                return true; // Confirmed call at current floor
            }
            if (my_direction == elevint::Direction::Up && call.floor > floor)
                || (my_direction == elevint::Direction::Down && call.floor < floor)
            {
                has_call_ahead = true;
            }
        }
    }

    !has_call_ahead // Stop if no confirmed calls in the current direction
} */

// Clear the call from the memory
fn clear_call(my_state: mem::State,  memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) -> () {
    let current_floor = my_state.last_floor;
    let cab_call_to_check = mem::Call { call_type: mem::CallType::Cab, floor: current_floor };

    let hall_call_to_check = mem::Call { call_type: mem::CallType::Hall(prev_dir), floor: current_floor};

    if my_state.call_list.get(&cab_call_to_check) == Some(&mem::CallState::Confirmed) {
        
        println!("Brain: Want to clear call {:?} at my floor {} and in my direction {:?}", cab_call_to_check, current_floor, prev_dir);

        memory_request_tx
        .send(mem::MemoryMessage::UpdateOwnCall(cab_call_to_check, mem::CallState::PendingRemoval))
        .expect("Error sending call to memory");
    }
    
    if my_state.call_list.get(&hall_call_to_check) == Some(&mem::CallState::Confirmed) {

        println!("Brain: Want to clear call {:?} at my floor {} and in my direction {:?}", hall_call_to_check, current_floor, prev_dir);
        
        memory_request_tx
        .send(mem::MemoryMessage::UpdateOwnCall(hall_call_to_check, mem::CallState::PendingRemoval))
        .expect("Error sending call to memory");
    } 
}

// Potential improvements of clear_call:
/*fn clear_call(my_state: mem::State, memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) {
    let current_floor = my_state.last_floor;
    let mut calls_to_clear = Vec::new();

    for (call, state) in &my_state.call_list {
        if call.floor == current_floor && *state == mem::CallState::Confirmed {
            calls_to_clear.push(call.clone());
        }
    }

    if !calls_to_clear.is_empty() {
        println!("Brain: Clearing {} calls at floor {}", calls_to_clear.len(), current_floor);

        for call in calls_to_clear {
            memory_request_tx
                .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
                .expect("Error sending call to memory");
        }
    }
} */
/* I like this one better (bellow)
fn clear_call(my_state: &mem::State, memory_request_tx: &Sender<mem::MemoryMessage>, prev_dir: Direction) {
    let floor = my_state.last_floor;

    let cab_call = mem::Call { call_type: mem::CallType::Cab, floor };
    let hall_call = mem::Call { call_type: mem::CallType::Hall(prev_dir), floor };

    for &call in &[cab_call, hall_call] {
        if my_state.call_list.contains_key(&call) {
            println!("Brain: Clearing call {:?} at floor {}", call, floor);
            memory_request_tx
                .send(mem::MemoryMessage::UpdateOwnCall(call, mem::CallState::PendingRemoval))
                .expect("Error sending call update");
        }
    }
} */

// Check if the elevator should go or not
fn should_i_go(current_dir: &mut Direction, memory_request_tx: Sender<mem::MemoryMessage>, my_state: mem::State) -> bool {
    //println!("Brain: Checking if I should go w/ current direction {:?} and movement state {:?}", current_dir, my_state.move_state);
    match my_state.move_state {
        elevint::MovementState::Obstructed => {return false;}
        _ => {

            let calls: Vec<_> = my_state.call_list.clone()
            .into_iter()
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
            
            match confirmed_calls.is_empty() {
                true => {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed)).expect("Error sending movement state to memory");
                    return false;
                }
                _ => {
                    match calls_in_current_direction.is_empty() {
                        false => {
                            //println!("Brain: There are more calls in my current direction {:?} from before I stopped, continuing to move in that direction", current_dir);
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(*current_dir))).expect("Error sending movement state to memory");
                            return true;
                        }
                        true => {
                            //println!("Brain: There are no more hall calls in my current direction {:?} from before I stopped but there are calls in the other direction, opening doors before turning around to move in other direction", current_dir);
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopAndOpen)).expect("Error sending movement state to memory");
                            *current_dir = match *current_dir {
                                Direction::Up => Direction::Down,
                                Direction::Down => Direction::Up,
                            };
                            return true;
                        }
                    }
                }
            }
        }
    }
}

// Possible improvements of should_i_go:
/*
fn should_i_go(current_dir: &mut Direction, memory_request_tx: Sender<mem::MemoryMessage>, my_state: mem::State) -> bool {
    let my_floor = my_state.last_floor;
    let calls: Vec<_> = my_state.call_list.iter()
        .filter(|(_, state)| **state == mem::CallState::Confirmed)
        .map(|(call, _)| call)
        .collect();

    if calls.is_empty() {
        memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopDoorClosed))
            .expect("Error sending movement state to memory");
        return false;
    }

    // Find the closest call in either direction
    let closest_call = calls.iter().min_by_key(|call| (call.floor as i32 - my_floor as i32).abs());

    match closest_call {
        Some(call) => {
            let target_direction = if call.floor > my_floor { Direction::Up } else { Direction::Down };

            if *current_dir != target_direction {
                println!("Brain: Switching direction to {:?}", target_direction);
                *current_dir = target_direction;
            }

            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::Moving(*current_dir)))
                .expect("Error sending movement state to memory");
            true
        }
        None => false,
    }
}
 */
/* I like this one better (bellow)
fn should_i_go(current_dir: &mut Direction, memory_request_tx: &Sender<mem::MemoryMessage>, my_state: &mem::State) -> bool {
    if matches!(my_state.move_state, elevint::MovementState::Obstructed) {
        return false;
    }

    let my_floor = my_state.last_floor;
    let mut has_calls_ahead = false;
    let mut has_any_calls = false;

    for (call, state) in &my_state.call_list {
        if *state == mem::CallState::Confirmed {
            has_any_calls = true;
            if (matches!(current_dir, elevint::Direction::Up) && call.floor > my_floor)
                || (matches!(current_dir, elevint::Direction::Down) && call.floor < my_floor)
            {
                has_calls_ahead = true;
                break;
            }
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

 */
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