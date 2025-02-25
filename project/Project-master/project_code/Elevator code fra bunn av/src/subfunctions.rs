
use std::default;
use std::hash::Hash;
use std::hash::Hasher;
use std::collections::HashSet;
use std::net::Ipv6Addr;
use std::thread::*;
use std::time::*;
use std::u8;
use std::sync::*;
use std::cmp::max;


use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;

use driver_rust::elevio::{self, elev::{self, Elevator}};


use crate::memory as mem;

use crate::motor_controller as mot;

use crate::network_communication as netwrk;




pub fn state_machine_check(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}



pub fn rx(rx_send: Sender<mem::State>) -> () {

}

pub fn tx(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}


pub fn elevator_logic(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>) -> () {

}

pub fn button_checker() -> () {

}