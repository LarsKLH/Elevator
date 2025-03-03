


use std::time::*;
use std::thread::*;



use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;



use driver_rust::elevio::elev::DIRN_STOP;
use driver_rust::elevio::{self, elev::{self, Elevator}};
use crate::memory as mem;






pub enum MotorMessage {
    Up,
    Down,
    EmergencyStop,
    StopAndOpen
}

// Motor controller function. Takes controller messages and sends them to the elevator
// controller. Also updates the memory with the current direction of the elevator
pub fn motor_controller(memory_request_tx: Sender<mem::MemoryMessage>, motor_controller_receive: Receiver<MotorMessage>, elevator: Elevator) -> () {
    // Create direction variable and send elevator down until it hits a floor
    let mut direction = elevio::elev::DIRN_DOWN;
    elevator.motor_direction(direction);

    // Update direction in memory
    memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();

    // Infinite loop checking for motor controller messages
    loop {
        cbc::select! {
            recv(motor_controller_receive) -> order => {
                let received_order = order.unwrap();
                match received_order {
                    MotorMessage::Down => {
                        // Turn off elevator light before starting
                        elevator.door_light(false);
                        sleep(Duration::from_millis(500));

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_DOWN;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::Up => {
                        // Turn off elevator light before starting
                        elevator.door_light(false);
                        sleep(Duration::from_millis(500));

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_UP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::EmergencyStop => {
                        // Turn off elevator light just in case
                        elevator.door_light(false);

                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();
                    }
                    MotorMessage::StopAndOpen => {
                        // Change direction and update memory
                        direction = elevio::elev::DIRN_STOP;
                        elevator.motor_direction(direction);
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnDirection(direction)).unwrap();

                        // Turn on light for now
                        elevator.door_light(true);
                    }
                }
            }
        }
    }
}

// The main elevator logic. Determines where to go next and sends commands to the motor controller
pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<MotorMessage>) -> () {
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
                        motor_controller_send.send(MotorMessage::StopAndOpen).unwrap();

                        sleep(Duration::from_millis(3000));

                        let memory_request_tx = memory_request_tx.clone();
                        let memory_recieve_rx = memory_recieve_rx.clone();
                        let my_state_copy = my_state.clone();
                        restart_elevator(memory_request_tx, memory_recieve_rx, my_state_copy);
                    }
                }
                recv(cbc::after(Duration::from_millis(100))) -> _a => {
                    println!("No new floor received, refreshing");
                }
            }
        }
    }
}

fn restart_elevator(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, my_state_copy: mem::State) -> () {

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
    else if !lower_calls(new_floor, my_state) {
        return true;
    }
    else {
        return false;
    }
}

fn lower_calls(new_floor: u8, my_state: &mem::State) -> bool {
    for floor in my_state.call_list {
    }
    return true
}


// Probably not needed
pub fn button_checker() -> () {

}