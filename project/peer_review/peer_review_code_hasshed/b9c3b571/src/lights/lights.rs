use driver_rust::elevio;

/*
fn main(){
let (floor_tx,floor_rx) = cbc::unbounded();

//thread to poll the floor sensor:

let elev_for_sensor = elevator.clone();
thread::spawn(move || {
    floor_sensor(elev_for_sensor,floor_tx,Duration::from_millis(100));
});

let mut current_floor: Option<u8> = None;

    // Main control loop: listen for floor sensor events.
    for floor in floor_rx.iter() {
        // If the elevator was previously at a different floor,
        // assume it has now left that floor.
        if let Some(prev_floor) = current_floor {
            if prev_floor != floor {
                // Turn off the door light (or other lights) at the previous floor.
                // (You might also want to turn off call button lights, etc.)
                elevator.door_light(false);
                println!("Leaving floor {}", prev_floor);
            }
        }

        // When a new floor is reached, update the floor indicator,
        // turn on the door light, etc.
        elevator.floor_indicator(floor);
        elevator.door_light(true);
        println!("Arrived at floor {}", floor);

        // Save the current floor.
        current_floor = Some(floor);
    }
}
    */