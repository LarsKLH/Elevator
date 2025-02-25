use std::net::Ipv6Addr;

use std::hash::{Hash,Hasher};
use std::collections::HashSet;


use crossbeam_channel::{Receiver, Sender};

use crossbeam_channel as cbc;

use crate::memory as mem;



pub struct Memory {
    pub my_id: Ipv6Addr, // Jens fikser
    pub state_list: HashSet<State>
}


#[derive(Eq, PartialEq, Clone)]
pub struct State {
    pub id: Ipv6Addr, // Jens fiksers
    pub direction: u8,
    pub last_floor: u8,
    pub call_list: HashSet<Call>,
    pub cab_calls: HashSet<u8>
}



#[derive(Eq, PartialEq, Clone, Copy)]
struct Call {
    pub direction: u8,
    pub floor: u8,
    pub call_state: CallState
}

#[derive(Eq, PartialEq, Clone, Copy)]
enum CallState {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
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


impl Hash for Call {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.direction.hash(state);
        self.floor.hash(state);
    }
}

impl From<Ipv6Addr> for Memory {
    fn from (ip: Ipv6Addr) -> Self {
        !todo!()
    }
    
}

impl Memory {
    pub fn get_state_from_id(&self, id: Ipv6Addr) -> State {
        self.state_list.get(&State::new(id)).unwrap().clone()
    }
}

impl Hash for State { // todo 
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl State {
    fn new (id: Ipv6Addr) -> Self {
        !todo!()
    }
}




pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<MemoryMessage>, ipv6: Ipv6Addr) -> () {
    let memory = Memory::from(ipv6);
    
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
                        
                        memory.state_list(memory.my_id).call_list.replace(call); // todo add aceptence test
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



pub fn state_machine_check(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}

