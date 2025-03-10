// finite-state-machine for one elevator
//use sfsm::*;
use driver_rust::elevio;
use driver_rust::elevio::elev::*;
//use driver_rust::elevio::elev as e;
//use driver_rust::{elevio::elev::{Elevator, DIRN_DOWN}, Elevator::elev};
use crossbeam_channel::{self as cbc, select, Sender,Receiver};
use std::io::Error;
use std::thread;
use std::time::{self, Duration};
use crate::orderqueue::orderqueue::{CabOrder, ElevatorQueue, FloorOrder};
use std::net::UdpSocket;
use std::net::TcpStream;
use network_rust::udpnet;

use std::sync::{Arc, Mutex};


#[derive(Debug)]
enum State {
    Init,
    SetTargetFloor,
    Moving { target_floor: u8 },
    DoorOpen,
}


pub struct CurrentOrder {
    target_floor: u8,
    direction: u8, // 0 = up, 1 = down, 2 Stop
    stopover: bool,
}
impl CurrentOrder {
    pub fn new() -> Self {
        Self {
            target_floor: 0,
            direction: 2,
            stopover: false,
        }
    }
}
pub struct CustomDataType{
    message: u32,
    iteration:u8,
}


pub struct ElevFsm {    
    elevator: Elevator,
    call_button_rx: Receiver<elevio::poll::CallButton>,
    floor_sensor_rx: Receiver<u8>,
    stop_button_rx: Receiver<bool>,
    obstruction_rx: Receiver<bool>,
    orders_rx: Receiver<u8>,
    fsm_return_tx: Sender<String>,
    state: State,
    current_order: CurrentOrder,
    queue: ElevatorQueue
}

impl ElevFsm {
    // constructor for FSM, creates FSM with elevator-object, and channels 
    
    pub fn new(
        elevator: Elevator,
        call_button_rx: Receiver<elevio::poll::CallButton>,
        floor_sensor_rx: Receiver<u8>,
        stop_button_rx: Receiver<bool>,
        obstruction_rx: Receiver<bool>,
        orders_rx: Receiver<u8>,
        fsm_return_tx: Sender<String>,
        current_order: CurrentOrder,
    ) -> Self {
        Self {
            elevator,
            call_button_rx,
            floor_sensor_rx,
            stop_button_rx,
            obstruction_rx,
            orders_rx,
            fsm_return_tx,
            state: State::Init,
            current_order: CurrentOrder::new(),
            queue: ElevatorQueue::new(),
        }
    }

