
use std::net::Ipv4Addr;
use std::thread::*;
use std::time::*;
use std::u8; // what?

use driver_rust::elevio;

use crossbeam_channel as cbc;

mod memory;
mod elevator_interface;
mod network_communication;
mod brain;
mod sanity;

use crate::memory as mem;

use std::env;


// TODO: change all intences of unwrap to expect with sensible error messages



// Argument list order methinks should be ./elevator_code {number of floors}[an u8] {id/ipv4}[xxx.xxx.xxx.xxx] {socket to broadcast to}[int under like 60 000]
fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();

    //print!("arguments are: arg 1 = {}, arg 2 = {}, arg 3 = {}", args[1], args[2], args[3]);


    let num_floors: u8 = args[1].parse().expect("could not convert the first argument to a u8");

    let ipv4_id: Ipv4Addr = args[2].parse().expect("could not convert the second argument to a ipv4addr");
    
    let socket_number: u16 = args[3].parse().expect("could not convert the second argument to a socket/u16");


    let elevator = elevio::elev::Elevator::init("localhost:15657", num_floors)?;

    // Initialize memory access channels
    // - One for requests, one for receiving
    let (memory_request_channel, memory_request_channel_rx) = cbc::unbounded::<mem::MemoryMessage>();
    let (memory_receive_channel_tx, memory_recieve_channel) = cbc::unbounded::<mem::Memory>();

    // Run memory thread
    // - Accesses memory, other functions message it to write or read
    {
        let memory_request_channel_rx = memory_request_channel_rx.clone();
        let memory_receive_channel_tx = memory_receive_channel_tx.clone();
        spawn(move || mem::memory(memory_receive_channel_tx, memory_request_channel_rx, ipv4_id, num_floors));
    }

    // Initialize motor controller channel
    // - Only goes one way
    let (elevator_outputs_send, elevator_outputs_receive) = cbc::unbounded::<mem::State>();

    // Run motor controller thread
    // - Accesses motor controls, other functions command it and it updates direction in memory
    {
        let elevator = elevator.clone();

        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        let elevator_outputs_receive = elevator_outputs_receive.clone();
        spawn(move || elevator_interface::elevator_outputs(memory_request_channel, memory_recieve_channel, elevator_outputs_receive, elevator, num_floors));
    }

    // Run button checker thread
    // - Checks buttons, and sends to state machine thread

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();

    {
        let elevator = elevator.clone();

        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        spawn(move || elevator_interface::elevator_inputs(memory_request_channel, memory_recieve_channel, floor_sensor_tx,elevator, num_floors));
    }


    let net_config = network_communication::net_init_udp_socket(ipv4_id, socket_number);

    // Initialize rx channel
    // - Only goes one way
    let (rx_send, rx_get) = cbc::unbounded::<mem::Memory>();

    // Run Reciever thread
    // - Recieves broadcasts and sends to sanity check
    {
        let rx_send = rx_send.clone();
        let rx_net_config = net_config.try_clone();
        spawn(move || network_communication::net_rx(rx_send, rx_net_config));
    }

    // Run sanity check thread
    // - Checks whether changes in order list makes sense
    {
        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        let rx_get = rx_get.clone();
        spawn(move || sanity::sanity_check_incomming_message(memory_request_channel, memory_recieve_channel, rx_get));
    }


    /* 

    Deprecated code

    // Run State machine thread
    // - Checks whether to change the calls in the call lists' state based on recieved broadcasts from other elevators
    {
        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        spawn(move || mem::state_machine_check(memory_request_channel, memory_recieve_channel));
    }
    */


    // Run Transmitter thread
    // - Constantly sends elevator direction, last floor and call list
    {
        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        let tx_net_config = net_config.try_clone();
        spawn(move || network_communication::net_tx(memory_request_channel, memory_recieve_channel, tx_net_config));
    }

    // Run elevator logic thread
    // - Controls whether to stop, go up or down and open door. Sends to motor controller
    {
        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        let floor_sensor_rx = floor_sensor_rx.clone();
        spawn(move || brain::elevator_logic(memory_request_channel, memory_recieve_channel, floor_sensor_rx));
    }

    {
        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        spawn(move || mem::printout(memory_request_channel, memory_recieve_channel));
    }


    // Loop forever, error handling goes here somewhere
    loop {
        sleep(Duration::from_millis(1000));
        // Do nothing
    }
}