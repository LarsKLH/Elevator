
use std::collections::HashMap;
use std::net::Ipv4Addr;

use crossbeam_channel::{Receiver, Sender};
use crossbeam_channel as cbc;
use std::time::{Duration, SystemTime};

use crate::memory::{self as mem, Call};
use crate::elevator_interface as elevint;


use itertools::Itertools;

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
        if new_calls.get(&call.0) != old_calls.get(&call.0) && new_calls.get(&call.0).is_some() {
            difference.insert(call.0, *new_calls.get(&call.0).expect("Incorrect call state found"));
        }
    }
    return difference;
}

// Checks whether the changes follow the rules for the cyclic counter
fn filter_changes(differences: HashMap<mem::Call, mem::CallState>, received_last_floor: u8, state_list_with_changes: HashMap<Ipv4Addr, mem::State>) -> HashMap<mem::Call, mem::CallState> {
    let mut new_differences = HashMap::new();
    for change in differences {
        match change.1 {
            mem::CallState::Nothing => {
                // If the others don't agree, then we cannot update the order to none

                let mut pending = 0;
                let mut none = 0;
                let mut new = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone(){
                    total += 1;
                    if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::PendingRemoval {
                        pending += 1;
                    }
                    else if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::Nothing {
                        none += 1;
                    }
                    else if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::New {
                        new += 1;
                    }
                }
                if (pending + none + new) == total {
                    new_differences.insert(change.0, change.1);
                }
            }
            mem::CallState::New => {
                // Always add, new button presses are always legit
                new_differences.insert(change.0, change.1);
            }
            mem::CallState::Confirmed => {
                // If the others don't agree, then we cannot update the order to confirmed

                let mut new = 0;
                let mut confirmed = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone(){
                    total += 1;
                    if state.1.call_list.get(&change.0).expect("Incorrect call state found").clone() == mem::CallState::New {
                        new += 1;
                    }
                    else if state.1.call_list.get(&change.0).expect("Incorrect call state found").clone() == mem::CallState::Confirmed {
                        confirmed += 1;
                    }
                }
                if (new + confirmed) == total {
                    new_differences.insert(change.0, change.1);
                }
            }
            mem::CallState::PendingRemoval => {

                let mut other_was_first = false;
                let mut others_set_correct = false;

                let mut confirmed = 0;
                let mut pending = 0;
                let mut total = 0;
                for state in state_list_with_changes.clone() {
                    total += 1;
                    if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::Confirmed {
                        confirmed += 1;
                    }
                    else if *state.1.call_list.get(&change.0).expect("Incorrect call state found") == mem::CallState::PendingRemoval {
                        pending += 1;
                        other_was_first = true;
                    }
                }
                if (pending + confirmed) == total {
                    others_set_correct = true;
                }

                // If the others don't agree or we aren't on the correct floor, we cannot accept the changes
                if (received_last_floor == change.0.floor || other_was_first) && others_set_correct {
                    new_differences.insert(change.0, change.1);
                }
            }
        }
    }

    return new_differences;
        
}

