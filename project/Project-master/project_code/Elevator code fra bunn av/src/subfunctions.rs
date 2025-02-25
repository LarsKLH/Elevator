
use std::default;
use std::hash::Hash;
use std::hash::Hasher;
use std::collections::HashSet;
use std::net::Ipv6Addr;
use std::thread::*;
use std::time::*;
use std::u8;
use std::sync::*;
use std::cmp::max;

use driver_rust::elevio;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel as cbc;
use driver_rust::elevio::elev;
use driver_rust::elevio::elev::Elevator;


use crate::memory as mem;




pub fn state_machine_check(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}

pub fn sanity_check(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::State>) -> () {

    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
                let old_memory = memory_recieve_rx.recv().unwrap();

                let recieved_state = rx.unwrap();
                let old_calls = old_memory.state_list.get(recieved_state.id).unwrap().call_list;
                let changes = recieved_state.call_list.difference(&old_calls);
                
            }
        }
    }
}

pub fn rx(rx_send: Sender<mem::State>) -> () {

}

pub fn tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}

enum MotorMessage {
    Up,
    Down,
    EmergencyStop,
    StopAndOpen
}

// Motor controller function. Takes controller messages and sends them to the elevator
// controller. Also updates the memory with the current direction of the elevator
pub fn motor_controller(memory_request_tx: Sender<MemoryMessage>, motor_controller_receive: Receiver<MotorMessage>, elevator: Elevator) -> () {
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