use std::{str, net::UdpSocket,thread::sleep, time::Duration};

const chosen_ip_sock: &str = "127.0.0.4:30005";

const broad_ip_sock: &str = "127.0.0.7:30005";

const snedr_sleep_time: Duration = Duration::from_millis(300);



fn main() {

    println!("Hello, world!");

    let socket_sender = UdpSocket::bind(chosen_ip_sock).unwrap();

    socket_sender.set_broadcast(true).unwrap();

    println!("Broadcast: {}", socket_sender.broadcast().unwrap());

    loop {
        let buf_sendr = "Message from sending program".as_bytes();

        println!("Sending from thred");
        socket_sender.send_to(buf_sendr, broad_ip_sock).expect("couldn't send data");

        let buf_sendr = "Sending a second message".as_bytes();
        socket_sender.send_to(buf_sendr, broad_ip_sock).expect("couldn't send data");

        sleep(snedr_sleep_time);
    }


}
