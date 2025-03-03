



// note should probebly be a submodule of memory


use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use crate::memory as mem;

use postcard;
use serde::{Serialize, Deserialize};
use heapless;


use postcard::to_stdvec;


const MAXIMUM_BYTES_IN_PACKAGE: usize = 65_000;
const BROADCAST_ADDRESS_bytes: [u8;4] = [255,255,255,255];



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

pub fn net_init_udp_socket(ipv4: Ipv4Addr, wanted_port: u16) -> (UdpSocket, SocketAddrV4) {

    let target_ip = Ipv4Addr::from(BROADCAST_ADDRESS_bytes);

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