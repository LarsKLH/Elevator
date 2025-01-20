use std::{str, net::UdpSocket, thread::{self, sleep}, time::{Duration}};


fn main() {
    println!("Hello, world!");

    let chosen_ip_sock: &str = "127.0.0.8:30005";

    let sendr_sleep_time = Duration::from_millis(300);
    let recvr_sleep_time = Duration::from_millis(100);

    let socket_base = UdpSocket::bind(chosen_ip_sock).unwrap();

    let socket_sender = socket_base.try_clone().unwrap();
    let socket_reader = socket_base.try_clone().unwrap(); 

    let join_sender = thread::spawn(move || {
        println!("preparing sender...");
        sleep(Duration::from_secs(4));
        loop {
            let buf_sendr = "Message from sending thread".as_bytes();

            println!("Sending from thred");
            socket_sender.send_to(buf_sendr, chosen_ip_sock).expect("couldn't send data");

            let buf_sendr = "Sending a second message".as_bytes();
            socket_sender.send_to(buf_sendr, chosen_ip_sock).expect("couldn't send data");

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
