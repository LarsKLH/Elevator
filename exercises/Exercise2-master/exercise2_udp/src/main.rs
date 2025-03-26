use std::{str, net::UdpSocket, thread::{self, sleep}, time::Duration};


fn main() {
    println!("Hello, world!");

    let list_sock_addr: &str = "127.0.0.1:20005";

    let send_sock_addr: &str = "127.0.0.2:20005";

    let broadcast_sock_addr: &str = "255.255.255.255:20005";

    let sendr_sleep_time = Duration::from_millis(300);
    let recvr_sleep_time = Duration::from_millis(100);

    let socket_reader = UdpSocket::bind(list_sock_addr).unwrap();

    let socket_sender =  UdpSocket::bind(send_sock_addr).unwrap();
    
    socket_reader.set_broadcast(true).unwrap();
    socket_sender.set_broadcast(true).unwrap();


    let join_sender = thread::spawn(move || {
        println!("preparing sender...");
        sleep(Duration::from_secs(4));
        loop {
            let buf_sendr = "Message from sending thread".as_bytes();

            println!("Sending from thred");
            socket_sender.send_to(buf_sendr, broadcast_sock_addr).expect("couldn't send data");

            let buf_sendr = "Sending a second message".as_bytes();
            socket_sender.send_to(buf_sendr, broadcast_sock_addr).expect("couldn't send data");

            sleep(sendr_sleep_time);
        };
    });

    let join_reader = thread::spawn(move || {
        println!("preparing recr...");
        loop {
            let mut buf_reader = [0;50];

            println!("Reading from thred");
            let (_number_of_bytes, _src_addr) = socket_reader.recv_from(&mut buf_reader).expect("Didn't receive data");

            let message = str::from_utf8(&buf_reader).unwrap();
            println!("Messege contents: {}", message);
            sleep(recvr_sleep_time);
            
        };
    });

    join_sender.join().unwrap();
    join_reader.join().unwrap();
}

