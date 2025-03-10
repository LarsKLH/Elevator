#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorOrder {
    pub floor: u8,
    pub direction: u8, // 0 = up, 1 = down
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CabOrder {
    pub floor: u8,
}

pub struct ElevatorQueue {
    pub floor_orders: Vec<FloorOrder>,
    pub cab_orders: Vec<CabOrder>,
}

impl ElevatorQueue {
    /// constructor for new empty order que
    pub fn new() -> Self {
        Self {
            floor_orders: Vec::new(),
            cab_orders: Vec::new(),
        }
    }

    /// adds new order, if order does not exist already
    pub fn add_order(&mut self, floor: u8, direction: u8) {
        let new_order = FloorOrder { floor, direction };
        if !self.floor_orders.contains(&new_order) {
            self.floor_orders.push(new_order);
        }
    }

    /// removes floororders and caborders with same floor
    pub fn remove_order(&mut self, floor: u8) {
        self.floor_orders.retain(|order| order.floor != floor);
        self.cab_orders.retain(|order| order.floor != floor);
    }

    /// gives reference to an order
    pub fn get_order(&self, index: usize) -> Option<&FloorOrder> {
        self.floor_orders.get(index)
    }

    /// print all orders (for debuging purpose)
    pub fn print_orders(&self) {
        for order in &self.floor_orders {
            let dir = if order.direction == 0 { "Up" } else { "Down" };
            println!("Floororders: Floor: {}, Direction: {}", order.floor, dir);
        }
    }

    // ---------------------- CabOrder-Methods ----------------------

    // Add new caborder, if order doesnt exist
    pub fn add_cab_order(&mut self, floor: u8) {
        let new_order = CabOrder { floor };
        if !self.cab_orders.contains(&new_order) {
            self.cab_orders.push(new_order);
        }
    }

    pub fn get_cab_order(&self, index: usize) -> Option<&CabOrder> {
        self.cab_orders.get(index)
    }

    /// Print cab order to console (for debugging)
    pub fn print_cab_orders(&self) {
        for order in &self.cab_orders {
            println!("Cab Order - Floor: {}", order.floor);
        }
    }
}
