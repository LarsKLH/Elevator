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

// Clear the call from the memory
fn clear_call(my_state: mem::State,  memory_request_tx: Sender<mem::MemoryMessage>, prev_dir: Direction) -> () {
    use std::collections::HashMap;

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

// Check if the elevator should go or not
fn should_i_go(current_dir: &mut Direction, memory_request_tx: Sender<mem::MemoryMessage>, my_state: mem::State) -> bool {
    //println!("Brain: Checking if I should go w/ current direction {:?} and movement state {:?}", current_dir, my_state.move_state);
    match my_state.move_state {
        elevint::MovementState::Obstructed => {return false;}
        _ => {

            let calls: Vec<_> = my_state.call_list.clone().into_iter()
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

// Check if the elevator is the best elevator to respond to a call
fn am_i_best_elevator_to_respond(call: mem::Call, memory: mem::Memory, current_dir: Direction) -> bool {
    let my_id = memory.my_id;
    let my_floor = memory.state_list.get(&my_id).unwrap().last_floor;
    let current_dir = memory.state_list.get(&my_id).unwrap().move_state;
    let call_floor = call.floor;
    if (current_dir == elevint::MovementState::Moving(Direction::Up) && call_floor < my_floor)
        || (current_dir == elevint::MovementState::Moving(Direction::Down) && call_floor > my_floor){
        return false;
    }

    return memory.am_i_closest(my_id, call_floor);
}

/*fn restart(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {
    // TODO
    println!("Restarting elevator");
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();
    let my_state = memory.state_list.get(&memory.my_id).unwrap();
}*/