use std::thread::{self, sleep};
use std::sync::mpsc::{channel,Receiver,Sender};
use std::time::Duration;
use std::net::TcpStream;
use network_rust::udpnet;
use crossbeam_channel as cbc;
use serde::{Serialize, Deserialize};
//use orderqueue::{ElevatorQueue, FloorOrder, CabOrder};
use crate::orderqueue::orderqueue::{ElevatorQueue};


   
// do stuff with udpnet::peers::tx(), or similar

//const IP_ADDR : &str = "127.0.0.1";
//const SERVER_IP_PORT : u32 = [20021,20022,20023]; 

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BroadcastMsg {
    //queue: ElevatorQueue,
    floor: u8,
}

impl BroadcastMsg{

    pub fn new(floor: u8) -> Self {
        BroadcastMsg { floor }
    }
    
    pub fn UDP_broadcast_message(&self) {
        println!("Function UDP_broadcast_message started");

        // let stream1 = TcpStream::connect("127.0.0.1:20021").unwrap().local_addr().unwrap().ip();
        // let stream2 = TcpStream::connect("127.0.0.1:20022").unwrap().local_addr().unwrap().ip();
        // let stream3 = TcpStream::connect("127.0.0.1:20023").unwrap().local_addr().unwrap().ip();

        let (alive_tx, alive_rx) = cbc::unbounded::<BroadcastMsg>();
        
        let mut broadcast_msg = BroadcastMsg {
           // queue: ElevatorQueue::new(),
            floor: 0
        };

        thread::spawn(move || {
            println!("About to initiate sender");

            udpnet::bcast::tx(19735, alive_rx).expect("Error when starting the (TCP/UDP/???) sender");

        });



        thread::spawn(move || {

            loop {
                println!("About to send message to cbm channel :D");
                alive_tx.send(broadcast_msg.clone()).expect("Error when sendnig broadcast message via cbc");
                println!("Message sent to crossbeam channel");
                thread::sleep(Duration::from_millis(100));
            }
            
        });

        
    }


    pub fn UDP_listen_message(&self) {
        let (listening_tx, listening_rx) = cbc::unbounded::<BroadcastMsg>();

        thread::spawn(move || {
            udpnet::bcast::rx(19735, listening_tx).expect("Error when receiving broadcast message via cbc");  
        });

        thread::spawn(move || {

        loop {

        match listening_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(msg) => {
                println!("message received: {:?}", msg);
            }
            Err(_) => {
                println!("No message received");
            }
        }
        sleep(Duration::from_millis(200));
        }
        });
    }
}

