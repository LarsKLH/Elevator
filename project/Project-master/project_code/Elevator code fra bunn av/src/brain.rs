use std::time::Duration;
use std::thread;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;
use crate::memory::{self as mem, Call, CallState, CallType};
use crate::elevator_interface::{self as elevint, Direction};

// The main elevator logic. Determines where to go next and sends commands to the elevator interface
pub fn elevator_logic(
    memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, brain_stop_direct_link: Sender<mem::State>) -> () {

    let mut prev_direction = elevint::Direction::Down;
    loop {
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
                            my_state.last_floor = a.expect("Error reading from floor sensor");
                            my_state.move_state = elevint::MovementState::StopAndOpen;
                            brain_stop_direct_link.send(my_state).expect("Error sending stop and open to brain");
                            memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(elevint::MovementState::StopAndOpen)).expect("Error sending stop and open to memory"); 
                        }
                        else {}
                    }
                    default(Duration::from_millis(1000)) => {
                    }
                }
            } 
            elevint::MovementState::StopDoorClosed => {
                let going = should_i_go(&mut prev_direction, &memory_request_tx ,&my_state , &memory);
                if going {}
            }
            elevint::MovementState::StopAndOpen => {
                thread::sleep(Duration::from_secs(3));
                clear_call(&mut my_state,  &memory_request_tx, prev_direction);    
                let going = should_i_go(&mut prev_direction, &memory_request_tx ,&my_state, &memory);
                if going {}
            }
            elevint::MovementState::Obstructed => {
                let going = should_i_go(&mut prev_direction, &memory_request_tx ,&my_state , &memory);
                if going {}  
            }
            //_ => {}
        }
    }
}

