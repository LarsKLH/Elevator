use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;

use crate::modules::common::*;

use std::sync::Arc;
use crossbeam_channel as cbc; //for message passing
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::{ Instant};
use std::process::{Command, Stdio};
use tokio::time::{sleep, Duration};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::sync::mpsc::{Sender,Receiver};
use driver_rust::elevio::elev as e;
const MAX_FLOORS: usize = 4; //IMPORT FROM MAIN
// All peers supposed to have:
// list of elevator states
// list of orders --> needs more states such as new, in process, finished

// honestly the only reason we transfer cab orders globally is to use executable
// otherwise they are managed locally since other elevators are not modifying or
// taking over them, if elev dies, cab orders die too... 
// TODO: maybe we need backup? maybe not




pub struct Decision {
    //LOCAL
    local_id: String,
    local_broadcastmessage: Arc<RwLock<BroadcastMessage>>, // everything locally sent as heartbeat
    dead_elev: Arc<Mutex<std::collections::HashMap<String, bool>>>,
    //NETWORK CBC
    network_elev_info_tx: Mutex<mpsc::Sender<BroadcastMessage>>, 
    network_elev_info_rx: Mutex<mpsc::Receiver<BroadcastMessage>>,
    network_alivedead_rx: Mutex<mpsc::Receiver<AliveDeadInfo>>,
    //OTEHRS/UNSURE
    new_elev_state_rx: Mutex<mpsc::Receiver<ElevatorState>>, //state to modify
    order_completed_rx: Mutex<mpsc::Receiver<u8>>, //elevator floor
    new_order_rx: Mutex<mpsc::Receiver<Order>>, //should be mapped to cab or hall orders (has id, call, floor), needs DIR
    elevator_assigned_orders_tx: mpsc::Sender<Order>, //one order only actually, s is typo
}

impl Decision {
    pub fn new(
        local_id: String,

        network_elev_info_tx: Sender<BroadcastMessage>,
        network_elev_info_rx: Receiver<BroadcastMessage>,
        network_alivedead_rx: Receiver<AliveDeadInfo>,

        new_elev_state_rx: Receiver<ElevatorState>,
        order_completed_rx: Receiver<u8>,
        new_order_rx: Receiver<Order>,
        elevator_assigned_orders_tx: mpsc::Sender<Order>,
    ) -> Self {
        Decision {
            local_id,
            local_broadcastmessage: Arc::new(RwLock::new(BroadcastMessage::new(0))), //TODO: when empty?
            dead_elev: Arc::new(Mutex::new(std::collections::HashMap::new())), // wrap in Mutex

            network_elev_info_tx: Mutex::new(network_elev_info_tx),
            network_elev_info_rx: Mutex::new(network_elev_info_rx),
            network_alivedead_rx: Mutex::new(network_alivedead_rx),

            new_elev_state_rx: Mutex::new(new_elev_state_rx),
            order_completed_rx: Mutex::new(order_completed_rx),
            new_order_rx: Mutex::new(new_order_rx),
            elevator_assigned_orders_tx,
        }
    }


