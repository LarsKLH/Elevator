
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel as cbc;

enum States {
    nothing,
    new,
    confirmed,
    pending_removal
}

struct Call {
    direction: u8,
    floor: u8,
    call_state: States
}

impl Hash for Call {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.direction.hash(state);
        self.floor.hash(state);
    }
}

struct State {
    id: Macaddr, // Jens fikser
    direction: u8,
    last_floor: u8,
    call_list: HashSet<Call>,
    cab_calls: HashSet<u8>
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

struct Memory {
    my_id: Macaddr, // Jens fikser
    state_list: HashSet<State>
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