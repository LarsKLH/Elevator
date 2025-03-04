

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;


use crate::memory as mem;
use crate::motor_controller as motcon;

use driver_rust::elevio::{self, elev::{self, Elevator}};





// The main elevator logic. Determines where to go next and sends commands to the motor controller
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {

    loop {
        memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
        let memory = memory_recieve_rx.recv().unwrap();
        let my_state = memory.state_list.get(&memory.my_id).unwrap();
        let current_direction = my_state.direction;
        if current_direction == elevio::elev::DIRN_STOP {
            // If stopped restart elevator as needed
            let memory_request_tx = memory_request_tx.clone();
            let memory_recieve_rx = memory_recieve_rx.clone();
            let my_state_copy = my_state.clone();
            restart_elevator(memory_request_tx, memory_recieve_rx, my_state_copy);
        }
        else {
            cbc::select! {
                recv(floor_sensor_rx) -> a => {
                    println!("New floor received, checking whether or not to stop");
                    if should_i_stop(a.unwrap(), my_state) {
                        // If we have determined to stop, stop, wait and restart
                        println!("Should stop");
                        motor_controller_send.send(motcon::MotorMessage::StopAndOpen).unwrap();

                        thread::sleep(Duration::from_millis(3000));

                        let memory_request_tx = memory_request_tx.clone();
                        let memory_recieve_rx = memory_recieve_rx.clone();
                        let my_state_copy = my_state.clone();
                        restart_elevator(memory_request_tx, memory_recieve_rx, my_state_copy);
                    }
                }
                recv(cbc::after(Duration::from_millis(100))) -> _a => {
                    println!("No new floor received, refreshing");
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

fn restart_elevator(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, my_state_copy: mem::State) -> () {
    match my_state_copy.direction {
        elevio::elev::DIRN_STOP => {

        }
        elevio::elev::DIRN_UP => {

        }
        elevio::elev::DIRN_DOWN => {

        }
        2_u8..=254_u8 => {
            println!("Error: invalid direction")
        }
    }
}

// Check whether we should stop or not
fn should_i_stop(new_floor: u8, my_state: &mem::State) -> bool {

    let check_call = mem::Call {
        direction: my_state.direction,
        floor: new_floor
    };
    // If the state of our current floor is confirmed, we should stop
    if *my_state.call_list.get(&check_call).unwrap() == mem::CallState::Confirmed {
        return true;
    }
    // if there are no more floors below us, we should stop
    else if !lower_calls(new_floor, my_state.clone()) {
        return true;
    }
    else {
        return false;
    }
}

fn lower_calls(new_floor: u8, my_state: mem::State) -> bool {

    match my_state.direction {
        elevio::elev::DIRN_UP => {
            for call in my_state.call_list {
                if call.0.floor > new_floor && call.1 == mem::CallState::Confirmed {
                    return false;
                }
            }
        }
        elevio::elev::DIRN_DOWN => {
            for call in my_state.call_list {
                if call.0.floor < new_floor && call.1 == mem::CallState::Confirmed {
                    return false;
                }
            }
        }
        0_u8|2_u8..=254_u8 => {
            println!("Error: Direction not valid");
        }
    }
    return true
}