// This function merges two call lists, always accepting the one with the "highest" callstate
fn merge_calls(old_calls: HashMap<Call, mem::CallState>, new_calls: HashMap<Call, mem::CallState>) -> HashMap<Call, mem::CallState> {
    let mut merged_calls = HashMap::new();
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



fn deal_with_calls_for_me(received_memory: mem::Memory, old_memory: mem::Memory, memory_request_tx: Sender<mem::MemoryMessage>) -> () {
    let mut cab_calls = HashMap::new();
    let mut hall_calls = HashMap::new();
    for call in old_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone() {
        if call.0.call_type == mem::CallType::Cab {
            cab_calls.insert(call.0, call.1);
        }
        else {
            hall_calls.insert(call.0, call.1);
        }
    }

    // Getting the old and received interpretations of our cab calls
    let mut cab_calls_for_comparison = HashMap::new();
    let mut cab_calls_cycled = cab_calls.clone();
    if received_memory.my_id != old_memory.my_id && received_memory.state_list.contains_key(&old_memory.my_id) {
        cab_calls_for_comparison.insert(received_memory.my_id,received_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone());
        //cab_calls_for_comparison.insert(old_memory.my_id,old_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone());
        cab_calls_cycled = cyclic_counter(cab_calls.clone(), &cab_calls_for_comparison.clone());
    }
    else if received_memory.my_id == old_memory.my_id && received_memory.state_list.clone().into_iter().filter(|state| !state.1.timed_out).count() == 1 {
        cab_calls_cycled = cyclic_counter(cab_calls.clone(), &received_memory.state_list.clone()
        .into_iter().filter(|state| !state.1.timed_out).collect());
    }
        
    
    

    let cab_calls_difference = difference(cab_calls.clone(), cab_calls_cycled.clone());

    let mut hall_calls_for_comparison = old_memory.state_list.clone();
    hall_calls_for_comparison.insert(received_memory.my_id,received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone());
    let hall_calls_cycled = cyclic_counter(hall_calls.clone(), &hall_calls_for_comparison.clone());

    let hall_calls_difference = difference(hall_calls.clone(), hall_calls_cycled.clone());

    let mut calls_difference_assembled = hall_calls_difference.clone();
    calls_difference_assembled.extend(cab_calls_difference.clone());

    for change in calls_difference_assembled {
        memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Sanity: Could not send call update");
        println!("Sanity: Sent call update for {:?}", change.0);
        println!("Sanity: New call state: {:?}", change.1);
    }
}

fn deal_with_calls_for_other(received_memory: mem::Memory, old_memory: mem::Memory, memory_request_tx: Sender<mem::MemoryMessage>) -> HashMap<Call, mem::CallState> {
    let mut cab_calls = HashMap::new();
    let mut hall_calls = HashMap::new();
    for call in received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone() {
        if call.0.call_type == mem::CallType::Cab {
            cab_calls.insert(call.0, call.1);
        }
        else {
            hall_calls.insert(call.0, call.1);
        }
    }

    /* for cab_call in cab_calls.clone() {
        println!("Sanity: Cab call: {:?} {:?}", cab_call.0, cab_call.1);
    }

    for hall_call in hall_calls.clone() {
        println!("Sanity: Hall call: {:?} {:?}", hall_call.0, hall_call.1);
    } */

    let mut cab_calls_difference = cab_calls.clone();

    if received_memory.my_id != old_memory.my_id {
        // Getting the old and received interpretations of our cab calls
        let mut cab_calls_for_comparison = HashMap::new();
        cab_calls_for_comparison.insert(old_memory.my_id,old_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone());
        //cab_calls_for_comparison.insert(received_memory.my_id,received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone());
        let cab_calls_filtered = filter_changes(cab_calls.clone(), received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone().last_floor, cab_calls_for_comparison.clone());
        //println!("Sanity: Cab calls filtered: {:?}", cab_calls_filtered.clone());

        let cab_calls_to_remove = difference(cab_calls.clone(), cab_calls_filtered.clone());
        for cab_call in cab_calls_to_remove {
            cab_calls_difference.remove(&cab_call.0);
        }
    }

    let mut hall_calls_for_comparison = old_memory.state_list.clone();
    //hall_calls_for_comparison.insert(received_memory.my_id,received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone());
    hall_calls_for_comparison.remove(&received_memory.my_id);
    let hall_calls_filtered = filter_changes(hall_calls.clone(), received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone().last_floor, hall_calls_for_comparison.clone());

    let mut hall_calls_difference = hall_calls.clone();
    let hall_calls_to_remove = difference(hall_calls.clone(), hall_calls_filtered.clone());
    for hall_call in hall_calls_to_remove {
        hall_calls_difference.remove(&hall_call.0);
    }

    let mut calls_difference_assembled = hall_calls_difference.clone();
    calls_difference_assembled.extend(cab_calls_difference.clone());

    let mut received_state_to_commit = received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone();

    let change_not_accepted = difference(calls_difference_assembled.clone(), received_state_to_commit.call_list.clone());
    for change in change_not_accepted.clone() {
        received_state_to_commit.call_list.insert(change.0, old_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.get(&change.0).expect("Sanity: Wrong call, cannot deal with it").clone());
    }

    if calls_difference_assembled.is_empty() {
        received_state_to_commit.timed_out = old_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").timed_out;
    }

    memory_request_tx.send(mem::MemoryMessage::UpdateOthersState(received_state_to_commit.clone())).expect("Sanity: Could not send state update");

    return received_state_to_commit.call_list;
}

fn did_i_deal_with_it(received_memory: mem::Memory, old_memory: mem::Memory, accepted_changes: HashMap<Call, mem::CallState>) -> bool {
    let mut did_i_deal_with_it = true;

    let accepted_hall_changes: HashMap<Call, mem::CallState> = accepted_changes.clone()
    .into_iter().filter(|x| x.0.call_type != mem::CallType::Cab).collect();
    let accepted_cab_changes: HashMap<Call, mem::CallState> = accepted_changes.clone()
    .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect();

    let received_hall_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
    .into_iter().filter(|x| x.0.call_type != mem::CallType::Cab).collect();
    let old_hall_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
    .into_iter().filter(|x| x.0.call_type != mem::CallType::Cab).collect();

    let hall_changes_to_do = difference(old_hall_calls.clone(), received_hall_calls.clone());
    let hall_changes_done = difference(old_hall_calls.clone(), accepted_hall_changes.clone());

    if hall_changes_to_do != hall_changes_done {
        did_i_deal_with_it = false;
    }

    let old_cab_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
    .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect(); 
    let received_cab_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
    .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect();

    let _cab_changes_to_do = difference(old_cab_calls.clone(), received_cab_calls.clone());
    let _cab_changes_done = difference(old_cab_calls.clone(), accepted_cab_changes.clone());

    if hall_changes_to_do != hall_changes_done {
        did_i_deal_with_it = false;
    }

    return did_i_deal_with_it;
}

fn merge_my_and_others_calls(mut received_memory: mem::Memory, old_memory: mem::Memory, memory_request_tx: Sender<mem::MemoryMessage>) -> () {

    let old_hall_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
    .into_iter().filter(|x| x.0.call_type != mem::CallType::Cab).collect();
    let new_hall_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
    .into_iter().filter(|x| x.0.call_type != mem::CallType::Cab).collect();

    let merged_hall_calls = merge_calls(old_hall_calls.clone(), new_hall_calls.clone());
    let merged_hall_difference = difference(old_hall_calls.clone(), merged_hall_calls.clone());

    let mut my_merged_calls_difference = merged_hall_difference.clone();
    if received_memory.state_list.contains_key(&old_memory.my_id) {
        let my_old_cab_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
        .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect();
        let my_new_cab_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
        .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect();

        let my_cab_calls = merge_calls(my_old_cab_calls.clone(), my_new_cab_calls.clone());
    
    
        for change in my_cab_calls {
            my_merged_calls_difference.insert(change.0, change.1);
        }
    }

    for change in my_merged_calls_difference {
        memory_request_tx.send(mem::MemoryMessage::UpdateOwnCall(change.0, change.1)).expect("Sanity: Could not send call update");
        
    }

    let mut their_merged_calls_difference = merged_hall_difference.clone();
    
    if old_memory.state_list.contains_key(&received_memory.my_id) {
        let their_old_cab_calls: HashMap<Call, mem::CallState> = old_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
        .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect();
        let their_new_cab_calls: HashMap<Call, mem::CallState> = received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.clone()
        .into_iter().filter(|x| x.0.call_type == mem::CallType::Cab).collect();

        let their_cab_calls = merge_calls(their_old_cab_calls.clone(), their_new_cab_calls.clone());
    
    
        for change in their_cab_calls {
            their_merged_calls_difference.insert(change.0, change.1);
        }
    }

    for change in their_merged_calls_difference {
        received_memory.state_list.get_mut(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.insert(change.0, change.1);
    }

    received_memory.state_list.get_mut(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").timed_out = false;

    memory_request_tx.send(mem::MemoryMessage::UpdateOthersState(received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone())).expect("Sanity: Could not send state update");
}

fn deal_with_received_orders(mut received_memory: mem::Memory, mut old_memory: mem::Memory, memory_request_tx: Sender<mem::MemoryMessage>) -> bool {
    let mut _dealt_with = false;

    if !old_memory.state_list.contains_key(&received_memory.my_id) {
        println!("Sanity: Received memory from new elevator");
        if received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it") != old_memory.state_list.get(&old_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it") {
            merge_my_and_others_calls(received_memory.clone(), old_memory.clone(), memory_request_tx.clone());
            println!("Sanity: Differences detected, merging calls");
        }
        memory_request_tx.send(mem::MemoryMessage::UpdateOthersState(received_memory.state_list.get(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").clone())).expect("Sanity: Could not send state update");
        _dealt_with = true;
    }
    else if old_memory.state_list.get(&received_memory.my_id).expect("Sanity: wrong state already").timed_out
    || if received_memory.state_list.contains_key(&old_memory.my_id) { received_memory.state_list.get(&old_memory.my_id).expect("Sanity: wrong state received").timed_out } else { false } {
        println!("Sanity: Received memory from timed out elevator");
        merge_my_and_others_calls(received_memory.clone(), old_memory.clone(), memory_request_tx.clone());
        
        _dealt_with = true;
    }
    else {
        // This one was a bit to much
        // println!("Sanity: Received memory from elevator that isn't timed out");
        old_memory.state_list = old_memory.state_list.clone().into_iter().filter(|x| !x.1.timed_out).collect();
        let accepted_changes = deal_with_calls_for_other(received_memory.clone(), old_memory.clone(), memory_request_tx.clone());
        received_memory.state_list.get_mut(&received_memory.my_id).expect("Sanity: Wrong in state, cannot deal with it").call_list.extend(accepted_changes.clone());
        deal_with_calls_for_me(received_memory.clone(), old_memory.clone(), memory_request_tx.clone());
        _dealt_with = did_i_deal_with_it(received_memory.clone(), old_memory.clone(), accepted_changes.clone());
    }
    
    return _dealt_with;
}

// Sanity check and state machine function. Only does something when a new state is received from another elevator
pub fn sanity_check_incomming_message(memory_request_tx: Sender<mem::MemoryMessage>, memory_recieve_rx: Receiver<mem::Memory>, rx_get: Receiver<mem::Memory>) -> () {
    // Setting up a hashmap to keep track of the last time a message was received from each elevator
    let mut last_received: HashMap<Ipv4Addr, SystemTime> = HashMap::new();

    println!("Sanity: Starting sanity check");
    println!("Sanity: Testing function returned: {}", testing_function());

    println!("Sanity: Done with Initialization");

    loop {
        cbc::select! {
            recv(rx_get) -> rx => {
                // Getting old memory
                let mut old_memory = mem::Memory::get(memory_request_tx.clone(), memory_recieve_rx.clone());
                old_memory.state_list.get_mut(&old_memory.my_id).expect("Sanity: Wrong state in default").timed_out = false;

                // Getting new state from rx, extracting both old and new calls for comparison
                match rx {
                    Ok(rx) => {
                        let received_memory = rx;
                        let _dealt_with = deal_with_received_orders(received_memory.clone(), old_memory.clone(), memory_request_tx.clone());

                        if _dealt_with {
                            last_received.insert(received_memory.my_id, SystemTime::now());
                        }
                        else {
                            println!("Sanity: Received memory was not dealt with");
                        }
                    }
                    Err(e) => {
                        println!("Sanity: Error receiving memory: {:?}", e);
                    }
                }

                
                timeout_check(last_received.clone(), memory_request_tx.clone());
                
            }

            // If we don't get a new state within 100 ms
            default(Duration::from_millis(1000)) => {
                println!("Sanity: Default case");

                // Getting old memory
                let mut old_memory = mem::Memory::get(memory_request_tx.clone(), memory_recieve_rx.clone());
                old_memory.state_list.get_mut(&old_memory.my_id).expect("Sanity: Wrong state in default").timed_out = false;
                old_memory.state_list = old_memory.state_list.clone().into_iter().filter(|x| !x.1.timed_out).collect();

                deal_with_calls_for_me(old_memory.clone(), old_memory.clone(), memory_request_tx.clone());
                timeout_check(last_received.clone(), memory_request_tx.clone());

                
            }
        }
    }
}




// this function is run once at startup to check if we havent made any braking changed to our code
// I know rust has the ability to have tests on compilation but still

fn testing_function() -> bool {
    let mut memory = mem::Memory::new(Ipv4Addr::new(0, 0, 0, 0), 8);

    let mut state1 = mem::State::new(Ipv4Addr::new(0, 0, 0, 1), 8);

    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 0 }, mem::CallState::Nothing);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 1 }, mem::CallState::Nothing);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 2 }, mem::CallState::Nothing);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 3 }, mem::CallState::Nothing);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 4 }, mem::CallState::New);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 5 }, mem::CallState::New);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 6 }, mem::CallState::New);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 7 }, mem::CallState::New);

    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 0 }, mem::CallState::Confirmed);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 1 }, mem::CallState::Confirmed);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 2 }, mem::CallState::Confirmed);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 3 }, mem::CallState::Confirmed);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 4 }, mem::CallState::PendingRemoval);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 5 }, mem::CallState::PendingRemoval);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 6 }, mem::CallState::PendingRemoval);
    state1.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 7 }, mem::CallState::PendingRemoval);


    let mut state2 = mem::State::new(Ipv4Addr::new(0, 0, 0, 0), 8);

    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 0 }, mem::CallState::Nothing);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 1 }, mem::CallState::New);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 2 }, mem::CallState::Confirmed);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 3 }, mem::CallState::PendingRemoval);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 4 }, mem::CallState::Nothing);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 5 }, mem::CallState::New);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 6 }, mem::CallState::Confirmed);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 7 }, mem::CallState::PendingRemoval);

    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 0 }, mem::CallState::Nothing);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 1 }, mem::CallState::New);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 2 }, mem::CallState::Confirmed);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 3 }, mem::CallState::PendingRemoval);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 4 }, mem::CallState::Nothing);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 5 }, mem::CallState::New);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 6 }, mem::CallState::Confirmed);
    state2.call_list.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 7 }, mem::CallState::PendingRemoval);


    memory.state_list.insert(state2.id.clone(), state2.clone());
    let test_calls = cyclic_counter(state1.call_list.clone(), &memory.state_list.clone());

    memory.state_list.insert(state2.id.clone(), state1.clone());

    let differences = difference(memory.state_list.get(&memory.my_id).expect("Sanity: Wrong state in test").call_list.clone(), state2.call_list.clone());

    println!("Sanity: Unfiltered differences: {:?}", differences.clone().iter().sorted());

    let mut differences_inserted = memory.state_list.get(&memory.my_id).expect("Sanity: Wrong state in test").call_list.clone();
    differences_inserted.extend(differences.clone());

    let test_filter_calls = filter_changes(differences.clone(), 3, memory.state_list.clone());

    println!("Sanity: Filtered calls: {:?}", test_filter_calls.clone().iter().sorted());

    memory.state_list.get_mut(&state2.id).expect("Sanity: Wrong state in test").call_list.extend(test_filter_calls.clone());

    for call in state1.call_list.clone().iter().sorted() {
        println!("Direction: {:?} Floor: {:?} - Other state: {:?}, Attempted: {:?}, State after {:?}", call.0.call_type, call.0.floor, call.1, differences_inserted.get(call.0).expect("Sanity: Wrong call in test"), memory.state_list.get(&state2.id).expect("Sanity: Wrong state in test").call_list.get(call.0).expect("Sanity: Wrong call in test"));
    }

    let expected_state = mem::State::new(Ipv4Addr::new(0, 0, 0, 0), 8);
    let mut expected_calls = expected_state.call_list.clone();

    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 0 }, mem::CallState::Nothing);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 1 }, mem::CallState::New);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 2 }, mem::CallState::Nothing);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 3 }, mem::CallState::Nothing);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 4 }, mem::CallState::New);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 5 }, mem::CallState::Confirmed);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 6 }, mem::CallState::Confirmed);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Up), floor: 7 }, mem::CallState::New);

    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 0 }, mem::CallState::Confirmed);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 1 }, mem::CallState::Confirmed);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 2 }, mem::CallState::Confirmed);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 3 }, mem::CallState::PendingRemoval);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 4 }, mem::CallState::Nothing);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 5 }, mem::CallState::PendingRemoval);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 6 }, mem::CallState::PendingRemoval);
    expected_calls.insert(Call { call_type: mem::CallType::Hall(elevint::Direction::Down), floor: 7 }, mem::CallState::Nothing);

    let wrong_answers = difference(test_calls, expected_calls);

    println!("Sanity: Wrong calls:");
    for mistake in wrong_answers.clone() {
        println!("Sanity: {:?} {:?}", mistake.0, mistake.1);
    }


    if wrong_answers.is_empty() {
        println!("Sanity: All good!");
        return true;
    } else {
        println!("Sanity: Something went wrong!");
        return false;
    }
}