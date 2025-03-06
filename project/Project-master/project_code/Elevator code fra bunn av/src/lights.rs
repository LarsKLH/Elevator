use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;


use crate::memory as mem;
use crate::motor_controller as motcon;

use driver_rust::elevio::{self, elev::{self, Elevator}};


#[derive(Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum LightState {
    On,
    Off
}

pub struct Lights { // Be added under memory struct if we want to keep it in the same file
    pub hall_lights: HashMap<Call, LightState>,
    pub cab_lights: HashMap<u8, LightState>
}

pub fn let_there_be_light(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, floor_sensor_rx: Receiver<u8>, motor_controller_send: Sender<motcon::MotorMessage>) -> () {

    loop {
        memory_request_tx.send(mem::MemoryMessage::Request).unwrap();
        let memory = memory_recieve_rx.recv().unwrap();
        let my_state = memory.state_list.get(&memory.my_id).unwrap();
        let cab_calls = my_state.cab_calls.clone();
        let call_list = my_state.call_list.clone();
        
    }