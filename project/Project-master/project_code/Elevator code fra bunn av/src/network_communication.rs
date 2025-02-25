



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
                let old_calls = old_memory.state_list.get(recieved_state.id).unwrap().call_list;
                let changes = recieved_state.call_list.difference(&old_calls);
                
            }
        }
    }
}