    /*
    // BARRIER NOTE: for barrier to be approved we need to check
    // which elevators are alive (local field: dead_elev) and then if all
    // ALIVE elevators have attached ID in order's barrier, then we move
    // however, we still jump to confirmed without barrier (kinda obvious)
    pub async fn step(& self) { 
        let memory = memory_recieve_rx.recv().expect("Error receiving memory");

        let alive_elevators: Vec<String> = memory.state_list
            .iter()
            .filter(|(_, state)| !state.timed_out)
            .map(|(id, _)| id.to_string())
            .collect();

        //let mut broadcast_msg = self.local_broadcastmessage.write().await;
        let mut status_changed = false; //flag

        for (_elev_id, state_list) in &memory.state_list {
            for state in state_list.iter_mut() {
                if state.status == OrderStatus::Requested && alive_elevators.is_subset(&order.barrier) { // Get callstates from memory for each elevator
                    state.status = OrderStatus::Confirmed;                                                  // and check if all alive elevators have the order
                    state.barrier.clear();
                    status_changed = true;
                }
            }
        }
        if status_changed {
            self.hall_order_assigner();
        }

        //check if we can move from finished to NoOrder, clean barrier
        for (_elev_id, orders) in &mut broadcast_msg.orders {
            for order in orders.iter_mut() {
                if order.status == OrderStatus::Completed && alive_elevators.is_subset(&order.barrier) {
                    order.status = OrderStatus::Noorder;
                    order.barrier.clear();
                }
            }
        }

    }
*/
    pub async fn hall_order_assigner(& self) { //check if mut is needed here
        //1. map broadcast Message to Elevator system struct
        //take even dead elevators? and then reassign orders
        //status assigned stays but elevators take possibly diff orders
        let memory = memory_recieve_rx.recv().expect("Error receiving memory");

        let mut broadcast = self.local_broadcastmessage.write().await;
        
        let mut hall_requests = vec![vec![false, false]; MAX_FLOORS];
        let mut states = std::collections::HashMap::new();

        // Get callstates from memory for each elevator
        for orders in broadcast.orders.values() {
            for order in orders {
                if order.status == OrderStatus::Confirmed && order.call < 2 {
                    hall_requests[(order.floor - 1) as usize][order.call as usize] = true;
                }
            }
        }

        // Fetch id and state_list from memory
        for (id, state) in &memory {
            let dead_elevators = m; // something decided by timeout

            // Skip dead elevators
            if let Some(true) = dead_elevators.get(id) {
                continue;
            }
        
            // Get cab calls
            let cab_requests: Vec<bool> = (1..=MAX_FLOORS) 
            .map(|floor| {
                broadcast.orders.values().any(|orders| {
                    orders.iter().any(|order| {
                        order.floor as usize == floor && order.call == 2 && order.status == OrderStatus::Confirmed
                    })
                })
            })
            .collect();

            // Get elevator state (MovementState)
            let behaviour = if state.door_open {
                "doorOpen"
            } else if state.current_direction != e::DIRN_STOP { 
                "moving"
            } else {
                "idle"
            };
        
            // insert states(the values) into the hashmap based on id(the key), states is to be changed to state_list
            states.insert(id.clone(), serde_json::json!({
                "behaviour": behaviour,
                "floor": state.current_floor,
                "direction": match state.current_direction {
                    e::DIRN_DOWN => "down",
                    e::DIRN_UP => "up",
                    _ => "stop",
                },
                "cabRequests": cab_requests
            }));
        }

        // Create a json variable for optimal hall order assignment
        let input_json = serde_json::json!({
            "hallRequests": hall_requests,
            "states": states
        }).to_string();
        
        println!("{}", serde_json::to_string_pretty(&input_json).unwrap());

        // Execute hall_request_assigner for optimal hall order assignment
        let hra_output = Command::new("./hall_request_assigner")
        .arg("--input")
        .arg(&input_json)
        .output()
        .expect("Failed to execute hall_request_assigner");

        let hra_output_str : String;
        let mut new_orders: HashMap<String, Vec<Order>> = HashMap::new();

        // If successful, print the output of hall_request_assigner
        if hra_output.status.success() {
            let hra_output_str = String::from_utf8(hra_output.stdout)
                .expect("Invalid UTF-8 hra_output");
            
            let hra_output: HashMap<String, Vec<Vec<bool>>> = serde_json::from_str(&hra_output_str)
                .expect("Failed to deserialize hra_output");
        
            for (elev_id, floors) in &hra_output {
                println!("Elevator ID: {}, Floors: {:?}", elev_id, floors);
            }

            // Change local orders based on the output of hall_request_assigner
            for (new_elevator_id, orders) in hra_output.iter() {
                for (floor_index, buttons) in orders.iter().enumerate() {
                    let floor = (floor_index + 1) as u8; 
                    for (call_type, &is_confirmed) in buttons.iter().enumerate() { //call type can only be either 0 or 1 (up, down)
                        if is_confirmed { //true e. i. there is an order
                            let call = call_type as u8; 
    
                            let mut found_order: Option<Order> = None;
                            let mut previous_elevator_id: Option<String> = None;
    
                            for (elevator_id, orders) in broadcast.orders.iter_mut() {
                                if let Some(order) = orders.iter_mut().find(|order| order.floor == floor && order.call == call) {
                                    found_order = Some(order.clone());
                                    previous_elevator_id = Some(elevator_id.clone());
                                    break;
                                }
                            }
            
                            if let Some(order) = found_order {
                                if let Some(prev_id) = previous_elevator_id {
                                    if let Some(prev_orders) = broadcast.orders.get_mut(&prev_id) {
                                        if let Some(pos) = prev_orders.iter().position(|x| x == &order) {
                                            prev_orders.remove(pos);
                                        }
                                    }
                                }
            
                                new_orders.entry(new_elevator_id.clone())
                                    .or_default()
                                    .push(order);
                            }
                        }
                    }
                }
            }
        }
        
        for (elevator_id, orders) in new_orders {
            for order in orders {
                broadcast.orders.entry(elevator_id.clone()).or_default().push(order);
            }
        }

        //4. send order one by one to FSM
        for (_elevator_id, orders) in &broadcast.orders {
            for order in orders.iter() {
                if order.status == OrderStatus::Confirmed {
                    if let Err(e) = self.elevator_assigned_orders_tx.send(order.clone()).await {
                        eprintln!("Failed to send confirmed order: {}", e);
                    }
                }
            }
        }

    }
}

/* 
fn hall_order_assignment() -> () {
let input_json = serde_json::json!({
    "hallRequests": hall_requests,
    "states": states
}).to_string();

println!("{}", serde_json::to_string_pretty(&input_json).unwrap());

//2. use hall order assigner
let input_json = serde_json::to_string_pretty(&elevator_system).expect("Failed to serialize");
let hra_output = Command::new("./hall_request_assigner")
.arg("--input")
.arg(&input_json)
.output()
.expect("Failed to execute hall_request_assigner");

if hra_output.status.success() {
    hra_output_str = String::from_utf8(hra_output.stdout).expect("Invalid UTF-8 hra_output");
    let hra_output = serde_json::from_str::<HashMap<String, Vec<Vec<bool>>>>(&hra_output_str)
        .expect("Failed to deserialize hra_output");
    //return decision::global_to_local(hra_output); //need to transofrm from vec vec bool to order
} else {
    hra_output_str = "Error: Execution failed".to_string();
    //return;
}
}*/

