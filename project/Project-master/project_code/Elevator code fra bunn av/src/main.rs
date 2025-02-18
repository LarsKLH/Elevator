


use driver_rust::elevio;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel as cbc;

mod subfunctions;


fn main() -> std::io::Result<()> {
    let num_floors = 4;
    let elevator = elevio::elev::Elevator::init("localhost:15657", num_floors)?;

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

    // Run memory thread
    // - Accesses memory, other functions message it to write or read
    let (memory_tx, memory_rx) = cbc::unbounded::<bool>(); 

    // Run motor controller thread
    // - Accesses motor controls, other functions command it and it updates direction in memory

    // Run Reciever thread
    // - Recieves broadcasts and sends to sanity check

    // Run sanity check thread
    // - Checks whether changes in order list makes sense

    // Run State machine thread
    // - Checks whether to change the calls in the call lists' state based on recieved broadcasts from other elevators

    {
        let memory_tx = memory_tx.clone();
        let memory_rx = memory_rx.clone();
        spawn(move || subfunctions::state_machine_check(memory_tx, memory_rx));
    }

    // Run Transmitter thread
    // - Constantly sends elevator direction, last floor and call list

    // Run elevator logic thread
    // - Controls whether to stop, go up or down and open door. Sends to motor controller
}