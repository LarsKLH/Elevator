


use std::time::*;
use std::thread::*;



use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use serde::{Serialize, Deserialize};

use driver_rust::elevio::elev::DIRN_STOP;
use driver_rust::elevio::{self, elev::{self, Elevator}};
use crate::memory::State;
use crate::memory as mem;



#[derive(Eq, PartialEq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down
}

#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum MovementState {
    Dir(Direction),
    StopDoorClosed,
    StopAndOpen
}

// TODO: add from and to for movement state and elevio::elev::DIRV

// Motor controller function. Takes controller messages and sends them to the elevator
// controller. Also updates the memory with the current direction of the elevator
pub fn elevator_controller(memory_request_tx: Sender<mem::MemoryMessage>, elevator_controller_receive: Receiver<State>, elevator: Elevator) -> () {
    
    
    // TODO: jens want to remove the next two lines
    
    // Create direction variable and send elevator down until it hits a floor
    let mut direction = elevio::elev::DIRN_DOWN;
    elevator.motor_direction(direction);

    // Update direction in memory
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Dir(Direction::Down))).unwrap();

    // Infinite loop checking for elevator controller messages
    loop {
        cbc::select! {
            recv(elevator_controller_receive) -> state_to_mirror => {
                let received_state_to_mirror = state_to_mirror.unwrap();
                match received_state_to_mirror.move_state {
                    MovementState::Dir(dirn) => {
                        match dirn {
                            Direction::Down => {
                                // Turn off elevator light before starting
                                elevator.door_light(false);
                                sleep(Duration::from_millis(500));
                                

                                // Change direction and update memory
                                direction = elevio::elev::DIRN_DOWN;
                                elevator.motor_direction(direction);
                                memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Dir(Direction::Down))).unwrap();
                            }
                            Direction::Up => {
                                // Turn off elevator light before starting
                                elevator.door_light(false);
                                sleep(Duration::from_millis(500));

                                // Change direction and update memory
                                direction = elevio::elev::DIRN_UP;
                                elevator.motor_direction(direction);
                                memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::Dir(Direction::Down))).unwrap();
                            }
                        }
                    }

                    MovementState::StopDoorClosed => {
                        // Turn off elevator light just in case
                        elevator.door_light(false);

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::StopDoorClosed)).unwrap();
                    }
                    MovementState::StopAndOpen => {
                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnMovementState(MovementState::StopAndOpen)).unwrap();

                        // Turn on light for now
                        elevator.door_light(true);
                    }
                }
            }
        }
    }
}



// Probably not needed
pub fn button_checker() -> () {

}