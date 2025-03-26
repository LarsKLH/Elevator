
use std::collections::HashMap;
use std::hash::Hash;
use std::net::Ipv4Addr;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;
use driver_rust::elevio::elev;
use std::time::{Duration, SystemTime};

use crate::memory::{self as mem, Call};
use crate::elevator_interface as elevint;

use log;

// Basics of our cyclic counter:
// - It only goes one way, from Nothing to new to confirmed to pendingremoval and then back around
// - To go from nothing to new or from confirmed to pendingremoval only one elevator needs to be in the previous state, ie. we do not need the others to agree
// - To go from new to confirmed or from pendingremoval to nothing we need all the elevators to agree

// There also needs to be some way of dealing with elevators reconnecting with different states, but this is not implemented yet

// Iterates the cyclic counter correctly
fn cyclic_counter(state_to_change: HashMap<Call, mem::CallState>, state_list: &HashMap<Ipv4Addr, mem::State>) -> HashMap<Call, mem::CallState> {
    
    let mut changed_state = state_to_change.clone();
    
    for call in &state_to_change {
        match call.1 {
            mem::CallState::Nothing => {
                // If one of the others has a new order that passed sanity check, change our state to new
                for state in state_list {
                    if *state.1.call_list.get(call.0).expect("Incorrect call state found") == mem::CallState::New {
                        changed_state.insert(call.0.clone(), mem::CallState::New);
                        println!("Sanity: Want to update cyclic counter for call {:?} from Nothing to New", call.0);
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
                    if *state.1.call_list.get(call.0).expect("Incorrect call state found") == mem::CallState::New {
                        new += 1;
                    }
                    else if *state.1.call_list.get(call.0).expect("Incorrect call state found") == mem::CallState::Confirmed {
                        confirmed += 1;
                    }
                }
                if (new + confirmed) == total {
                    changed_state.insert(call.0.clone(), mem::CallState::Confirmed);
                    println!("Sanity: Want to update cyclic counter for call {:?} from New to Confirmed", call.0);
                }
            }

            mem::CallState::Confirmed => {
                // If one of the others has removed an order that passed sanity check, change our state to new
                for state in state_list {
                    if *state.1.call_list.get(call.0).expect("Incorrect call state found") == mem::CallState::PendingRemoval {
                        changed_state.insert(call.0.clone(), mem::CallState::PendingRemoval);
                        println!("Sanity: Want to update cyclic counter for call {:?} from Confirmed to PendingRemoval", call.0);
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
                    if *state.1.call_list.get(call.0).expect("Incorrect call state found") == mem::CallState::PendingRemoval {
                        pending += 1;
                    }
                    else if *state.1.call_list.get(call.0).expect("Incorrect call state found") == mem::CallState::Nothing {
                        nothing += 1;
                    }
                }
                if (pending + nothing) == total {
                    changed_state.insert(call.0.clone(), mem::CallState::Nothing);
                    println!("Sanity: Want to update cyclic counter for call {:?} from PendingRemoval to Nothing", call.0);
                }
            }
        }
    }
    return changed_state;
}

// Gets the difference between two call lists
fn difference(old_calls: HashMap<mem::Call, mem::CallState>, new_calls: HashMap<Call, mem::CallState>) -> HashMap<Call, mem::CallState> {
    let mut difference: HashMap<Call, mem::CallState> = HashMap::new();
    for call in old_calls.clone() {
        if new_calls.get(&call.0) != old_calls.get(&call.0) {
            difference.insert(call.0, *new_calls.get(&call.0).expect("Incorrect call state found"));
        }
    }
    return difference;
}

// Checks whether the changes follow the rules for the cyclic counter
fn filter_changes(differences: HashMap<mem::Call, mem::CallState>, received_state: mem::State, state_list_with_changes: HashMap<Ipv4Addr, mem::State>) -> HashMap<mem::Call, mem::CallState> {
    let mut new_differences = differences.clone();
    for change in differences {
        match change.1 {
            mem::CallState::Nothing => {
                // If the others don't agree, then we cannot update the order to none

                let mut pending = 0;
                let mut new = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone(){
                    total += 1;
                    if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::PendingRemoval {
                        pending += 1;
                    }
                    else if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::New {
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
                // If the others don't agree, then we cannot update the order to confirmed

                let mut new = 0;
                let mut confirmed = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone(){
                    total += 1;
                    if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::New {
                        new += 1;
                    }
                    else if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::Confirmed {
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
                    if *state.call_list.get(&change.0.clone()).expect("Incorrect call state found") == change.1 {
                        others_agree = true;
                        break;
                    }
                }

                // If the others don't agree or we aren't on the correct floor, we cannot accept the changes
                if received_state.last_floor != change.0.floor || !others_agree {
                    new_differences.remove(&change.0);
                }
            }
        }
    }

    return new_differences;
        
}

// Does as it says on the tin, handles hall calls. Returns hall calls for other elevator
fn handle_hall_calls(old_memory: mem::Memory, received_state: mem::State, memory_request_tx: Sender<mem::MemoryMessage>, state_list_with_changes: HashMap<Ipv4Addr, mem::State>) -> HashMap<mem::Call, mem::CallState> {
     
     // Dealing with hall calls from other elevator

     // Getting new and old calls
     let old_calls: HashMap<mem::Call, mem::CallState> = old_memory.state_list.get(&received_state.id).expect("Incorrect state found").call_list
     .clone()
     .into_iter()
     .filter(|x| x.0.call_type == mem::CallType::Hall(elevint::Direction::Down) || x.0.call_type == mem::CallType::Hall(elevint::Direction::Up))
     .collect();

     let new_calls: HashMap<mem::Call, mem::CallState> = received_state.call_list
     .clone()
     .into_iter()
     .filter(|x| x.0.call_type == mem::CallType::Hall(elevint::Direction::Down) || x.0.call_type == mem::CallType::Hall(elevint::Direction::Up))
     .collect();

     // Getting the difference between the old and new calls to get what calls have changed since last time
     let mut differences = difference(old_calls.clone(), new_calls.clone());

     // Check whether the changed orders are valid or not
     differences = filter_changes(differences, received_state.clone(), state_list_with_changes.clone());


     // Changing our hall calls based on the changes to the received state

     // Getting the relevant calls from my state
     let my_diff: HashMap<mem::Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Invalid state list received").call_list.clone().into_iter().filter(|x| differences.contains_key(&x.0)).collect();

     // Running the state machine on only the changed calls
     let my_diff_changed = cyclic_counter(my_diff.clone(), &state_list_with_changes);

     // Extracting the calls that were actually changed to minimize memory changing and avoid errors
     let changed_calls = difference(my_diff, my_diff_changed);

     // Sending the changes to memory one after the other
     for change in changed_calls {
         memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Error sending memory request");
     }

     // Returning the hall call changes for the other elevator so it can be included in a state update later
     return differences;
}

//  Dealing with the cab calls for the other elevator
// This means filtering out the changes that make no sense
fn handle_cab_calls_for_other(old_memory: mem::Memory, received_memory: mem::Memory, memory_request_tx: Sender<mem::MemoryMessage>) -> HashMap<mem::Call, mem::CallState> {
    
    // Checking for cab calls concerning the other elevator
    let old_cab_calls: HashMap<mem::Call, mem::CallState> = old_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").call_list
    .clone()
    .into_iter()
    .filter(|x| x.0.call_type == mem::CallType::Cab)
    .collect();
    let new_cab_calls: HashMap<mem::Call, mem::CallState> = received_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").call_list
    .clone()
    .into_iter()
    .filter(|x| x.0.call_type == mem::CallType::Cab)
    .collect();

    // Getting the difference between the old and new cab calls to get what calls have changed since last time
    let mut others_differences_cab = difference(old_cab_calls.clone(), new_cab_calls.clone());

    // Getting a state list with only cab calls from the other elevator
    let mut others_states_for_comparison: HashMap<Ipv4Addr, mem::State> = HashMap::new();
    others_states_for_comparison.insert(received_memory.my_id, received_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").clone());
    others_states_for_comparison.insert(0.into(), old_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").clone());

    // Check whether the changed cab calls are valid or not
    others_differences_cab = filter_changes(others_differences_cab, received_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").clone(), others_states_for_comparison.clone());

    // Returning the cab call changes for the other elevator so it can be included in a state update later
    return others_differences_cab;
}

// Dealing with the cab calls for our elevator
// This means changing the state of our elevators based on the rules of the cyclic counter
fn handle_cab_calls_for_me(old_memory: mem::Memory, received_memory: mem::Memory, memory_request_tx: Sender<mem::MemoryMessage>) -> () {

    // Checking for cab calls concerning our elevator
    let my_old_cab_calls: HashMap<mem::Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").call_list
    .clone()
    .into_iter()
    .filter(|x| x.0.call_type == mem::CallType::Cab)
    .collect();
    let my_new_cab_calls: HashMap<mem::Call, mem::CallState> = received_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").call_list
    .clone()
    .into_iter()
    .filter(|x| x.0.call_type == mem::CallType::Cab)
    .collect();

    // Getting only the cab calls that have changed to minimize overwriting of memory
    let my_differences_cab = difference(my_old_cab_calls.clone(), my_new_cab_calls.clone());

    let mut my_states_for_comparison: HashMap<Ipv4Addr, mem::State> = HashMap::new();
    my_states_for_comparison.insert(old_memory.my_id, old_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").clone());
    my_states_for_comparison.insert(0.into(), received_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").clone());

    let my_differences_cab_changed = cyclic_counter(my_differences_cab, &my_states_for_comparison);

    // Sending the changes to memory one after the other
    for change in my_differences_cab_changed {
        memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Error sending memory message");
    }
}

// This function merges two call lists, always accepting the one with the "highest" callstate
fn merge_calls(old_calls: HashMap<Call, mem::CallState>, new_calls: HashMap<Call, mem::CallState>) -> HashMap<Call, mem::CallState> {
    let mut merged_calls = old_calls.clone();
    for call in new_calls {
        if old_calls.contains_key(&call.0) {
            match call.1 {
                mem::CallState::Nothing => {
                    let old_call = old_calls.get(&call.0).expect("Incorrect call found in merging").clone();
                    match old_call {
                        mem::CallState::Nothing => {
                            merged_calls.insert(call.0, call.1);
                        }
                        _ => {
                            merged_calls.insert(call.0, old_call);
                        }
                    }
                }
                mem::CallState::New => {
                    let old_call = old_calls.get(&call.0).expect("Incorrect call found in merging").clone();
                    match old_call {
                        mem::CallState::Nothing | mem::CallState::New => {
                            merged_calls.insert(call.0, call.1);
                        }
                        _ => {
                            merged_calls.insert(call.0, old_call);
                        }
                    }
                }
                mem::CallState::Confirmed => {
                    let old_call = old_calls.get(&call.0).expect("Incorrect call found in merging").clone();
                    match old_call {
                        mem::CallState::PendingRemoval => {
                            merged_calls.insert(call.0, old_call);
                        }
                        _ => {
                            merged_calls.insert(call.0, call.1);
                        }
                    }
                }
                mem::CallState::PendingRemoval => {
                    merged_calls.insert(call.0, call.1);
                }
            }
        }
    }
    return merged_calls;
}

fn timeout_check(last_received: HashMap<Ipv4Addr, SystemTime>, memory_request_tx: Sender<mem::MemoryMessage>) -> () {

    // If we have no response from an elevator for a long time, we should not care about it's opinion anymore
    for elevator in last_received {
        if elevator.1.elapsed().expect("Invalid time found") > Duration::from_secs(3) {
            memory_request_tx.send(mem::MemoryMessage::DeclareDead(elevator.0)).expect("Cannot declare elevator dead");
        }
    }
}

fn testing_function() -> bool {
    let mut memory_before = mem::Memory::new(0.into(), 4);
    memory_before.state_list.get_mut(&memory_before.my_id).expect("Incorrect state found").call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 1 }, mem::CallState::New);
    memory_before.state_list.get_mut(&memory_before.my_id).expect("Incorrect state found").call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 2 }, mem::CallState::New);


    return true;
}

// Sanity check and state machine function. Only does something when a new state is received from another elevator
pub fn sanity_check_incomming_message(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::Memory>) -> () {
    // Setting up a hashmap to keep track of the last time a message was received from each elevator
    let mut last_received: HashMap<Ipv4Addr, SystemTime> = HashMap::new();

    println!("Sanity: Starting sanity check");
    println!("Sanity: Testing function returned: {}", testing_function());

    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                // Getting old memory
                let old_memory = mem::Memory::get(memory_request_tx.clone(), memory_recieve_rx.clone());

                // Getting new state from rx, extracting both old and new calls for comparison
                let received_memory = rx.expect("Invalid memory found");
                let received_state = received_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").clone();

                if received_memory.my_id == old_memory.my_id {
                    // Do same as default if we get our own state back

                    timeout_check(last_received.clone(), memory_request_tx.clone());

                    // Getting old memory and extracting my own call list
                    let old_memory = mem::Memory::get(memory_request_tx.clone(), memory_recieve_rx.clone());
                    let my_call_list = old_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").clone().call_list;

                    // Running the state machine on my own calls
                    let new_call_list = cyclic_counter(my_call_list.clone(), &old_memory.state_list);

                    // Extracting the calls that were actually changed to minimize memory changing and avoid errors
                    let changed_calls = difference(my_call_list, new_call_list);

                    // No need to print that there is nothing to change 
                    if !changed_calls.is_empty() {
                        println!("Sanity: Changed calls: {:?}", changed_calls);
                        // Sending the changes to memory one after the other
                    }
                    
                    for change in changed_calls {
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Could not update memory");
                    }
                }
                else if !old_memory.state_list.contains_key(&received_memory.my_id) {

                    // Sending the data for the new elevator to memory
                    memory_request_tx.send(mem::MemoryMessage::UpdateOthersState(received_state.clone())).expect("Could not update memory");

                    // Setting last received for this elevator to the current time
                    last_received.insert(received_state.id.clone(), SystemTime::now());
                }
                else if old_memory.state_list.get(&received_memory.my_id).expect("Incorrect state found").timed_out {
                    
                    let my_new_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").call_list.clone().into_iter().filter(|x| x.0.call_type == mem::CallType::Hall(elevint::Direction::Up) || x.0.call_type == mem::CallType::Hall(elevint::Direction::Down)).collect();
                    let my_old_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").call_list.clone().into_iter().filter(|x| x.0.call_type == mem::CallType::Hall(elevint::Direction::Up) || x.0.call_type == mem::CallType::Hall(elevint::Direction::Down)).collect();

                    // Merging the calls from the old and new state
                    let my_merged_calls = merge_calls(my_old_calls.clone(), my_new_calls.clone());

                    let my_modified_calls = difference(my_old_calls, my_merged_calls);

                    for change in my_modified_calls {
                        memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Could not update memory");
                    }

                    let others_new_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").call_list.clone().into_iter().filter(|x| x.0.call_type == mem::CallType::Hall(elevint::Direction::Up) || x.0.call_type == mem::CallType::Hall(elevint::Direction::Down)).collect();
                    let others_old_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").call_list.clone().into_iter().filter(|x| x.0.call_type == mem::CallType::Hall(elevint::Direction::Up) || x.0.call_type == mem::CallType::Hall(elevint::Direction::Down)).collect();

                    // Merging the calls from the old and new state
                    let others_merged_calls = merge_calls(others_old_calls.clone(), others_new_calls.clone());

                    let others_modified_calls = difference(others_old_calls, others_merged_calls);

                    let mut others_state_with_only_accepted = old_memory.state_list.get(&received_state.id).expect("Incorrect state found").clone();
                    for change in others_modified_calls.clone() {
                        others_state_with_only_accepted.call_list.insert(change.0, change.1);
                    }

                    // Sending the data for the updated elevator to memory
                    memory_request_tx.send(mem::MemoryMessage::UpdateOthersState(others_state_with_only_accepted.clone())).expect("Could not update memory");

                    // Setting last received for this elevator to the current time
                    last_received.insert(received_state.id.clone(), SystemTime::now());
                }
                else {

                    // Getting a new state list with the changes added
                    let mut state_list_with_changes: HashMap<Ipv4Addr, mem::State> = old_memory.state_list.clone().into_iter().filter(|x| x.1.timed_out == false).collect();
                    state_list_with_changes.insert(received_state.id, received_state.clone());

                    // Dealing with the new hall calls
                    let differences_in_hall = handle_hall_calls(old_memory.clone(), received_state.clone(), memory_request_tx.clone(), state_list_with_changes.clone());

                    // Dealing with the new cab calls
                    let differences_in_cab = handle_cab_calls_for_other(old_memory.clone(), received_memory.clone(), memory_request_tx.clone());
                    handle_cab_calls_for_me(old_memory.clone(), received_memory.clone(), memory_request_tx.clone());

                    
                    // Summing up all accepted changes and commiting to memory
                    let mut received_state_with_only_accepted = old_memory.state_list.get(&received_state.id).expect("Incorrect state found").clone();
                    for change in differences_in_hall.clone() {
                        received_state_with_only_accepted.call_list.insert(change.0, change.1);
                    }
                    for change in differences_in_cab.clone() {
                        received_state_with_only_accepted.call_list.insert(change.0, change.1);
                    }

                    // Getting differences initially
                    let differences_initially = difference(old_memory.state_list.get(&received_state.id).expect("Incorrect state found").call_list.clone(), state_list_with_changes.get(&received_state.id).expect("Incorrect state found").call_list.clone());

                    if differences_initially.len() > 0 {
                        let differences_after = difference(old_memory.state_list.get(&received_state.id).expect("Incorrect state found").call_list.clone(), received_state_with_only_accepted.call_list.clone());

                        // If less than half of the changes aren't accepted we do not accept the changes
                        // This ensures out of sync elevators will eventually be considered timed out and merged
                        if differences_after.len() > differences_initially.len()/2 {
                            // Setting last received for this elevator to the current time
                            last_received.insert(received_state.id.clone(), SystemTime::now());
                        }
                    }
                    else {
                        // Setting last received for this elevator to the current time
                        last_received.insert(received_state.id.clone(), SystemTime::now());
                    }

                    // Sending the new state to memory
                    memory_request_tx.send(mem::MemoryMessage::UpdateOthersState(received_state_with_only_accepted)).expect("Could not update memory");
                }
            }

            // If we don't get a new state within 100 ms
            default(Duration::from_millis(100)) => {
                timeout_check(last_received.clone(), memory_request_tx.clone());

                // Getting old memory and extracting my own call list
                let old_memory = mem::Memory::get(memory_request_tx.clone(), memory_recieve_rx.clone());
                let my_call_list = old_memory.state_list.get(&old_memory.my_id).expect("Incorrect state found").clone().call_list;

                // Running the state machine on my own calls
                let new_call_list = cyclic_counter(my_call_list.clone(), &old_memory.state_list);

                // Extracting the calls that were actually changed to minimize memory changing and avoid errors
                let changed_calls = difference(my_call_list, new_call_list);

                // No need to print that there is nothing to change 
                if !changed_calls.is_empty() {
                    println!("Sanity: Changed calls: {:?}", changed_calls);
                    // Sending the changes to memory one after the other
                }
                
                for change in changed_calls {
                    memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Could not update memory");
                }
            }
        }
    }
}