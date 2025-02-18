
use std::default;
use std::hash::Hash;
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

enum States {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
}

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

struct State {
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
    UpdateOwnCall(Call),
    UpdateOthersState(State)
    // TODO krangle om hvordan endre state med update
    // TODO gjøre requests av memory til immutable referanser og update til mutable referanser slik at compileren blir sur om vi ikke gj;r ting riktig
    
    // Mulig fix, gjøre update slik at den sender en init update som låser databasen til den blir skrevet til igjen
}

pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<MemoryMessage>) -> () {
    memory = Memory::new();

    loop {
        cbc::select! {
            recv(memory_request_rx) -> raw => {
                let request = raw.unwrap();
                match request {
                    MemoryMessage::Request => {
                        memory_recieve_tx.send(memory).unwrap();
                    }
                    MemoryMessage::Update_own_direction(dirn) => {

                        // Change the requested state in memory
                        
                        // Syntaks er definitivt feil, men dette viser ideen
                        memory.state_list(memory.my_id).direction = dirn;
                    }
                    MemoryMessage::Update_own_call(call) => {

                        // Change the requested state in memory
                        
                        // Syntaks er definitivt feil, men dette viser ideen
                        memory.state_list(memory.my_id).call_list(call) = call;
                    }
                    MemoryMessage::Update_others_state(state) => {

                        // Change the requested state in memory

                        // Syntaks er definitivt feil, men dette viser ideen
                        memory.state_list(state.id) = state;
                    }
                }
            }
        }
    }
}

pub fn state_machine_check(memory_tx: Sender<>, memory_rx: Receiver<>) -> () {

}

pub fn sanity_check() -> () {

}

pub fn rx() -> () {

}

pub fn tx() -> () {

}

pub fn motor_controller() -> () {

}

pub fn elevator_logic() -> () {

}

pub fn button_checker() -> () {

}