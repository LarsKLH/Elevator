
use std::default;
use std::hash::Hash;
use std::hash::Hasher;
use std::thread::*;
use std::time::*;
use std::collections::HashSet;
use std::u8;
use std::sync::*;
use std::cmp::max;

use driver_rust::elevio;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel as cbc;
use driver_rust::elevio::elev;
use driver_rust::elevio::elev::Elevator;

#[derive(Eq, PartialEq)]
enum States {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
}

#[derive(Eq, PartialEq)]
struct Call {
    direction: u8,
    floor: u8,
    call_state: States
}

impl Hash for Call {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.direction.hash(state);
        self.floor.hash(state);
    }
}

#[derive(Eq, PartialEq)]
pub struct State {
    id: Macaddr, // Jens fikser
    direction: u8,
    last_floor: u8,
    call_list: HashSet<Call>,
    cab_calls: HashSet<u8>
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct Memory {
    my_id: Macaddr, // Jens fikser
    state_list: HashSet<State>
}

pub enum MemoryMessage {
    Request,
    UpdateOwnDirection(u8),
    UpdateOwnFloor(u8),
    UpdateOwnCall(Call),
    UpdateOthersState(State)
    // TODO krangle om hvordan endre state med update
    // TODO gjøre requests av memory til immutable referanser og update til mutable referanser slik at compileren blir sur om vi ikke gj;r ting riktig
    
    // Mulig fix, gjøre update slik at den sender en init update som låser databasen til den blir skrevet til igjen
}

pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<MemoryMessage>) -> () {
    let memory = Memory::new();

    loop {
        cbc::select! {
            recv(memory_request_rx) -> raw => {
                let request = raw.unwrap();
                match request {
                    MemoryMessage::Request => {
                        memory_recieve_tx.send(memory).unwrap();
                    }
                    MemoryMessage::UpdateOwnDirection(dirn) => {

                        // Change own direction in memory
                        
                        memory.state_list(memory.my_id).direction = dirn;
                    }
                    MemoryMessage::UpdateOwnFloor(floor) => {

                        // Change own floor in memory
                        
                        memory.state_list(memory.my_id).last_floor = floor;
                    }

                    MemoryMessage::UpdateOwnCall(call) => {

                        // Update a single call in memory
                        
                        memory.state_list(memory.my_id).call_list.replace(call);
                    }
                    MemoryMessage::UpdateOthersState(state) => {

                        // Change the requested state in memory

                        memory.state_list.replace(state);
                    }
                }
            }
        }
    }
}

pub fn state_machine_check(memory_request_tx: Sender<MemoryMessage>, memory_recieve_rx: Receiver<Memory>) -> () {

}

pub fn sanity_check(memory_request_tx: Sender<MemoryMessage>, memory_recieve_rx: Receiver<Memory>, rx_get: Receiver<State>) -> () {

    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                memory_request_tx.send(MemoryMessage::Request).unwrap();
                let old_memory = memory_recieve_rx.recv().unwrap();

                let recieved_state = rx.unwrap();
                let old_calls = old_memory.state_list.get(recieved_state.id).unwrap().call_list;
                let changes = recieved_state.call_list.difference(&old_calls);
                
            }
        }
    }
}

pub fn rx(rx_send: Sender<State>) -> () {

}

pub fn tx(memory_request_tx: Sender<MemoryMessage>, memory_recieve_rx: Receiver<Memory>) -> () {

}

enum MotorMessage {
    Up,
    Down,
    EmergencyStop,
    StopAndOpen
}

pub fn motor_controller(memory_request_tx: Sender<MemoryMessage>, motor_controller_receive: Receiver<MotorMessage>, elevator: Elevator) -> () {
    // Create direction variable and send elevator down until it hits a floor
    let mut direction = elevio::elev::DIRN_DOWN;
    elevator.motor_direction(direction);

    // Update direction in memory
    memory_request_tx.send(MemoryMessage::UpdateOwnDirection(direction)).unwrap();

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
                        memory_request_tx.send(MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::Up => {
                        // Turn off elevator light before starting
                        elevator.door_light(false);
                        sleep(Duration::from_millis(500));

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_UP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::EmergencyStop => {
                        // Turn off elevator light just in case
                        elevator.door_light(false);

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::StopAndOpen => {
                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(MemoryMessage::UpdateOwnDirection(direction)).unwrap();

                        // Turn on light for now
                        elevator.door_light(true);
                    }
                }
            }
        }
    }
}

pub fn elevator_logic(memory_request_tx: Sender<MemoryMessage>, memory_recieve_rx: Receiver<Memory>) -> () {

}

pub fn button_checker() -> () {

}