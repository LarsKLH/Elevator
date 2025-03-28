// note should probebly be a submodule of memory

use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4, SocketAddr};
use std::thread::sleep;
use std::time::{Duration, Instant};  // new: Added Instant for timeout tracking
use std::collections::HashMap;       // new: For tracking sequence numbers
use crossbeam_channel::{Receiver, Sender};
use crate::mem;
use postcard;

const MAXIMUM_BYTES_IN_PACKAGE: usize = 65_000;
const BROADCAST_ADDRESS_BYTES: [u8;4] = [255,255,255,255];
const MAX_RETRIES: u8 = 3;                     // new: Maximum retry attempts
const ACK_TIMEOUT_MS: u64 = 500;               // new: Timeout before retry (ms)
const HEARTBEAT_INTERVAL_MS: u64 = 1000;       // new: Heartbeat frequency (ms)

// new: Packet header structure
#[derive(Serialize, Deserialize)]
struct PacketHeader {
    seq_num: u32,               // Sequence number
    is_ack: bool,               // Is this an ACK packet
    ack_num: Option<u32>,       // Optional ACK number
}

// new: Enhanced network config with reliability features
pub struct NetWorkConfig {
    sending_socket: UdpSocket,
    listning_socket: UdpSocket,
    target_socket: SocketAddrV4,
    next_seq_num: u32,                          // new: Sequence counter
    pending_acks: HashMap<u32, (Instant, Vec<u8>)>, // new: Track unacknowledged packets
    last_heartbeat: Instant,                    // new: Last heartbeat time
}

impl NetWorkConfig {
    pub fn try_clone(&self) -> Self {
       let new_send = self.sending_socket.try_clone().unwrap();
       let new_list = self.listning_socket.try_clone().unwrap();
       let new_target = self.target_socket;
       NetWorkConfig{
        sending_socket: new_send,
        listning_socket: new_list,
        target_socket: new_target,
        next_seq_num: self.next_seq_num,        // new: Copy sequence counter
        pending_acks: HashMap::new(),           // new: New empty map for clones
        last_heartbeat: self.last_heartbeat,    // new: Copy heartbeat time
       }
    }

    // new: Generate next sequence number
    fn next_seq(&mut self) -> u32 {
        let seq = self.next_seq_num;
        self.next_seq_num = self.next_seq_num.wrapping_add(1);
        seq
    }

    // new: Check for expired packets that need retransmission
    fn check_retransmits(&mut self) {
        let now = Instant::now();
        let mut to_retry = Vec::new();

        for (seq, (time, data)) in &self.pending_acks {
            if now.duration_since(*time) > Duration::from_millis(ACK_TIMEOUT_MS) {
                to_retry.push((*seq, data.clone()));
            }
        }

        for (seq, data) in to_retry {
            if let Some((ref mut time, _)) = self.pending_acks.get_mut(&seq) {
                *time = Instant::now();
                self.sending_socket.send_to(&data, self.target_socket)
                    .expect("Failed to retransmit packet");
            }
        }
    }

    // new: Send with reliability
    pub fn reliable_send(&mut self, data: &[u8]) {
        let seq = self.next_seq();
        let header = PacketHeader {
            seq_num: seq,
            is_ack: false,
            ack_num: None,
        };

        let mut packet = Vec::with_capacity(data.len() + 16);
        packet.extend_from_slice(&postcard::to_stdvec(&header).unwrap());
        packet.extend_from_slice(data);

        self.pending_acks.insert(seq, (Instant::now(), packet.clone()));
        self.sending_socket.send_to(&packet, self.target_socket)
            .expect("Failed to send packet");
    }
}

pub fn net_init_udp_socket(ipv4: Ipv4Addr, wanted_port: u16) -> NetWorkConfig {
    let target_ip = Ipv4Addr::from(BROADCAST_ADDRESS_BYTES);
    let socket_to_target = SocketAddrV4::new(target_ip, wanted_port);

    let native_send_socket = UdpSocket::bind((ipv4, wanted_port))
        .expect("NetWork: Failed to bind to socket");
    native_send_socket.set_broadcast(true)
        .expect("NetWork: Failed to set socket to broadcast");

    let native_list_socket = native_send_socket.try_clone()
        .expect("NetWork: Failed to clone socket");

    NetWorkConfig {
        sending_socket: native_send_socket,
        listning_socket: native_list_socket,
        target_socket: socket_to_target,
        next_seq_num: 0,                          // new: Initialize sequence counter
        pending_acks: HashMap::new(),             // new: Initialize ACK tracker
        last_heartbeat: Instant::now(),           // new: Initialize heartbeat timer
    }
}

pub fn net_rx(rx_sender_to_memory: Sender<mem::Memory>, net_config: NetWorkConfig) -> () {
    let mut recieve_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];
    let recv_socket = net_config.listning_socket;
    recv_socket.set_nonblocking(false)
        .expect("NetWork: Failed to set the recv socket to non-blocking");

    loop {
        match recv_socket.recv_from(&mut recieve_buffer) {
            Ok((number_of_bytes_recieved, address_of_sender)) => {
                // new: Parse packet header
                let (header, payload) = parse_packet(&recieve_buffer[..number_of_bytes_recieved]);

                if header.is_ack {
                    // new: Handle ACK packet
                    if let Some(ack_num) = header.ack_num {
                        net_config.pending_acks.remove(&ack_num);
                    }
                } else {
                    // new: Send ACK response
                    let ack_header = PacketHeader {
                        seq_num: 0, // Doesn't matter for ACKs
                        is_ack: true,
                        ack_num: Some(header.seq_num),
                    };
                    let ack_packet = postcard::to_stdvec(&ack_header).unwrap();
                    recv_socket.send_to(&ack_packet, address_of_sender).ok();

                    // Process payload
                    if let Ok(recieved_memory) = postcard::from_bytes(payload) {
                        rx_sender_to_memory.send(recieved_memory)
                            .expect("NetWork: Failed to send message to memory");
                    }
                }
            },
            Err(e) => {
                eprintln!("Network receive error: {}", e);
            }
        }
    }
}

// new: Helper function to parse packet header and payload
fn parse_packet(data: &[u8]) -> (PacketHeader, &[u8]) {
    // Simple implementation - adjust based on your actual header size
    if data.len() < 16 {
        panic!("Malformed packet");
    }
    let header: PacketHeader = postcard::from_bytes(&data[..16]).unwrap();
    (header, &data[16..])
}

pub fn net_tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, mut net_config: NetWorkConfig) -> () {
    let mut card_buffer: [u8; MAXIMUM_BYTES_IN_PACKAGE] = [0; MAXIMUM_BYTES_IN_PACKAGE];
    
    loop {
        // new: Check for needed retransmits
        net_config.check_retransmits();

        // new: Send periodic heartbeat
        if net_config.last_heartbeat.elapsed() > Duration::from_millis(HEARTBEAT_INTERVAL_MS) {
            memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
            let memory = memory_recieve_rx.recv().unwrap();
            
            let written_card = postcard::to_slice(&memory, &mut card_buffer)
                .expect("NetWork: Was not able to serialize the memory");
            
            net_config.reliable_send(written_card);
            net_config.last_heartbeat = Instant::now();
        }

        sleep(Duration::from_millis(100));
    }
}