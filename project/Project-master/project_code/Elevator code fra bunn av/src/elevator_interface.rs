


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
    StopAndOpen
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
    }
}

fn mirror_lights(state_to_mirror: State, elevator: &Elevator) {
    
    // update call button lighs
    for (cab_call_floor, cab_call_state) in state_to_mirror.cab_calls {
        match cab_call_state {
            CallState::Nothing | CallState::New => elevator.call_button_light(cab_call_floor, elevio::elev::CAB, false),
            CallState::Confirmed | CallState::PendingRemoval => elevator.call_button_light(cab_call_floor, elevio::elev::CAB, true),
        }
    }

    for (spesific_call, call_state) in state_to_mirror.call_list {
        // Talk to Seb about doing this in a sensible way
        match spesific_call.direction {
            Direction::Up  => {
                match call_state {
                    CallState::Nothing | CallState::New => elevator.call_button_light(spesific_call.floor, elevio::elev::HALL_UP, false),
                    CallState::Confirmed | CallState::PendingRemoval => elevator.call_button_light(spesific_call.floor, elevio::elev::HALL_UP, true),
            }}
            Direction::Down => {
                match call_state {
                    CallState::Nothing | CallState::New => elevator.call_button_light(spesific_call.floor, elevio::elev::HALL_DOWN, false),
                    CallState::Confirmed | CallState::PendingRemoval => elevator.call_button_light(spesific_call.floor, elevio::elev::HALL_DOWN, true),
                }
            }
        }
    }

    elevator.floor_indicator(state_to_mirror.last_floor);

    // might want to also add the stop light 

    

}




pub fn elevator_inputs(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, elevator: Elevator) -> () {

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

                todo!("have to update the cyclic counter for this floor")
            }

            recv(floor_sensor_rx) -> floor_sensor_notif => {
                let floor_sensed = floor_sensor_notif.unwrap();

                // might be a bad thing too do
                memory_request_tx.send(mem::MemoryMessage::UpdateOwnFloor(floor_sensed)).unwrap();
            }

            recv(stop_button_rx) -> stop_button_notif => {
                let stop_button_pressed = stop_button_notif.unwrap();

                // Do we want to do anything here?
            }

            recv(obstruction_rx) -> obstruction_notif => {
                let obstruction_sensed = obstruction_notif.unwrap();

                todo!("we need to figure out how to do here")
            }
        }

    }




}






