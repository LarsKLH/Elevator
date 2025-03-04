



// note should probebly be a submodule of memory


use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};

use std::collections::HashMap;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use crate::mem;

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