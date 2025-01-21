use std::{str, net::UdpSocket, thread::{self, sleep}, time::Duration};

const chosen_ip_sock: &str = "127.0.0.8:30000";

const lisnr_sleep_time: Duration = Duration::from_millis(100);



fn main() {
    println!("Hello, world!");

    let socket_lisner = UdpSocket::bind(chosen_ip_sock).unwrap();

    let joiner_lisner = thread::spawn(move || {
        println!("preparing lisner...");

        loop {
            let mut buf_lisner = [0;50];

            println!("Reading from thred");
            let (_number_of_bytes, _src_addr) = socket_lisner.recv_from(&mut buf_lisner).expect("Didn't receive data");

            let message = str::from_utf8(&buf_lisner).unwrap();
            println!("Messege contents: {}", message);
            sleep(lisnr_sleep_time);
        };
    });

    let _ = joiner_lisner.join();

}
