use std::net::{TcpStream, TcpListener};
use std::io::{Write, Read};

fn handle_stream(mut stream: TcpStream){
    // Create a buffer of 1024 bytes to hold data
    let mut buffer = [0; 1024];

    // Read data from the stream to the buffer
    stream.read(&mut buffer).expect("Couldn't read data from the stream...");

    // Print the data from the buffer in readable format
    let received_data = String::from_utf8_lossy(&buffer);
    println!("Data received: {}", received_data);

    // Write response to the stream in correct format
    let response = "Hello from the listener!".as_bytes();
    stream.write(response).expect("Couldn't write data to the stream...");

}

fn main() {
    // Bind the listener to the local address
    let listener = TcpListener::bind("127.0.0.1:80").expect("Couldn't bind to the address...");

    //let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    for stream in listener.incoming(){
        match stream{
            Ok(stream) => {
                std::thread::spawn(|| {handle_stream(stream);});
                }
            Err(e) => {
                eprintln!("Error: {}", e);
                }
    
//let stream = TcpStream::connect("127.0.0.1:8080").expect("Couldn't connect to the server...");
            }
        }
    }
