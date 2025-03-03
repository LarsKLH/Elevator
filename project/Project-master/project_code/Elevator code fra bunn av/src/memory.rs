use core::hash;
use std::{  net::Ipv4Addr,
            hash::{Hash,Hasher},
            collections::{HashMap, HashSet},
            ops::Deref};

use postcard;
use serde::{Serialize, Deserialize};



use crossbeam_channel::{Receiver, Sender};

use crossbeam_channel as cbc;

use crate::memory as mem;


#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub my_id: Ipv4Addr,
    pub state_list: HashMap<Ipv4Addr,State>
}


#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: Ipv4Addr, // Jens fiksers
    pub direction: u8, // Jens: alle u8 i denne burde endres til typer tror jeg
    pub last_floor: u8,
    pub call_list: HashMap<Call, CallState>,
    pub cab_calls: HashMap<u8, CallState>
}



#[derive(Eq, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct Call{
    pub direction: u8,
    pub floor: u8
}

#[derive(Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum CallState {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
}



pub enum MemoryMessage {
    Request,
    UpdateOwnDirection(u8),
    UpdateOwnFloor(u8),
    UpdateOwnCall(Call, CallState),
    UpdateOthersState(State)
    // TODO krangle om hvordan endre state med update
    // TODO gjøre requests av memory til immutable referanser og update til mutable referanser slik at compileren blir sur om vi ikke gj;r ting riktig
    
    // Mulig fix, gjøre update slik at den sender en init update som låser databasen til den blir skrevet til igjen
}

impl From<Ipv4Addr> for Memory {
    fn from (ip: Ipv4Addr) -> Self {
        !todo!()
    }
    
}

impl Memory {

}

impl Hash for State { // todo 
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl State {
    fn new (id: Ipv4Addr) -> Self {
        !todo!()
    }
}




pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<MemoryMessage>, ipv4: Ipv4Addr) -> () {
    let mut memory = Memory::from(ipv4);
    
    loop {
        cbc::select! {
            recv(memory_request_rx) -> raw => {
                let request = raw.unwrap();
                match request {
                    MemoryMessage::Request => {
                        let memory_copy = memory.clone();
                        memory_recieve_tx.send(memory_copy).unwrap();
                    }
                    MemoryMessage::UpdateOwnDirection(dirn) => {
                        
                        // Change own direction in memory
                        
                        memory.state_list.get_mut(&memory.my_id).unwrap().direction = dirn;
                    }
                    MemoryMessage::UpdateOwnFloor(floor) => {

                        // Change own floor in memory
                        
                        memory.state_list.get_mut(&memory.my_id).unwrap().last_floor = floor;
                    }
                    
                    MemoryMessage::UpdateOwnCall(call, call_state) => {

                        // Update a single call in memory
                        
                        memory.state_list.get_mut(&memory.my_id).unwrap().call_list.insert(call, call_state); // todo add aceptence test
                    }
                    MemoryMessage::UpdateOthersState(state) => {
                        
                        // Change the requested state in memory
                        
                        memory.state_list.insert(state.id, state);
                    }
                }
            }
        }
    }
}



/* pub fn state_machine_check(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {
    
    loop {
        memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
        let memory = memory_recieve_rx.recv().unwrap();
        let my_state = memory.state_list.get(&memory.my_id).unwrap();
        
    }
}
 */
