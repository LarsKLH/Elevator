This is a peer-2-peer system that uses UDP-broadcasting to transmit each elevators worldview. The code consists of several modules, all running as separate threads with crossbeam channels between them.

The code is structured around a central memory unit that all the other processes can query for readout or editing.

To communicate between the elevators we have a TX and a RX thread running in a loop. The TX routinely sends the current memory state out to the broadcast address for the others to read. The RX receives broadcasts from other elevators and sends their contents onward.

The input from the RX is then passed through a sanity check which updates the memory if the changes line up with the rules of our cyclic state machine. This is the backbone of our order structure, and consists of four states. No order, new order, confirmed order and pending removal. We have to make sure that the states only go one way, but if they do, no thread can make a change that invalidates a change from another thread. The rules for changing are that for an order to become confirmed or removed (set back to no order) all connected elevators have to agree. For an order to become new or pending, we only need our elevator or one other elevator to set this state.

As memory is updated, a "brain?" thread tries to figure out what to do next. It communicates it's orders to the elevator itself through the elevator output thread, which realizes the current movement-state from memory and turns on the appropriate lights.
