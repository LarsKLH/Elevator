use core::hash;
use std::{  net::Ipv4Addr,
            hash::{Hash,Hasher},
            collections::{HashMap, HashSet},
            ops::Deref};

use driver_rust::elevio;
use postcard;
use serde::{Serialize, Deserialize};



use crossbeam_channel::{Receiver, Sender};

use crossbeam_channel as cbc;

use crate::{elevator_interface::MovementState, memory as mem};
use crate::elevator_interface as elevint;


#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub my_id: Ipv4Addr,
    pub state_list: HashMap<Ipv4Addr,State>
}


#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: Ipv4Addr,
    pub move_state: elevint::MovementState, // Jens: alle u8 i denne burde endres til typer tror jeg
    pub last_floor: u8,
    pub call_list: HashMap<Call, CallState>
}

#[derive(Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum CallState {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct Call{
    pub call_type: CallType,
    pub floor: u8
}

#[derive(Eq, PartialEq, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum CallType {
    Cab,
    Hall(elevint::Direction) 
}




pub enum MemoryMessage {
    Request,
    UpdateOwnMovementState(MovementState),
    UpdateOwnFloor(u8),
    UpdateOwnCall(Call, CallState),
    UpdateOthersState(State)
    // TODO krangle om hvordan endre state med update
    // TODO gjøre requests av memory til immutable referanser og update til mutable referanser slik at compileren blir sur om vi ikke gj;r ting riktig
    
    // Mulig fix, gjøre update slik at den sender en init update som låser databasen til den blir skrevet til igjen
}

impl From<Ipv4Addr> for Memory {
    fn from (ip: Ipv4Addr) -> Self {
        Self { my_id: ip,
            state_list: HashMap::from([(ip, State::new(ip))]) 
        } 
    }
    
}

impl State {
    fn new (id_of_new: Ipv4Addr) -> Self {
        Self {  id: id_of_new,
                move_state: elevint::MovementState::StopDoorClosed,
                last_floor: 0,
                call_list: HashMap::new() // need to intitialize with the required number of floors that requires we pass the number of floors 
            }
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

                        // Update a single call in memory
                        memory.state_list.get_mut(&memory.my_id).unwrap().call_list.insert(call, call_state); // todo add aceptence test, sanity check?
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
