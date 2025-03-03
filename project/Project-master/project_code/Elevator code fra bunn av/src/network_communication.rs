



// note should probebly be a submodule of memory


use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};

use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use crate::memory as mem;

use postcard;


const MAXIMUM_BYTES_IN_PACKAGE: usize = 65_000;
const BROADCAST_ADDRESS_BYTES: [u8;4] = [255,255,255,255];

fn difference(old_calls: HashMap<mem::Call, mem::CallState>, new_calls: HashMap<mem::Call, mem::CallState>) -> HashMap<mem::Call, mem::CallState> {
    let mut difference: HashMap<mem::Call, mem::CallState> = HashMap::new();
    for call in old_calls.keys() {
        if new_calls.get(call) != old_calls.get(call) {
            difference.insert(call.clone(), *new_calls.get(call).unwrap());
        }
    }
    return difference;
}

pub fn sanity_check_incomming_message(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::State>) -> () {
    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
                let old_memory = memory_recieve_rx.recv().unwrap();
                let my_state = old_memory.state_list.get(&old_memory.my_id).unwrap().clone();

                let recieved_state = rx.unwrap();
                let old_calls = old_memory.state_list.get(&recieved_state.id).unwrap().call_list.clone();
                let new_calls = recieved_state.call_list.clone();

                let difference = difference(old_calls.clone(), new_calls.clone());
                let my_diff: HashMap<mem::Call, mem::CallState> = my_state.call_list.iter().filter(|x| difference.contains_key(&x.0)).collect();

                for mut call in my_diff {
                    match call.1 {
                        mem::CallState::Nothing => {
                            // If one of the others has a new order that passed sanity check, change our state to new
                            for state in &memory.state_list {
                                if *state.1.call_list.get(call.0).unwrap() == mem::CallState::New {
                                    call.1 = mem::CallState::New;
                                    break;
                                }
                            }
                        }
                        mem::CallState::New => {
                            // If all the others are either new or confirmed, change our state to confirmed
                            let mut new = 0;
                            let mut confirmed = 0;
                            let mut total = 0;
                            for state in &memory.state_list {
                                total += 1;
                                if *state.1.call_list.get(call.0).unwrap() == mem::CallState::New {
                                    new += 1;
                                }
                                else if *state.1.call_list.get(call.0).unwrap() == mem::CallState::Confirmed {
                                    confirmed += 1;
                                }
                            }
                            if (new + confirmed) == total {
                                call.1 = &mem::CallState::Confirmed;
                            }
                        }
                        mem::CallState::Confirmed => {
                            // If one of the others has removed an order that passed sanity check, change our state to new
                            for state in &memory.state_list {
                                if *state.1.call_list.get(call.0).unwrap() == mem::CallState::PendingRemoval {
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
                            for state in &memory.state_list {
                                total += 1;
                                if *state.1.call_list.get(call.0).unwrap() == mem::CallState::PendingRemoval {
                                    pending += 1;
                                }
                                else if *state.1.call_list.get(call.0).unwrap() == mem::CallState::Nothing {
                                    nothing += 1;
                                }
                            }
                            if (pending + nothing) == total {
                                call.1 = &mem::CallState::Nothing;
                            }
                        }
                    }
                }
                
            }
        }
    }
}

pub fn net_init_udp_socket(ipv4: Ipv4Addr, wanted_port: u16) -> (UdpSocket, SocketAddrV4) {

    let target_ip = Ipv4Addr::from(BROADCAST_ADDRESS_BYTES);

    let target_socket = SocketAddrV4::new(target_ip, wanted_port);

    let native_socket = UdpSocket::bind((ipv4, wanted_port)).unwrap();

    return (native_socket, target_socket);
}


pub fn net_rx(rx_sender_to_memory: Sender<mem::Memory>, listning_socket: UdpSocket) -> () {
    let mut recieve_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];

    listning_socket.set_nonblocking(false).unwrap();

    loop{
        listning_socket.recv(&mut recieve_buffer);

        let recieved_memory: mem::Memory  = postcard::from_bytes(&recieve_buffer).unwrap();
    
        rx_sender_to_memory.send(recieved_memory);    
    }

}

pub fn net_tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, sending_socket: UdpSocket, target_socket: SocketAddrV4) -> () {
    memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
    let memory = memory_recieve_rx.recv().unwrap();

    let mut card_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];

    let written_card= postcard::to_slice(&memory, &mut card_buffer).unwrap();

    sending_socket.send_to(&written_card, target_socket);

}