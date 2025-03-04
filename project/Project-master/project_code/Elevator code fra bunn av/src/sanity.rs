
use std::collections::HashMap;
use std::net::Ipv4Addr;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;
use std::time::Duration;

use crate::memory as mem;


// TODO: Give better name or make a description
fn cyclic_counter(state_to_change: HashMap<mem::Call, mem::CallState>, state_list: &HashMap<Ipv4Addr, mem::State>) -> HashMap<mem::Call, mem::CallState> {
    for mut call in &state_to_change {
        match call.1 {
            mem::CallState::Nothing => {
                // If one of the others has a new order that passed sanity check, change our state to new
                for state in state_list {
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::New {
                        call.1 = &mem::CallState::New;
                        break;
                    }
                }
            }
            mem::CallState::New => {
                // If all the others are either new or confirmed, change our state to confirmed
                let mut new = 0;
                let mut confirmed = 0;
                let mut total = 0;
                for state in state_list {
                    total += 1;
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::New {
                        new += 1;
                    }
                    else if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::Confirmed {
                        confirmed += 1;
                    }
                }
                if (new + confirmed) == total {
                    call.1 = &mem::CallState::Confirmed;
                }
            }
            mem::CallState::Confirmed => {
                // If one of the others has removed an order that passed sanity check, change our state to new
                for state in state_list {
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::PendingRemoval {
                        call.1 = &mem::CallState::PendingRemoval;
                        break;
                    }
                }
            }
            mem::CallState::PendingRemoval => {
                // If all the others are either pending or nothing, change our state to nothing
                // it an PendingRemoval is in memory it has to have passed the sanity check
                // TODO check if the sanity check allows other elevators to acsept PendingRemoval of other elevators
                let mut pending = 0;
                let mut nothing = 0;
                let mut total = 0;
                for state in state_list {
                    total += 1;
                    if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::PendingRemoval {
                        pending += 1;
                    }
                    else if *state.1.call_list.get(&call.0).unwrap() == mem::CallState::Nothing {
                        nothing += 1;
                    }
                }
                if (pending + nothing) == total {
                    call.1 = &mem::CallState::Nothing;
                }
            }
        }
    }
    return state_to_change.clone();
}

// Gets the difference between two call lists
fn difference(old_calls: HashMap<mem::Call, mem::CallState>, new_calls: HashMap<mem::Call, mem::CallState>) -> HashMap<mem::Call, mem::CallState> {
    let mut difference: HashMap<mem::Call, mem::CallState> = HashMap::new();
    for call in old_calls.keys() {
        if new_calls.get(call) != old_calls.get(call) {
            difference.insert(call.clone(), *new_calls.get(call).unwrap());
        }
    }
    return difference;
}

fn insanity(differences: HashMap<mem::Call, mem::CallState>, received_state: mem::State, state_list_with_changes: HashMap<Ipv4Addr, mem::State>) -> HashMap<mem::Call, mem::CallState> {
    let mut new_differences = differences.clone();
    for change in differences {
        match change.1 {
            mem::CallState::Nothing => {
                let mut pending = 0;
                let mut new = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone(){
                    total += 1;
                    if *state.1.call_list.get(&change.0).unwrap() == mem::CallState::PendingRemoval {
                        pending += 1;
                    }
                    else if *state.1.call_list.get(&change.0).unwrap() == mem::CallState::New {
                        new += 1;
                    }
                }
                if (pending + new) != total {
                    new_differences.remove(&change.0);
                }
            }
            mem::CallState::New => {
                // Do nothing, new button presses are always legit
            }
            mem::CallState::Confirmed => {
                let mut new = 0;
                let mut confirmed = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone(){
                    total += 1;
                    if *state.1.call_list.get(&change.0).unwrap() == mem::CallState::New {
                        new += 1;
                    }
                    else if *state.1.call_list.get(&change.0).unwrap() == mem::CallState::Confirmed {
                        confirmed += 1;
                    }
                }
                if (new + confirmed) != total {
                    new_differences.remove(&change.0);
                }
            }
            mem::CallState::PendingRemoval => {

                let mut others_agree = false;
                for state in state_list_with_changes.values() {
                    if *state.call_list.get(&change.0.clone()).unwrap() == change.1 {
                        others_agree = true;
                    }
                }

                if received_state.last_floor != change.0.floor || others_agree {
                    new_differences.remove(&change.0);
                }
            }
        }
    }

    return new_differences;
        
}

// Sanity check and state machine function. Only does something when a new state is received from another elevator
pub fn sanity_check_incomming_message(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::State>) -> () {
    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                // Getting old memory and extracting my own state
                memory_request_tx.send(mem::MemoryMessage::Request).unwrap_or(println!("Error in requesting memory"));
                let old_memory = memory_recieve_rx.recv().unwrap();
                let my_state = old_memory.state_list.get(&old_memory.my_id).unwrap().clone();

                // Getting new state from rx, extracting both old and new calls for comparison
                let received_state = rx.unwrap();
                let old_calls = old_memory.state_list.get(&received_state.id).unwrap().call_list.clone();
                let new_calls = received_state.call_list.clone();

                // Getting the difference between the old and new calls
                let mut differences = difference(old_calls.clone(), new_calls.clone());

                // Getting a new state list with the changes added
                let mut state_list_with_changes = old_memory.state_list.clone();
                state_list_with_changes.insert(received_state.id, received_state.clone());

                // Check whether the changed orders are valid or not
                differences = insanity(differences, received_state.clone(), state_list_with_changes);

                // Getting the relevant calls from my state
                let my_diff: HashMap<mem::Call, mem::CallState> = my_state.call_list.into_iter().filter(|x| differences.contains_key(&x.0)).collect();

                // Running the state machine on only the changed calls
                let my_diff_changed = cyclic_counter(my_diff.clone(), &state_list_with_changes);

                // Extracting the calls that were actually changed to minimize memory changing and avoid errors
                let changed_calls = difference(my_diff, my_diff_changed);

                // Sending the changes to memory one after the other
                for change in changed_calls {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).unwrap();
                }
            }
            // If we don't get a new state in decent time, this function runs
            default(Duration::from_millis(100)) => {
                // Getting old memory and extracting my own call list
                let old_memory = memory_recieve_rx.recv().unwrap();
                let my_call_list = old_memory.state_list.get(&old_memory.my_id).unwrap().clone().call_list;

                // Running the state machine on my own calls
                let new_call_list = cyclic_counter(my_call_list.clone(), &old_memory.state_list);

                // Extracting the calls that were actually changed to minimize memory changing and avoid errors
                let changed_calls = difference(my_call_list, new_call_list);

                // Sending the changes to memory one after the other
                for change in changed_calls {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).unwrap();
                }
            }
        }
    }
}