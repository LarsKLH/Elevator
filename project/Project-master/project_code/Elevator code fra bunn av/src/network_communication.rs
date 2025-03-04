



// note should probebly be a submodule of memory


use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};

use std::collections::HashMap;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use crate::memory as mem;

use postcard;


const MAXIMUM_BYTES_IN_PACKAGE: usize = 65_000;
const BROADCAST_ADDRESS_BYTES: [u8;4] = [255,255,255,255];


pub struct NetWorkConfig {
    sending_socket: UdpSocket,
    listning_socket: UdpSocket,
    target_socket: SocketAddrV4,

}

impl NetWorkConfig {
    pub fn try_clone(&self) -> Self {
       let new_send = self.sending_socket.try_clone().unwrap();
       let new_list = self.listning_socket.try_clone().unwrap();
       let new_target = self.target_socket;
       NetWorkConfig{
        sending_socket: new_send,
        listning_socket: new_list,
        target_socket: new_target
       }
    }
}



// TODO: Give better name or make a description
fn state_machine(state_to_change: HashMap<mem::Call, mem::CallState>, state_list: &HashMap<Ipv4Addr, mem::State>) -> HashMap<mem::Call, mem::CallState> {
    for mut call in &state_to_change {
        match call.1 {
            mem::CallState::Nothing => {
                // If one of the others has a new order that passed sanity check, change our state to new
                for state in state_list {
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::New {
                        call.1 = &mem::CallState::New;
                        break;
                    }
                }
            }
            mem::CallState::New => {
                // If all the others are either new or confirmed, change our state to confirmed
                let mut new = 0;
                let mut confirmed = 0;
                let mut total = 0;
                for state in state_list {
                    total += 1;
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::New {
                        new += 1;
                    }
                    else if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::Confirmed {
                        confirmed += 1;
                    }
                }
                if (new + confirmed) == total {
                    call.1 = &mem::CallState::Confirmed;
                }
            }
            mem::CallState::Confirmed => {
                // If one of the others has removed an order that passed sanity check, change our state to new
                for state in state_list {
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::PendingRemoval {
                        call.1 = &mem::CallState::PendingRemoval;
                        break;
                    }
                }
            }
            mem::CallState::PendingRemoval => {
                // If all the others are either pending or nothing, change our state to nothing
                // it an PendingRemoval is in memory it has to have passed the sanity check
                // TODO check if the sanity check allows other elevators to acsept PendingRemoval of other elevators
                let mut pending = 0;
                let mut nothing = 0;
                let mut total = 0;
                for state in state_list {
                    total += 1;
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::PendingRemoval {
                        pending += 1;
                    }
                    else if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::Nothing {
                        nothing += 1;
                    }
                }
                if (pending + nothing) == total {
                    call.1 = &mem::CallState::Nothing;
                }
            }
        }
    }
    return state_to_change.clone();
}

// Gets the difference between two call lists
fn difference(old_calls: HashMap<mem::Call, mem::CallState>, new_calls: HashMap<mem::Call, mem::CallState>) -> HashMap<mem::Call, mem::CallState> {
    let mut difference: HashMap<mem::Call, mem::CallState> = HashMap::new();
    for call in old_calls.keys() {
        if new_calls.get(call) != old_calls.get(call) {
            difference.insert(call.clone(), *new_calls.get(call).unwrap());
        }
    }
    return difference;
}

// Sanity check and state machine function. Only does something when a new state is received from another elevator
pub fn sanity_check_incomming_message(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::State>) -> () {
    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                // Getting old memory and extracting my own state
                memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
                let old_memory = memory_recieve_rx.recv().unwrap();
                let my_state = old_memory.state_list.get(&old_memory.my_id).unwrap().clone();

                // Getting new state from rx, extracting both old and new calls for comparison
                let received_state = rx.unwrap();
                let old_calls = old_memory.state_list.get(&received_state.id).unwrap().call_list.clone();
                let new_calls = received_state.call_list.clone();

                // Getting the difference between the old and new calls
                let differences = difference(old_calls.clone(), new_calls.clone());
                // Getting the relevant calls from my state
                let my_diff: HashMap<mem::Call, mem::CallState> = my_state.call_list.into_iter().filter(|x| differences.contains_key(&x.0)).collect();

                // Copying the old state list and adding the changes
                let mut state_list_with_changes = old_memory.state_list.clone();
                for change in &differences {
                    state_list_with_changes.get_mut(&received_state.id).unwrap().call_list.insert(change.0.clone(), change.1.clone());
                }

                // Running the state machine on only the changed calls
                let my_diff_changed = state_machine(my_diff.clone(), &state_list_with_changes);

                // Extracting the calls that were actually changed to minimize memory changing and avoid errors
                let changed_calls = difference(my_diff, my_diff_changed);

                // Sending the changes to memory one after the other
                for change in changed_calls {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).unwrap();
                }
            }
            // If we don't get a new state in decent time, this function runs
            default(Duration::from_millis(100)) => {
                // Getting old memory and extracting my own call list
                let old_memory = memory_recieve_rx.recv().unwrap();
                let my_call_list = old_memory.state_list.get(&old_memory.my_id).unwrap().clone().call_list;

                // Running the state machine on my own calls
                let new_call_list = state_machine(my_call_list.clone(), &old_memory.state_list);

                // Extracting the calls that were actually changed to minimize memory changing and avoid errors
                let changed_calls = difference(my_call_list, new_call_list);

                // Sending the changes to memory one after the other
                for change in changed_calls {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).unwrap();
                }
            }
        }
    }
}

pub fn net_init_udp_socket(ipv4: Ipv4Addr, wanted_port: u16) -> NetWorkConfig {

    let target_ip = Ipv4Addr::from(BROADCAST_ADDRESS_BYTES);

    let socket_to_target = SocketAddrV4::new(target_ip, wanted_port);

    let native_send_socket = UdpSocket::bind((ipv4, wanted_port)).unwrap();
    let native_list_socket = native_send_socket.try_clone().unwrap();

    let net_config = NetWorkConfig {
        sending_socket: native_send_socket,
        listning_socket: native_list_socket,
        target_socket: socket_to_target
    };

    return net_config
}


pub fn net_rx(rx_sender_to_memory: Sender<mem::State>, net_config: NetWorkConfig) -> () {
    let mut recieve_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];

    let recv_socket = net_config.listning_socket;

    recv_socket.set_nonblocking(false).unwrap();

    loop{
        recv_socket.recv(&mut recieve_buffer);

        let recieved_memory: mem::State  = postcard::from_bytes(&recieve_buffer).unwrap();
    
        rx_sender_to_memory.send(recieved_memory);    
    }

}

pub fn net_tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, net_config: NetWorkConfig) -> () {
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();

    let mut card_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];

    let written_card= postcard::to_slice(&memory, &mut card_buffer).unwrap();

    let from_socket = net_config.sending_socket;
    let to_socket = net_config.target_socket;

    from_socket.send_to(&written_card, to_socket);

}