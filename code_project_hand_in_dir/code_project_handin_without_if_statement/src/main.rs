
use std::net::IpAddr;
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

use local_ip_address::local_ip;




// TODO: change all intences of unwrap to expect with sensible error messages



// Argument list order methinks should be ./elevator_code {number of floors}[an u8] {id/ipv4}[xxx.xxx.xxx.xxx] {socket to broadcast to}[int under like 60 000] {do printout of state and spam the terminal}[true/false] {port the server is on}[int under like 60 000]
fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();

    let my_local_ip = match local_ip().unwrap() {
        IpAddr::V4(v4) => v4,
        _ => panic!("Main: Recieved a non ipv4 local address")
    };

    println!("Main: Found local adress {:?}", my_local_ip, );
    
    if args.len() != 6 {
        panic!("5 arguments are required but {} were provided", args.len()-1)
    }

    let num_floors: u8 = args[1].parse().expect("could not convert the first argument to a u8, could i recomend '4'");

    let ipv4_id: Ipv4Addr = args[2].parse().expect("could not convert the second argument to a ipv4addr, could i recomend '0.0.0.0'");
    
    let socket_number: u16 = args[3].parse().expect("could not convert the second argument to a socket/u16 to broadcast to, could i recomend '26260'");

    let do_the_printout: bool = args[4].parse().expect("could not parse the fourth argument as a boolian value of wheither to do printout, could i recomend 'false'");

    let elevator_server_port_u16_val: u16 = args[5].parse().expect("could not parse the fith argument as a socket/u16 where the server is, could i recomend '15657'");

    
    
    let elevator_server_port_string = format!("localhost:{}",elevator_server_port_u16_val);

    let elevator = elevio::elev::Elevator::init(elevator_server_port_string.as_str(), num_floors)?;

    // Initialize memory access channels
    // - One for requests, one for receiving
    let (memory_request_channel, memory_request_channel_rx) = cbc::unbounded::<mem::MemoryMessage>();
    let (memory_receive_channel_tx, memory_recieve_channel) = cbc::unbounded::<mem::Memory>();

    // Run memory thread
    // - Accesses memory, other functions message it to write or read
    {
        let memory_request_channel_rx = memory_request_channel_rx.clone();
        let memory_receive_channel_tx = memory_receive_channel_tx.clone();
        spawn(move || mem::memory(memory_receive_channel_tx, memory_request_channel_rx, my_local_ip, num_floors));
    }

    // Initialize motor controller channel
    // - Only goes one way
    let (brain_stop_direct_link_tx, brain_stop_direct_link_rx ) = cbc::unbounded::<mem::State>();

    // Run motor controller thread
    // - Accesses motor controls, other functions command it and it updates direction in memory
    {
        let elevator = elevator.clone();

        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        let brain_stop_direct_link = brain_stop_direct_link_rx.clone();
        spawn(move || elevator_interface::elevator_outputs(memory_request_channel, memory_recieve_channel, brain_stop_direct_link, elevator, num_floors));
    }

    // Run button checker thread
    // - Checks buttons, and sends to state machine thread

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();

    {
        let elevator = elevator.clone();

        let memory_request_channel = memory_request_channel.clone();
        let memory_recieve_channel = memory_recieve_channel.clone();
        spawn(move || elevator_interface::elevator_inputs(memory_request_channel, memory_recieve_channel, floor_sensor_tx,elevator));
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
        let brain_stop_direct_link = brain_stop_direct_link_tx.clone();
        spawn(move || brain::elevator_logic(memory_request_channel, memory_recieve_channel, floor_sensor_rx, brain_stop_direct_link, num_floors));
    }

    if do_the_printout {
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