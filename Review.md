Reviewer 1:
7
Redefinition of enums/types necessary? Using a custom Direction and
elevio::driver's objects, for example, often requires translating between types.
Often verbose, such as:
    memory.state_list.get_mut(&memory.my_id).unwrap().move_state = new_move_state;
Just to do something as trivial as setting the elevator’s move_state.
Generally, a lot of code. Does everything need to be so complex and
interconnected?  For example, updating one’s own state over a channel: Does each
attribute of the state need a separate update function, or could the State
object just be sent over the channel?  Do the local elevator details need to be
so tightly connected to the memory model? At its core, I don’t think the local
elevator’s position, obstruction status, cab calls, etc., need to go through the
central "memory" model. The memory model is good, but it should only be used
where it’s actually necessary—when multiple elevators need to agree on things.
Lots of comments. No need to comment on things that are obvious given function names.
get_differences() // This function gets the differences.
Stuff like that.
Also, a lot of TODOs and similar, but understandable given the work in progress.
Somewhat confusing naming in the memory-related parts, such as request_tx and
receive_tx. "Receive" and "TX (Transmit)"? Aren’t these contradictions?
Generally, a good main file. It starts and spawns all threads in a clear way,
gathering everything in one place.  Not entirely sure about the point of
.clone()ing all objects and sending them to each thread—are they connected in
some way, or are they completely independent objects?  Might be something
Rust-related that I don’t fully understand.
I think an array of hall/cab orders is a better fit than a list/hashmap for the
UDP broadcast approach.  It makes it easier to compare and find differences
between two State objects, rather than having these complex functions like the
ones you currently have.
Good module separation. The way things are grouped makes sense.


Reviewer 2:
8
- + for using Rust
- fix compiler warnings such as unused imports. I also suggest using cargo clippy which gives more warnings and suggestions for how to improve the code.
- Tip: use the clap package to to command line arugment parsing (clap) in a better more robust way. However, not necessary
- Tip: add flag for elevio port. Then you can start multiple simulators on one computer and test your code with multiple "elevators" without using multiple physical computers.
- Main function is quite clean. Just starting threads and making channels.
- I immediatly got a bad feeling when I saw that there was a memory module. The memory state is basically one big global variable. Remember go's (and rust's) slogan: "do not communicate by sharing memory instead share memory by communicating". Furthermore, as far as I can see all moduels with access to memory can make whatever request they want. Thus if there is a new developer working on the elevator_interface it might send a request to memory which it is not supposed to do. This may cause very hard to track down bugs, sunce every module is interacting with the memory module. I would say it is better to split up the content of the State in "Memory" module to different modules. Then elevator_interface would be the owner of some of the data, brain would be the owner of some other data and so on. If the brains data is updated and elevator_interface is dependent on this information, then brain is responsible for sending the data to elevator_interface once it is updated. Thus, you can get better seperation of your code and it may become clearer. I see the idea with the memory module, and it works, however I would say there are better ways of doing this.
- Why does sanity_check need to be a thread? Why not just have a function which takes in the data you want to check and it retrurns a Result or bool. Then you can just panic if the Result is Err.
- Network communication is good small and it is doing what you would expect.
- Brain module name is a bit vague. Is it the "brain" for the single elevator/state machine or the "brain" for the communication?
- Brain module is quite clear with good funciton names
- There is good module separation. Everything is not just run from the main function.
- Perhaps too many comments. If the function names and variable names are good then it is most of the time not necessary with comments on what happens on that line. Use comments when you are not able to write the code such that it can be easily understood by other developers.
- Summary: Good modules, however my main concern is that the memory module makes the code less understandable and increases the interconnections/dependencies between the modules.


Reviewer 3:
7
- the state type is not just a state but the complete state for the system. change the name to reflect this.
- function names seems ok. 
	- tip: if you are using linux you can run this command to print all functions ( starting with fn) and the previous line.
		- grep -Rin --color -B 1 --include="*.rs" "fn "
- good use of comments.
- the type Memory have redundant information. may create problems if they ever get out of sync.
- either have sain default comandline values or give a hint of which arguments you want.
- are you using a old serde_with package?
- i cant find any code setting hard limits on the elevator movements. 
	- the elevator should not be able to drive up/down at the top/bottom floor
	- would advice to place these hard stops as close as possible to the actual motor controller logic. reduces the chance of getting around the security.
- elevator_interface.rs ->elevator_inputs().
	- keep it as simple as possible. i would strongly advice to only have logic which routes or translates the message ONLY based on the message and the recipient. do not make it state dependent.  right now it is only for routing so keep it that way.
- "brain.rs" should indicate that it is responsible for the elevator. it has the FSM and gives out signal based on that.  "brain" is a ok name, but try to indicate that it only handles the "physical" lift.
- sanity.rs is more or less helper functions for memory.rs. you may want to implement them as object functions for the types in memory.rs.
- it seems like you are sending the whole database (Memory) over the internet, which seems like allot of data. make sure that you dont DDoS yourself. 
	- you may get problems if there are multiple messages in the buffer (recieve_buffer).
	- there is also a risk that messages get split into multiple packages.
# conclusion
code seems good/ok with good named functions and states. there are some small exeptions, but these can be fixed. given sanity.rs it seems you have a tendency to make the modules you need, even though it may increase the interface. what you have implemented seems sain, even though there are still things missing. The biggest risk I see is that you will get to little time to make a coherent design, even though you are capable.

Reviewer 4:
9
*readme file well describes how orders are meant to be handled, though other modules lack that details of information.
*In the main.rs communication between components is a bit unclear, TX and a RX thread running in a loop probably to-be-implemented.
*Naming for modules and variables is consistent and clear. Readme file makes it clear that the system uses p2p and describes how order assignement works.
*Some modules lack comments, and a some of the comments will be only understandable for the one who wrote them.
*Each module handles its task. Sanity checker deals with a lot of functionalities, but it's coherent and it all concerns the subject.
*There is a shared memory, but it is explained and the modifications makes sense.
*It's a bit confusing how memory module works and how other modules access it.
*using uwrap() is generally considered to be a bad pracrise for multiple error handling since it can panic and abort the program without specifying error properly. expect() handles error messages, but using match could be more effective and flexible way to handle them.
*As a gut feeling I'd give this code 7 or 8 out of 10. It is not finished, but lacking sections are idetified and commented for the future improvement.

