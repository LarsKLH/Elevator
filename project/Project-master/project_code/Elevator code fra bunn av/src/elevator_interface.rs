


use core::num;
use std::time::*;
use std::thread::*;



use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use serde::{Serialize, Deserialize};

use driver_rust::elevio::{self, elev::{self, Elevator}};
use crate::memory::CallState;
use crate::memory::State;
use crate::memory as mem;


// Set poll period for buttons and sensors
const POLLING_PERIOD: Duration = Duration::from_millis(50);



#[derive(Eq, PartialEq, Hash, Clone, Copy, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub enum Direction {
    Up,
    Down
}

#[derive(Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum MovementState {
    Moving(Direction),
    StopDoorClosed,
    StopAndOpen,
    Obstructed // See spec, the oonly req on obstr. is that we do not close the door, we propebly need to ask about this
}

// TODO: add from and to for movement state and elevio::elev::DIRV

// Motor controller function. Takes controller messages and sends them to the elevator
// controller. Also updates the memory with the current direction of the elevator
pub fn elevator_outputs(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, brain_stop_direct_link: Receiver<mem::State>, elevator: Elevator, num_floors: u8) -> () {
    
    
    // TODO: jens want to remove the next two lines
    
    // Create direction variable and send elevator down until it hits a floor
    elevator.motor_direction(elevio::elev::DIRN_DOWN);

    // Update direction in memory
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Moving(Direction::Down))).unwrap();

    memory_request_tx.send(mem::MemoryMessage::Request).expect("ElevInt: Could not request memory");

    let original_memory = memory_recieve_rx.recv().expect("ElevInt: Could not recieve memory");

    let mut prev_state = original_memory.state_list.get(&original_memory.my_id).expect("ElevInt: could not extract my memory from memory").clone();

    println!("ElevInt: Done with Initialization of Outputs");

    // Infinite loop checking for elevator controller messages
    loop {
        cbc::select! {
            recv(brain_stop_direct_link) -> received_state => {
                let received_state_to_mirror = received_state.unwrap();

                mirror_movement_state(received_state_to_mirror.move_state, &elevator, num_floors, received_state_to_mirror.last_floor);
                
                mirror_lights(received_state_to_mirror, &elevator);
                
                sleep(Duration::from_millis(500));

            }
            default(Duration::from_millis(50))  => {
                let current_memory = mem::Memory::get(memory_request_tx.clone(), memory_recieve_rx.clone());

                let current_state = current_memory.state_list.get(&current_memory.my_id).unwrap().clone();

                // Dont need to send commands to the elevator if there is nothing to change
                if current_state != prev_state {
                    mirror_movement_state(current_state.move_state, &elevator, num_floors, current_state.last_floor);
                    
                    mirror_lights(current_state.clone(), &elevator);

                    prev_state = current_state;
                }
                
                
            }
        }
    }
}



fn mirror_movement_state (new_move_state: MovementState, elevator: &Elevator, num_floors: u8, last_floor: u8) {
    const GROUND_FLOOR: u8 = 0;

    match new_move_state {
        MovementState::Moving(dirn) => {
            match dirn {
                Direction::Down => {
                    // Turn off elevator light before starting
                    elevator.door_light(false);
                    
                    if last_floor == GROUND_FLOOR {
                        // Do nothing, this is wrong
                    }
                    else {
                        // Follow direction
                        elevator.motor_direction(elevio::elev::DIRN_DOWN);
                    }
                    
                }
                Direction::Up => {
                    // Turn off elevator light before starting
                    elevator.door_light(false);

                    if last_floor == num_floors - 1 {
                        // Do nothing, this is wrong
                    }
                    else {
                        // Follow direction
                        elevator.motor_direction(elevio::elev::DIRN_UP);
                    }
                }
            }
        }

        MovementState::StopDoorClosed => {
            // Turn off elevator light just in case
            elevator.door_light(false);

            // Change direction
            elevator.motor_direction(elevio::elev::DIRN_STOP);
        }
        MovementState::StopAndOpen => {

            // Change direction
            elevator.motor_direction(elevio::elev::DIRN_STOP);

            // Turn on light for now
            elevator.door_light(true);
        }
        MovementState::Obstructed => {elevator.motor_direction(elevio::elev::DIRN_STOP);/* Allow NO movement or open doors*/}
    }
}

fn mirror_lights(state_to_mirror: State, elevator: &Elevator) {

    // update call button lighs
    
    for (spesific_call, call_state) in state_to_mirror.call_list {
        elevator.call_button_light(spesific_call.floor, spesific_call.call_type.into_elevio_call_type(), call_state.into_elevio_light_state());
    }

    elevator.floor_indicator(state_to_mirror.last_floor);

    // might want to also add the stop light 

    

}




