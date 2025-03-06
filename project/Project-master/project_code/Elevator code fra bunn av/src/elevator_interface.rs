


use std::collections::HashMap;
use std::time::*;
use std::thread::*;



use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use driver_rust::elevio::elev::HALL_DOWN;
use serde::{Serialize, Deserialize};

use driver_rust::elevio::elev::DIRN_STOP;
use driver_rust::elevio::{self, elev::{self, Elevator}};
use crate::memory::CallState;
use crate::memory::State;
use crate::memory as mem;



#[derive(Eq, PartialEq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down
}

#[derive(Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum MovementState {
    Moving(Direction),
    StopDoorClosed,
    StopAndOpen,
    Obstructed // See spec, the oonly req on obstr. is that we do not close the door, we propebly need to ask about this
}

// TODO: add from and to for movement state and elevio::elev::DIRV

// Motor controller function. Takes controller messages and sends them to the elevator
// controller. Also updates the memory with the current direction of the elevator
pub fn elevator_controller(memory_request_tx: Sender<mem::MemoryMessage>, elevator_controller_receive: Receiver<State>, elevator: Elevator) -> () {
    
    
    // TODO: jens want to remove the next two lines
    
    // Create direction variable and send elevator down until it hits a floor
    elevator.motor_direction(elevio::elev::DIRN_DOWN);

    // Update direction in memory
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Moving(Direction::Down))).unwrap();

    // Infinite loop checking for elevator controller messages
    loop {
        cbc::select! {
            recv(elevator_controller_receive) -> state_to_mirror => {
                let received_state_to_mirror = state_to_mirror.unwrap();

                mirror_movement_state(received_state_to_mirror.move_state, &elevator);
                
                mirror_lights(received_state_to_mirror, &elevator);


                
            }
        }
    }
}



fn mirror_movement_state (new_move_state: MovementState, elevator: &Elevator) {
    match new_move_state {
        MovementState::Moving(dirn) => {
            match dirn {
                Direction::Down => {
                    // Turn off elevator light before starting
                    elevator.door_light(false);
                    sleep(Duration::from_millis(500));
                    

                    // Change direction and update memory
                    elevator.motor_direction(elevio::elev::DIRN_DOWN);
                }
                Direction::Up => {
                    // Turn off elevator light before starting
                    elevator.door_light(false);
                    sleep(Duration::from_millis(500));

                    // Change direction and update memory
                    elevator.motor_direction(elevio::elev::DIRN_UP);
                }
            }
        }

        MovementState::StopDoorClosed => {
            // Turn off elevator light just in case
            elevator.door_light(false);

            // Change direction and update memory
            elevator.motor_direction(elevio::elev::DIRN_STOP);
        }
        MovementState::StopAndOpen => {

            // Change direction and update memory
            elevator.motor_direction(elevio::elev::DIRN_STOP);

            // Turn on light for now
            elevator.door_light(true);
        }
        MovementState::Obstructed => {/* Do nothing, as per spec we must just make sure that the doors dont close */}
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

    // Set poll period for buttons and sensors
    let poll_period = Duration::from_millis(25);

    // Initialize button sensors
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>(); // Initialize call buttons
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_button_tx, poll_period));
    }

     // Initialize floor sensor
     let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period));
    }
    
    // Initialize stop button
    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_button_tx, poll_period));
    }
    
    // Initialize obstruction switch
    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstruction_tx, poll_period));
    } 

    loop {


        // TODO we need to ckeck if th

        cbc::select! {
            recv(call_button_rx) -> call_button_notif => {
                let button_pressed = call_button_notif.unwrap();

                //todo!("have to update the cyclic counter for this floor");
                // juct check if the current state is nothing then chnage to new, if else do nothing

                memory_request_tx.send(mem::MemoryMessage::Request);
                let current_memory = memory_recieve_rx.recv().unwrap();

                let current_calls = current_memory.state_list.get(&current_memory.my_id).unwrap().call_list.clone();

                let equivilent_button_in_memory = mem::Call::from(button_pressed);

                let pressed_button_current_state = current_calls.get(&equivilent_button_in_memory).unwrap();

                if pressed_button_current_state == &CallState::Nothing {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(equivilent_button_in_memory, CallState::New));
                }

            }

            recv(floor_sensor_rx) -> floor_sensor_notif => {
                let floor_sensed = floor_sensor_notif.unwrap();

                // might be a bad thing too do
                memory_request_tx.send(mem::MemoryMessage::UpdateOwnFloor(floor_sensed)).unwrap();
                //this is a hardware thing, if we cant trust it we cant trust anything
                
                
                
                
                // NEED to send to brain as this circumwent memory as of now

                // this might be a bad idea, as i think this open for a race condition
                // if the memory is not updated before the brain tries to read from the memory
                floor_sensor_to_brain_tx.send(floor_sensed).unwrap(); 
                
            }

            recv(stop_button_rx) -> stop_button_notif => {
                let stop_button_pressed = stop_button_notif.unwrap();

                // Do we want to do anything here?
            }

            recv(obstruction_rx) -> obstruction_notif => {
                let obstruction_sensed = obstruction_notif.unwrap();

                // todo!("we need to figure out how to do here");
                // add new move state obstructed that wil force us to do nothing, but check if obstr gets removed

                memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Obstructed)).unwrap();
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


