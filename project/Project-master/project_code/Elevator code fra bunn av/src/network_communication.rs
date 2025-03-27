



// note should probebly be a submodule of memory


use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4, SocketAddr};

use std::thread::sleep;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

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

    let native_send_socket = UdpSocket::bind((ipv4, wanted_port)).expect("NetWork: Failed to bind to socket");

    native_send_socket.set_broadcast(true).expect("NetWork: Failed to set socket to broadcast");

    let native_list_socket = native_send_socket.try_clone().expect("NetWork: Failed to clone socket");

    let net_config = NetWorkConfig {
        sending_socket: native_send_socket,
        listning_socket: native_list_socket,
        target_socket: socket_to_target
    };

    return net_config
}


pub fn net_rx(rx_sender_to_memory: Sender<mem::Memory>, net_config: NetWorkConfig) -> () {
    let mut recieve_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];

    let recv_socket = net_config.listning_socket;

    recv_socket.set_nonblocking(false).expect("NetWork: Failed to set the recv socket to non-blocking");

    loop{
        let (number_of_bytes_recieved, address_of_sender) = recv_socket.recv_from(&mut recieve_buffer).expect("NetWork: Failed to recv packet, if this is ever a problem add error handling");

        println!("NetWork: Recieved message of {} bytes from {}", number_of_bytes_recieved, address_of_sender);

        let mut recieved_memory: mem::Memory  = postcard::from_bytes(&recieve_buffer).expect("NetWork: Failed to unpack network message, this needs to be handled in a better way");
        
        let address_of_sender_ipv4 = match address_of_sender {
            SocketAddr::V4(v4) => *v4.ip(),
            _ => panic!("NetWork: Recieved a non ipv4 address")
        };

        /*
        recieved_memory.state_list.get_mut(&recieved_memory.my_id).expect("Network: Could not get state").id = address_of_sender_ipv4;
        let state_get = recieved_memory.state_list.remove(&recieved_memory.my_id).expect("Network: Could not remove state");
        recieved_memory.state_list.insert(address_of_sender_ipv4, state_get);

        recieved_memory.my_id = address_of_sender_ipv4;
        */

        //println!("Network: Received memory from {}", recieved_memory.my_id);
        //println!("Network: Received state list: {:?}", recieved_memory.state_list);

        rx_sender_to_memory.send(recieved_memory).expect("NetWork: Failed to send message to memory");
    }

}

pub fn net_tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, net_config: NetWorkConfig) -> () {
    let mut card_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];
    let from_socket = net_config.sending_socket;
    let to_socket = net_config.target_socket;

    loop {
        memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
        let memory = memory_recieve_rx.recv().unwrap();


        let written_card= postcard::to_slice(&memory, &mut card_buffer).expect("NetWork: Was not able to serialize the memory");


        println!("NetWork: Trying to send post out over the net from {:?} to {:?}", from_socket, to_socket);
        
        from_socket.send_to(&written_card, to_socket).expect("was not able to transmit to target socket");

        sleep(Duration::from_millis(1000)); // The devil made me do it


        // Dersom vi er obstructed burde vi kanskje ikke sende noe så de andre heisene antar at vi er døde
    }
    

}