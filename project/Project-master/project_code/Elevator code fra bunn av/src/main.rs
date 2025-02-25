
use std::default;
use std::hash::Hash;
use std::net::Ipv6Addr;
use std::thread::*;
use std::time::*;
use std::collections::HashSet;
use std::u8;
use std::sync::*;
use std::cmp::max;

use driver_rust::elevio;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel as cbc;
use motor_controller::motor_controller;

mod subfunctions;
mod memory;
mod motor_controller;
mod network_communication;

use crate::memory as mem;

use std::env;




fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();

    let ipv6_id = Ipv6Addr::from(args[0]);

    let num_floors = 4;
    let elevator = elevio::elev::Elevator::init("localhost:15657", num_floors)?;

    // Set poll period for buttons and sensors
    let poll_period = Duration::from_millis(25);

    // Initialize button sensors
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>(); // Initialize call buttons
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_button_tx, poll_period));
    }
     // Initialize floor sensor
     let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period));
    }
    // Initialize stop button
    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_button_tx, poll_period));
    }
    // Initialize obstruction switch
    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>(); 
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstruction_tx, poll_period));
    }

    // Run button checker thread
    // - Checks buttons, and sends to state machine thread

    {
        spawn(move || motor_controller::button_checker());
    }

    // Initialize memory access channels
    // - One for requests, one for receiving
    let (memory_request_tx, memory_request_rx) = cbc::unbounded::<mem::MemoryMessage>();
    let (memory_recieve_tx, memory_recieve_rx) = cbc::unbounded::<mem::Memory>();

    // Run memory thread
    // - Accesses memory, other functions message it to write or read
    {
        let memory_request_rx = memory_request_rx.clone();
        let memory_recieve_tx = memory_recieve_tx.clone();
        spawn(move || mem::memory(memory_recieve_tx, memory_request_rx, ipv6));
    }

    // Initialize motor controller channel
    // - Only goes one way
    let (motor_controller_send, motor_controller_receive) = cbc::unbounded::<motor_controller::MotorMessage>();

    // Run motor controller thread
    // - Accesses motor controls, other functions command it and it updates direction in memory
    {
        let elevator = elevator.clone();

        let memory_request_tx = memory_request_tx.clone();
        let motor_controller_receive = motor_controller_receive.clone();
        spawn(move || motor_controller::motor_controller(memory_request_tx, motor_controller_receive, elevator));
    }

    // Initialize rx channel
    // - Only goes one way
    let (rx_send, rx_get) = cbc::unbounded::<mem::State>();

    // Run Reciever thread
    // - Recieves broadcasts and sends to sanity check
    {
        let rx_send = rx_send.clone();
        spawn(move || network_communication::rx(rx_send));
    }

    // Run sanity check thread
    // - Checks whether changes in order list makes sense
    {
        let memory_request_tx = memory_request_tx.clone();
        let memory_recieve_rx = memory_recieve_rx.clone();
        let rx_get = rx_get.clone();
        spawn(move || network_communication::sanity_check_incomming_message(memory_request_tx, memory_recieve_rx, rx_get));
    }

    // Run State machine thread
    // - Checks whether to change the calls in the call lists' state based on recieved broadcasts from other elevators
    {
        let memory_request_tx = memory_request_tx.clone();
        let memory_recieve_rx = memory_recieve_rx.clone();
        spawn(move || mem::state_machine_check(memory_request_tx, memory_recieve_rx));
    }

    // Run Transmitter thread
    // - Constantly sends elevator direction, last floor and call list
    {
        let memory_request_tx = memory_request_tx.clone();
        let memory_recieve_rx = memory_recieve_rx.clone();
        spawn(move || network_communication::tx(memory_request_tx, memory_recieve_rx));
    }

    // Run elevator logic thread
    // - Controls whether to stop, go up or down and open door. Sends to motor controller
    {
        let memory_request_tx = memory_request_tx.clone();
        let memory_recieve_rx = memory_recieve_rx.clone();
        let floor_sensor_rx = floor_sensor_rx.clone();
        let motor_controller_send = motor_controller_send.clone();
        spawn(move || motor_controller::elevator_logic(memory_request_tx, memory_recieve_rx, floor_sensor_rx, motor_controller_send));
    }

    // Loop forever, error handling goes here somewhere
    loop {
        sleep(Duration::from_millis(1000));
        // Do nothing
    }
}