    pub fn run(&mut self) {
        //let socket = UdpSocket::bind("127.0.0.1:34254").expect("couldnt bind woho");
        // let stream = TcpStream::connect("127.0.0.1:34254").unwrap().local_addr().unwrap().ip();
        let mut current_floor = self.elevator.floor_sensor().unwrap_or(0);
        let mut obstruction: bool = false;
   

        loop {
            // Listen to messages on all channels
            cbc::select! {
                recv(self.call_button_rx) -> msg => {
                    let order = msg.unwrap();
                    match order.call {
                        0 | 1 => {
                            self.queue.add_order(order.floor, order.call);
                            self.elevator.call_button_light(order.floor, order.call, true);
                        }
                        2 => {
                            self.queue.add_cab_order(order.floor);
                            self.elevator.call_button_light(order.floor, order.call, true);
                        }
                        _ => {
                            println!("Received unknown calltype");
                        }
                    }
                    println!("{:#?}", order);
                    //println!("Number of orders: {}", self.queue.floor_orders.len());
                },
                recv(self.floor_sensor_rx) -> a => {
                    let floor = a.unwrap();
                    println!("Floorsensor: reached Floor: {:#?}", floor);
                    current_floor = floor;
                    let mut buf = [0,current_floor,0,0];
                    //socket.send_to(&buf,"0.0.0.0:20021").expect("coulndt send data");
                    // udpnet::bcast::tx().expect("bcast  udp");
                },
                recv(self.stop_button_rx) -> msg => {
                    if let Ok(true) = msg {
                        self.fsm_emergency();
                        println!("Emergancy!");
                        return;
                    }
                },
                recv(self.obstruction_rx) -> msg => {
                    if let Ok(true) = msg {
                        println!("obstruction detected, elevator stopped!");
                        self.elevator.motor_direction(DIRN_STOP);
                        obstruction = true;
                    }
                    else {
                        obstruction = false;
                    }
                },
                default(Duration::from_millis(100)) => {
                    //println!("No Message received on cbc, continuing with Loop");
                }
            }
        self.state_action(current_floor, obstruction);

        }

}

fn state_action(&mut self, current_floor: u8, obstruction: bool) {
    //println!("State-Action fn executed, State: {:#?}", self.state);
    match self.state {
        State::Init => {
            self.lights_init(4);
            loop {
                if let Some(floor) = self.elevator.floor_sensor() {   
                    println!("FSM-Init: reached floor: {}", floor);
                    self.elevator.motor_direction(DIRN_STOP);
                    self.state = State::SetTargetFloor;
                    
                
                    break;
                } else {
                    self.elevator.motor_direction(DIRN_DOWN);
                    //println!("FSM-Init: moving down to next floor");
                }
            }
        }

        State::SetTargetFloor => {

            self.queue.print_cab_orders();
            self.queue.print_orders();  
            //self.elevator.call_button_light(floor_order.floor,floor_order.direction,false);          
            // Set target floor, priorise caborders, afterwards progress with Moving-State
            if let Some(cab_order) = self.queue.cab_orders.first() {
                self.current_order.target_floor = cab_order.floor;
                
                self.state = State::Moving {
                    target_floor: cab_order.floor,
                };
                println!("Delivering Cab-Order: Set target-floor to: {}", cab_order.floor);
                
            } else if let Some(floor_order) = self.queue.floor_orders.first() {
                //turn on the floor-light. TO-DO: guarantee light is turned on only after order is shared with all nodes.
                self.elevator.call_button_light(floor_order.floor, floor_order.direction, true);
                self.current_order.target_floor = floor_order.floor;
                self.state = State::Moving {
                    target_floor: floor_order.floor,
                };
                println!("Delivering Floor-Order: Set target-floor to: {}", floor_order.floor);   
                             
            }

        }
        State::Moving { target_floor } => {
            // set elevator direction
            //println!("State: Moving with targetfloor {}", target_floor);
            //println!("StateMoving: current floor: {}", current_floor);
            //self.elevator.door_light(false);
            self.elevator.floor_indicator(current_floor);
            self.elevator.call_button_light(target_floor,CAB,true);
            if current_floor < target_floor {
                self.elevator.motor_direction(DIRN_UP);
                self.current_order.direction = 0;
                //self.elevator.call_button_light(current_floor,DIRN_UP,false);
            } else if current_floor > target_floor {
                self.elevator.motor_direction(DIRN_DOWN);
                self.current_order.direction = 1;
                self.elevator.call_button_light(current_floor,CAB,false);
                //self.elevator.call_button_light(current_floor,DIRN_DOWN,false);
            } else {
                self.elevator.motor_direction(DIRN_STOP);
                self.current_order.stopover = false;
                self.state = State::DoorOpen; 
            }

            // check for orders on the way with same direction
            for floor_order in self.queue.floor_orders.iter() {
                if floor_order.floor == current_floor && floor_order.direction == self.current_order.direction {
                    self.elevator.motor_direction(DIRN_STOP);
                    self.current_order.stopover = true;
                    self.elevator.door_light(true);
                    self.state = State::DoorOpen;
                    break; //return
                }
            }
        }

        State::DoorOpen => {

            self.elevator.door_light(true);
            self.elevator.call_button_light(current_floor,HALL_UP,false);
            self.elevator.call_button_light(current_floor,HALL_DOWN,false);
            thread::sleep(Duration::from_secs(2));

            if obstruction == false {

                self.elevator.door_light(false);
                self.elevator.call_button_light(current_floor,CAB,false);
                self.queue.remove_order(current_floor);
            
                if self.current_order.stopover == true {
                    self.state = State::Moving {
                        target_floor: self.current_order.target_floor,
                    };
                } else {
                    self.state = State::SetTargetFloor;
                }

            } else {
                println!("Obstruction active, elevator stopped");
                self.elevator.motor_direction(DIRN_STOP);
                self.elevator.door_light(true);
            }

        }
    }
}
fn lights_init(&mut self,floor:u8){
   for floor in 1..self.elevator.num_floors {
        self.elevator.call_button_light(floor,CAB,false);
        self.elevator.call_button_light(floor,HALL_DOWN,false);
        self.elevator.call_button_light(floor,HALL_UP,false);
    println!("Turned off lights on floor {}" , floor);
   }
}

fn fsm_emergency(&mut self) {
    self.elevator.motor_direction(DIRN_STOP);
}

/*pub fn fsm_init(&mut self) {

    loop {
        if let Some(floor) = self.elevator.floor_sensor() {
            println!("FSM-Init: reached floor: {}", floor);
            self.elevator.motor_direction(DIRN_STOP);
            self.state = State::SetTargetFloor;
            break;
        } else {
            self.elevator.motor_direction(DIRN_DOWN);
            println!("FSM-Init: moving down to next floor");
        }
    }

}
*/
}






