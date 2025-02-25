


pub struct Memory {
    my_id: Ipv4Addr, // Jens fikser
    state_list: HashSet<State>
}

pub enum MemoryMessage {
    Request,
    UpdateOwnDirection(u8),
    UpdateOwnFloor(u8),
    UpdateOwnCall(Call),
    UpdateOthersState(State)
    // TODO krangle om hvordan endre state med update
    // TODO gjøre requests av memory til immutable referanser og update til mutable referanser slik at compileren blir sur om vi ikke gj;r ting riktig
    
    // Mulig fix, gjøre update slik at den sender en init update som låser databasen til den blir skrevet til igjen
}


#[derive(Eq, PartialEq)]
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

#[derive(Eq, PartialEq)]
enum States {
    Nothing,
    New,
    Confirmed,
    PendingRemoval
}


#[derive(Eq, PartialEq)]
pub struct State {
    id: Ipv4Addr, // Jens fikser
    direction: u8,
    last_floor: u8,
    call_list: HashSet<Call>,
    cab_calls: HashSet<u8>
}


pub fn memory(memory_recieve_tx: Sender<Memory>, memory_request_rx: Receiver<MemoryMessage>) -> () {
    let memory = Memory::new();

    loop {
        cbc::select! {
            recv(memory_request_rx) -> raw => {
                let request = raw.unwrap();
                match request {
                    MemoryMessage::Request => {
                        memory_recieve_tx.send(memory).unwrap();
                    }
                    MemoryMessage::UpdateOwnDirection(dirn) => {

                        // Change own direction in memory
                        
                        memory.state_list(memory.my_id).direction = dirn;
                    }
                    MemoryMessage::UpdateOwnFloor(floor) => {

                        // Change own floor in memory
                        
                        memory.state_list(memory.my_id).last_floor = floor;
                    }

                    MemoryMessage::UpdateOwnCall(call) => {

                        // Update a single call in memory
                        
                        memory.state_list(memory.my_id).call_list.replace(call); // todo add aceptence test
                    }
                    MemoryMessage::UpdateOthersState(state) => {

                        // Change the requested state in memory

                        memory.state_list.replace(state);
                    }
                }
            }
        }
    }
}
