use core::hash;
use std::{  collections::{HashMap, HashSet}, hash::{Hash,Hasher}, net::Ipv4Addr, ops::Deref, time::Instant};

use driver_rust::elevio;
use postcard;
use serde::{Serialize, Deserialize};

use std::thread;
use std::time;

use itertools::Itertools;

use crossbeam_channel::{Receiver, Sender};

use crossbeam_channel as cbc;

use crate::{elevator_interface::MovementState, memory as mem};
use crate::elevator_interface as elevint;

const PRINT_STATUS_INTERVAL: time::Duration = time::Duration::from_millis(1000);


#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub my_id: Ipv4Addr,
    pub state_list: HashMap<Ipv4Addr,State>
}


#[derive(Eq, PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct State {
    pub id: Ipv4Addr,
    pub timed_out: bool,
    pub move_state: elevint::MovementState, // Jens: alle u8 i denne burde endres til typer tror jeg
    pub last_floor: u8,
    pub call_list: HashMap<Call, CallState>,
    pub is_stalled: bool,
}

#[derive(Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub enum CallState {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub struct Call{
    pub call_type: CallType,
    pub floor: u8
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub enum CallType {
    Cab,
    Hall(elevint::Direction) 
}




pub enum MemoryMessage {
    Request,
    UpdateOwnMovementState(MovementState),
    UpdateOwnFloor(u8),
    UpdateOwnCall(Call, CallState),
    UpdateOthersState(State),
    DeclareDead(Ipv4Addr),
    IsStalled(Ipv4Addr, bool),  
    // TODO krangle om hvordan endre state med update
    // TODO gjøre requests av memory til immutable referanser og update til mutable referanser slik at compileren blir sur om vi ikke gj;r ting riktig
    
    // Mulig fix, gjøre update slik at den sender en init update som låser databasen til den blir skrevet til igjen
}

impl Memory {
    pub fn new (ip: Ipv4Addr, n: u8) -> Self {
        Self { my_id: ip,
            state_list: HashMap::from([(ip, State::new(ip, n))]) 
        }
    }
    pub fn get (memory_request_channel: Sender<MemoryMessage>, memory_recieve_channel: Receiver<Memory>) -> Self {
        memory_request_channel.send(MemoryMessage::Request).expect("Failed to send request to memory thread");
        let received_memory = memory_recieve_channel.recv().expect("Failed to receive memory from memory thread");
        Self { my_id: received_memory.my_id,
            state_list: received_memory.state_list
        }
    }

    pub fn am_i_closest(&self, my_id: Ipv4Addr, call_floor: u8) -> bool {
        self.state_list
            .iter()
            .filter(|(_, state)| !state.timed_out)
            .min_by_key(|(_, state)| (state.last_floor as i8 - call_floor as i8).abs())
            .map(|(id, _)| *id == my_id)
            .unwrap_or(false)
    }
}

impl State {
    pub fn new (id_of_new: Ipv4Addr, n: u8) -> Self {
        let mut new_me = Self {  id: id_of_new,
                timed_out: false,
                move_state: elevint::MovementState::StopDoorClosed,
                last_floor: 0,
                call_list: HashMap::new(), // need to intitialize with the required number of floors that requires we pass the number of floors 
                is_stalled: false
            };
        for floor_to_add in 0..n {
            new_me.call_list.insert(Call { call_type: CallType::Cab, floor: floor_to_add }, CallState::Nothing);
            new_me.call_list.insert(Call { call_type: CallType::Hall(elevint::Direction::Up), floor: floor_to_add }, CallState::Nothing);
            new_me.call_list.insert(Call { call_type: CallType::Hall(elevint::Direction::Down), floor: floor_to_add}, CallState::Nothing);
        };
        new_me
    }
}

pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<MemoryMessage>, ipv4: Ipv4Addr, number_of_floors: u8) -> () {
    let mut memory = Memory::new(ipv4, number_of_floors);
    
    loop {
        cbc::select! {
            recv(memory_request_rx) -> raw => {
                let request = raw.unwrap();
                match request {
                    MemoryMessage::Request => {
                        let memory_copy = memory.clone();
                        memory_recieve_tx.send(memory_copy).unwrap();
                    }
                    MemoryMessage::UpdateOwnMovementState(new_move_state) => {
                        
                        // Change own direction in memory
                        
                        memory.state_list.get_mut(&memory.my_id).unwrap().move_state = new_move_state;
                    }
                    MemoryMessage::UpdateOwnFloor(floor) => {

                        // Change own floor in memory
                        memory.state_list.get_mut(&memory.my_id).unwrap().last_floor = floor;
                    }
                    
                    MemoryMessage::UpdateOwnCall(call, call_state) => {
                        // This works becouase the call is a cyclic counter, so it can only advance around

                        println!("Updating call: {:?} {:?}", call, call_state);
                        // Update a single call in memory
                        memory.state_list.get_mut(&memory.my_id).unwrap().call_list.insert(call, call_state); // todo add aceptence test, sanity check?
                    }
                    MemoryMessage::UpdateOthersState(state) => {
                        
                        // Change the requested state in memory, should never be called on our own state or we are fucked
                        
                        memory.state_list.insert(state.id, state);
                    }
                    MemoryMessage::DeclareDead(id) => {
                        
                        // Declare the requested elevator dead
                        
                        memory.state_list.get_mut(&id).unwrap().timed_out = true;
                    }
                    MemoryMessage::IsStalled(id, stalled) => {
                        
                        // Update the heartbeat of the requested elevator
                        
                        memory.state_list.get_mut(&id).unwrap().is_stalled = stalled;
                    }
                }
            }
        }
    }
}

// Jens: I really dont like this one

pub fn printout(memory_request_channel: Sender<MemoryMessage>, memory_recieve_channel: Receiver<Memory>) -> () {
    loop {
        let memory = Memory::get(memory_request_channel.clone(), memory_recieve_channel.clone());
        println!("-----------------------------------------------------------------------------------------------");
        println!("my_id: {}", memory.my_id);

        for state in memory.state_list.values() {
            println!("Elevator: {}", state.id);
            println!("Timed out: {}", state.timed_out);
            println!("Is stalled: {}", state.is_stalled);
            println!("Movement state: {:?}", state.move_state);
            println!("Last floor: {}", state.last_floor);
            for (call, call_state) in state.call_list.iter().sorted() {
                println!("Call: {:?} {:?} {:?}", call.call_type, call.floor, call_state);
            }
            thread::sleep(PRINT_STATUS_INTERVAL);
        }
    }
}