
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel as cbc;

enum states {
    nothing,
    new,
    confirmed,
    pending_removal
}

fn state_machine_check(memory_tx: Sender<>, memory_rx: Receiver<>) -> () {

}

fn sanity_check() -> () {

}

fn rx() -> () {

}

fn tx() -> () {

}

fn motor_controller() -> () {

}

fn elevator_logic() -> () {

}

fn button_checker() -> () {

}

fn memory() -> () {
    
}