pub fn elevator_inputs(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_to_brain_tx: Sender<u8>, elevator: Elevator) -> () {

    // We need to remember what we ware at so we dont prematurly change state when handling an order
    let mut movement_state_before_prev_obstruction = MovementState::StopDoorClosed; 


    // Initialize button sensors
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>(); // Initialize call buttons
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_button_tx, POLLING_PERIOD));
    }

     // Initialize floor sensor
     let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_sensor_tx, POLLING_PERIOD));
    }
    
    // Initialize stop button
    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_button_tx, POLLING_PERIOD));
    }
    
    // Initialize obstruction switch
    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstruction_tx, POLLING_PERIOD));
    } 

    println!("ElevInt: Done with Initialization of Inputs");

    loop {


        // TODO we need to ckeck if th

        cbc::select! {
            recv(call_button_rx) -> call_button_notif => {
                let button_pressed = call_button_notif.expect("Failed to unpack what putton was pressed");

                //todo! done I think - Jens ("have to update the cyclic counter for this floor");
                // juct check if the current state is nothing then chnage to new, if else do nothing

                memory_request_tx.send(mem::MemoryMessage::Request).expect("Failed to send request to memory");
                let current_memory = memory_recieve_rx.recv().expect("Failed to recieve memory");

                let current_calls = current_memory.state_list.get(&current_memory.my_id).expect("Failed to fetch mmy memory by id").call_list.clone();

                let equivilent_button_in_memory = mem::Call::from(button_pressed);
                
                let pressed_button_current_state = current_calls.get(&equivilent_button_in_memory).expect("Failed to fetch current call state of pressed button");
                
                if pressed_button_current_state == &CallState::Nothing {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(equivilent_button_in_memory, CallState::New)).expect("Failed to send new call to memory");
                }

            }

            recv(floor_sensor_rx) -> floor_sensor_notif => {
                let floor_sensed = floor_sensor_notif.expect("Failed to recieve floor sensor notification");

                // might be a bad thing too do
                memory_request_tx.send(mem::MemoryMessage::UpdateOwnFloor(floor_sensed)).expect("Failed to send floor to memory");
                //this is a hardware thing, if we cant trust it we cant trust anything
                
                
                
                
                // NEED to send to brain as this circumwent memory as of now

                // this might be a bad idea, as i think this open for a race condition
                // if the memory is not updated before the brain tries to read from the memory
                floor_sensor_to_brain_tx.send(floor_sensed).expect("Failed to send floor to brain"); 
                
            }

            recv(stop_button_rx) -> stop_button_notif => {
                let stop_button_pressed = stop_button_notif.unwrap();

                // Do we want to do anything here?
                // Dont think so
            }

            recv(obstruction_rx) -> obstruction_notif => {
                let obstruction_sensed = obstruction_notif.unwrap();

                memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
                let current_memory = memory_recieve_rx.recv().unwrap();

                let current_movement_state = current_memory.state_list.get(&current_memory.my_id).unwrap().move_state;

                // state obstructed that wil force us to do nothing, but check we need to check if obstructed gets removed
                if obstruction_sensed {
                    movement_state_before_prev_obstruction = current_movement_state;
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Obstructed)).unwrap();
                }
                else if current_movement_state == MovementState::Obstructed {
                    // obstruction is over, return to the preceedign movement state
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(movement_state_before_prev_obstruction)).unwrap();
                }
            }
        }

    }




}


impl From<elevio::poll::CallButton> for mem::Call {
    fn from(button_polled: elevio::poll::CallButton) -> mem::Call {
        let call_type_of_button = match button_polled.call {
            0 => mem::CallType::Hall(Direction::Up),
            1 => mem::CallType::Hall(Direction::Down),
            2 => mem::CallType::Cab,
            _ => panic!("recieved an u8 from the elevator button poller that is not either 0, 1, or 2, terminating immediatly!")
        };

        mem::Call {
            call_type: call_type_of_button,
            floor: button_polled.floor
        }
    }
}

impl CallState {
    fn into_elevio_light_state(&self) -> bool {
        match self {
            Self::Nothing | Self::New => false,
            Self::Confirmed | Self::PendingRemoval => true,
        }
    }
}

impl mem::CallType {
    fn into_elevio_call_type(&self) -> u8 {
        match self {
            Self::Cab => elevio::elev::CAB,
            Self::Hall(Direction::Up) => elevio::elev::HALL_UP,
            Self::Hall(Direction::Down) => elevio::elev::HALL_DOWN,
        }
    }
}


