
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

enum Memory_message {
    Request,
    Update(State)
    // TODO krangle om hvordan endre state med update
}

pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<Memory_message>) -> () {
    memory = Memory::new();

    loop {
        cbc::select! {
            recv(memory_request_rx) -> raw => {
                request = raw.unwrap();
                match request {
                    Memory_message::Request => {
                        memory_recieve_tx.send(memory).unwrap();
                    }
                    Memory_message::Update(s) => {
                        // Change the requested state in memory
                        memory.state_list(s.id) = s;
                    }
                }
            }
        }
    }
}

pub fn state_machine_check(memory_tx: Sender<>, memory_rx: Receiver<>) -> () {

}

pub fn sanity_check() -> () {

}

pub fn rx() -> () {

}

pub fn tx() -> () {

}

pub fn motor_controller() -> () {

}

pub fn elevator_logic() -> () {

}

pub fn button_checker() -> () {

}