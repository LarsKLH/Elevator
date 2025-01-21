use std::net::{TcpStream, TcpListener};
use std::io::{Write, Read};
use std::thread::sleep;

fn handle_stream(mut stream: TcpStream){
    loop{
    // Create a buffer of 1024 bytes to hold data
    let mut buffer = [0; 1024];

    // Read data from the stream to the buffer
    stream.read(&mut buffer).expect("Couldn't read data from the stream...");

    // Print the data from the buffer in readable format
    let received_data = String::from_utf8_lossy(&buffer);
    println!("Data received: {}", received_data);
    sleep(std::time::Duration::from_millis(1000));

    // Write response to the stream in correct format
    //let response = "Hello from the listener!".as_bytes();
    //stream.write(response).expect("Couldn't write data to the stream...");
}

}

fn main() {
    // Bind the listener to the local address
    let listener = TcpListener::bind("10.100.23.204:33546").expect("Couldn't bind to the address..."); //34933
    let mut sender = TcpStream::connect("10.100.23.204:33546").expect("Couldn't connect to the server...");
    
    
    std::thread::spawn(move || {
        for i in 0 .. 10 {
            let message = format!("Message number: {}\n", i);
            sender.write(message.as_bytes()).expect("Couldn't write data to the listener...");
            println!("[Stream] sending: {}", message);
        }
    });

    //let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    
    for stream in listener.incoming(){
        match stream{
            Ok(stream) => {
                std::thread::spawn(move ||
                    {handle_stream(stream);});
                }
            Err(e) => {
                eprintln!("Error: {}", e);
                }
    
//let stream = TcpStream::connect("127.0.0.1:8080").expect("Couldn't connect to the server...");
            }
        }
    }
