// use driver_rust::{elevio::elev::{Elevator, DIRN_DOWN}, Elevator::elev};
use crossbeam_channel::{self as cbc, select};
use driver_rust::elevio::elev as e;
use driver_rust::elevio::{self, elev::Elevator};
use network_rust::udpnet;
use std::io::Error;
use std::thread::{self, *};
use std::time::Duration;
mod buttons;
mod floors;
mod fsm;
mod lights;
mod orderqueue;
mod watchdog;
mod server;

use fsm::fsm::{CurrentOrder, ElevFsm};
use orderqueue::orderqueue::{ElevatorQueue, FloorOrder, CabOrder};
use std::sync::{Arc, Mutex};
use server::server::BroadcastMsg;

fn main() -> std::io::Result<()> {
    /*1 Add variables for Floors,.., elevator object*/
    let num_floors: u8 = 4;
    //let elevator = e::Elevator::init("localhost:15657", num_floors)?;
    let elevator = Elevator::init("localhost:15657", num_floors)?;
    let mut queue = ElevatorQueue::new();
    let mut target_floor: u8 = 0;
    let poll_period = Duration::from_millis(25);
    let mut initdone = false;

     const SERVER_IP_PORT : u32 = 20021; 

    //let buf = [1,2,3,4,5];
   
    //elevator.call_button_light(num_floors,num_floors,false);

    //crossbeamchannels for communicating between FSM and broadcasting threads:
    /* let (fsm_to_udp_tx, fsm_to_udp_rx) = cbc::unbounded<BroadcastMsg>();
    let (udp_to_fsm_tx, udp_to_fsm_rx) = cbc::unbounded<BroadcastMsg>(); */

    // channels for communication with the FSM
    let (orders_tx, orders_rx) = cbc::unbounded();
    let (fsm_return_tx, fsm_return_rx) = cbc::unbounded();

    /*2 Create threads to check Hardware buttons and call events on channels */
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let mut elevator = elevator.clone();
        spawn(move ||{
            elevio::poll::call_buttons(elevator, call_button_tx, poll_period);
        } );
    }

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let mut elevator = elevator.clone();
        spawn(move || {
            elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period);
        } );
    }

    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
    {   
        let mut elevator = elevator.clone();
        spawn(move || {
            elevio::poll::stop_button(elevator, stop_button_tx, poll_period);
        } );
    }

    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let mut elevator = elevator.clone();
        spawn(move || {
            elevio::poll::obstruction(elevator, obstruction_tx, poll_period);
        } );
    }

    // Create FSM
    let mut fsm = ElevFsm::new(
        elevator,
        call_button_rx,
        floor_sensor_rx,
        stop_button_rx,
        obstruction_rx,
        orders_rx,
        fsm_return_tx,
        CurrentOrder::new(),
    );


    // Spawn FSM in thread
    thread::spawn(move || {
        fsm.run();
        println!("fsm.run executed in main");
    });

    
    let mut broadcast = BroadcastMsg::new(3);

    broadcast.UDP_broadcast_message();
    broadcast.UDP_listen_message();
    
    
    
    
    loop {
    }


    Ok(())
}