// Check if the elevator should stop or not | Todo: Maybe turn if into a match statement
fn should_i_stop(
    floor: u8, my_state: &mem::State) -> bool {
    let my_direction = match my_state.move_state {
        elevint::MovementState::Moving(dir) => dir,
        _ => return false,
    };

    let mut has_call_ahead = false;

    for (call, state) in &my_state.call_list {
        if *state == mem::CallState::Confirmed {
            if call.floor == floor {
                match call.call_type {
                    CallType::Cab => {
                        println!("Brain: Stopping at cab call at floor {}", floor);
                        return true;
                    }
                    CallType::Hall(dir) => {
                        if dir == my_direction {
                            println!("Brain: Stopping at hall call {:?} at floor {}", my_direction, floor);
                            return true;
                        }
                    }
                    //_ => {}
                }
            }
            if (my_direction == elevint::Direction::Up && call.floor > floor)
                || (my_direction == elevint::Direction::Down && call.floor < floor)
            {
                has_call_ahead = true;
            }
        }
    }

    !has_call_ahead
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
fn should_i_go(
    current_dir: &mut Direction, memory_request_tx: &Sender<mem::MemoryMessage>,
    my_state: &mem::State, memory: &mem::Memory) -> bool {
    if my_state.move_state == elevint::MovementState::Obstructed {
        return false;
    }

    
    let my_floor = my_state.last_floor;
    let mut has_calls_ahead = false;
    let mut has_any_calls = false;
    
    if my_state.call_list.get(&Call { call_type: CallType::Cab, floor: my_state.last_floor}) == Some(&CallState::PendingRemoval)
        || my_state.call_list.get(&Call { call_type: CallType::Hall(*current_dir), floor: my_state.last_floor}) == Some(&CallState::PendingRemoval) {
            // There is a call that still needs top be removed so we cant move further untill it is
            
            println!("Brain: Cannot leave floor {} in direction {:?} as there are calls that are pending removal here", my_floor, current_dir);
            return false;
        }



    // Collect confirmed calls where this elevator is the best responder
    let mut best_calls: Vec<&mem::Call> = Vec::new();
    for (call, state) in &my_state.call_list {
        if *state == mem::CallState::Confirmed {
            if call.call_type != mem::CallType::Cab {
                if !am_i_best_elevator_to_respond(*call, memory.clone(), *current_dir) {
                    continue;
                }
            }
            best_calls.push(call);
        }
    }

    if best_calls.is_empty() {
        memory_request_tx
            .send(mem::MemoryMessage::UpdateOwnMovementState(
                elevint::MovementState::StopDoorClosed,
            ))
            .unwrap();
        return false;
    }

    for call in best_calls {
        has_any_calls = true;
        if (matches!(current_dir, elevint::Direction::Up) && call.floor > my_floor)
            || (matches!(current_dir, elevint::Direction::Down) && call.floor < my_floor)
        {
            has_calls_ahead = true;
            break;
        }
    }

    match (has_any_calls, has_calls_ahead) {
        (false, _) => {
            memory_request_tx
                .send(mem::MemoryMessage::UpdateOwnMovementState(
                    elevint::MovementState::StopDoorClosed,
                ))
                .unwrap();
            false
        }
        (true, true) => {
            memory_request_tx
                .send(mem::MemoryMessage::UpdateOwnMovementState(
                    elevint::MovementState::Moving(*current_dir),
                ))
                .unwrap();
            true
        }
        (true, false) => {
            memory_request_tx
                .send(mem::MemoryMessage::UpdateOwnMovementState(
                    elevint::MovementState::StopAndOpen,
                ))
                .unwrap();
            *current_dir = match *current_dir {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
            };
            true
        }
    }
}

/* Checks if the current elevator is the best one to respond
    to a call based on its state and the call's properties.
    NB! If you want want to differenciate between stopped 
    elevators w/ different directions, uncomment in func,
    current_dir is then used as well*/
fn am_i_best_elevator_to_respond(
    call: mem::Call, memory: mem::Memory, current_dir: Direction) -> bool {
    let my_id = memory.my_id;
    let my_state = memory.state_list.get(&my_id).unwrap();
    let my_floor = my_state.last_floor;
    let my_calls = my_state.call_list.len();
    
    if matches!(my_state.move_state, elevint::MovementState::Obstructed) {
        return false;
    }

    let call_floor = call.floor;
    
    // --- Direction/Movement Scoring ---
    let direction_score = match my_state.move_state {
        elevint::MovementState::Moving(dir) if (dir == Direction::Up && call_floor >= my_floor)
                                           || (dir == Direction::Down && call_floor <= my_floor) => 0,
        // Stopped but facing the right direction: medium priority (3)
        /*elevint::MovementState::StopDoorClosed | elevint::MovementState::StopAndOpen 
        if (*current_dir == Direction::Up && call_floor > my_floor)
        || (*current_dir == Direction::Down && call_floor < my_floor) => 3,*/
        elevint::MovementState::StopDoorClosed | elevint::MovementState::StopAndOpen => 5,
        _ => 10,
    };

    // --- Distance + Load ---
    let my_score = (call_floor as i32 - my_floor as i32).abs() as u32  // Distance penalty
        + direction_score                                             // Movement penalty
        + (my_calls as u32 * 2);                                     // Load penalty

    for (elev_id, elev_state) in &memory.state_list {
        if *elev_id == my_id || matches!(elev_state.move_state, elevint::MovementState::Obstructed) {
            continue;
        }

        let other_floor = elev_state.last_floor;
        let other_calls = elev_state.call_list.len();
        let other_direction_score = match elev_state.move_state {
            elevint::MovementState::Moving(dir) if (dir == Direction::Up && call_floor >= other_floor)
                                               || (dir == Direction::Down && call_floor <= other_floor) => 0,
            // Stopped but facing the right direction: medium priority (3)
            /*elevint::MovementState::StopDoorClosed | elevint::MovementState::StopAndOpen 
            if (*current_dir == Direction::Up && call_floor > my_floor)
            || (*current_dir == Direction::Down && call_floor < my_floor) => 3,*/
            elevint::MovementState::StopDoorClosed | elevint::MovementState::StopAndOpen => 5,
            _ => 10,
        };

        let other_score = (call_floor as i32 - other_floor as i32).abs() as u32
            + other_direction_score
            + (other_calls as u32 * 2);
        println!("Brain: My score: {}, Other score: {}", my_score, other_score);
        thread::sleep(Duration::from_millis(500));

        if other_score < my_score {
            return false;
        }
    }
    true
}
