



// note should probebly be a submodule of memory


use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use crate::memory as mem;



pub fn sanity_check_incomming_message(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::State>) -> () {
    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
                let old_memory = memory_recieve_rx.recv().unwrap();

                let recieved_state = rx.unwrap();
                let old_calls = old_memory.state_list.get(&recieved_state.id).unwrap().call_list.clone();
                !todo!("get the changes in the ") //let changes = recieved_state.call_list.eq(&old_calls);
                
            }
        }
    }
}


pub fn rx(rx_send: Sender<mem::State>) -> () {

}

pub fn tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}