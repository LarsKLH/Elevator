


use std::time::*;
use std::thread::*;



use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;



use driver_rust::elevio::{self, elev::{self, Elevator}};
use crate::memory as mem;






enum MotorMessage {
    Up,
    Down,
    EmergencyStop,
    StopAndOpen
}

// Motor controller function. Takes controller messages and sends them to the elevator
// controller. Also updates the memory with the current direction of the elevator
pub fn motor_controller(memory_request_tx: Sender<mem::MemoryMessage>, motor_controller_receive: Receiver<MotorMessage>, elevator: Elevator) -> () {
    // Create direction variable and send elevator down until it hits a floor
    let mut direction = elevio::elev::DIRN_DOWN;
    elevator.motor_direction(direction);

    // Update direction in memory
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();

    // Infinite loop checking for motor controller messages
    loop {
        cbc::select! {
            recv(motor_controller_receive) -> order => {
                let received_order = order.unwrap();
                match received_order {
                    MotorMessage::Down => {
                        // Turn off elevator light before starting
                        elevator.door_light(false);
                        sleep(Duration::from_millis(500));

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_DOWN;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::Up => {
                        // Turn off elevator light before starting
                        elevator.door_light(false);
                        sleep(Duration::from_millis(500));

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_UP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::EmergencyStop => {
                        // Turn off elevator light just in case
                        elevator.door_light(false);

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::StopAndOpen => {
                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();

                        // Turn on light for now
                        elevator.door_light(true);
                    }
                }
            }
        }
    }
}



pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}

pub fn button_checker() -> () {

}