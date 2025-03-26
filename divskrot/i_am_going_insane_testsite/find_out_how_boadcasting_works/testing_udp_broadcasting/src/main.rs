
use std::{net::{Ipv4Addr, SocketAddrV4, UdpSocket}, str::FromStr, thread::{self, sleep, spawn}, time::Duration};



fn main() {
    println!("Hello, world!");

    let port_for_broadcaasting: u16 = 2626;

    let muticast_adress = Ipv4Addr::from([224,26,26,26]);

    let sending_addr = Ipv4Addr::from_str("172.26.26.1").expect("Not valid ip");
    let list_addr = Ipv4Addr::from_str("172.26.26.2").expect("not valid iip");

    // let sending_socket = UdpSocket::bind((, port_for_broadcaasting)).expect("NetWork: Failed to bind to send socket");
    // let list_socket = UdpSocket::bind(("172.26.26.2", port_for_broadcaasting)).expect("NetWork: Failed to bind to list socket");


    //sending_socket.set_broadcast(true).expect("Cpould noe sert broadcast");

    let join_sender = spawn(move || {
        println!("preparing sender...");

        let sending_socket = UdpSocket::bind((sending_addr, port_for_broadcaasting)).expect("could not bid to socket");

        sending_socket.set_broadcast(true).expect("could not set to broadcast");

        sending_socket.join_multicast_v4(&muticast_adress, &Ipv4Addr::new(0, 0, 0, 0)).expect("coud not joun multicast");
        sending_socket.set_multicast_loop_v4(true).unwrap();

        sending_socket.connect((muticast_adress,port_for_broadcaasting)).expect("could not connnect to multicast addr"); 
        
        sleep(Duration::from_secs(2));
        loop {
            let buf_sendr = "Message from sending thread".as_bytes();

            println!("Sending from thred");
            sending_socket.send(buf_sendr).expect("couldn't send data");

            let buf_sendr = "Sending a second message".as_bytes();
            sending_socket.send(buf_sendr).expect("couldn't send data");

            sleep(Duration::from_millis(200));
        };
    });

    let join_lsitner = thread::spawn(move || {
        println!("preparing recr...");
        let list_socket = UdpSocket::bind((list_addr, port_for_broadcaasting)).expect("could not bid to socket");
        list_socket.set_broadcast(true).expect("could not set to broadcast");

        list_socket.join_multicast_v4(&muticast_adress, &Ipv4Addr::new(0, 0, 0, 0)).expect("could not joiin multcast");
        list_socket.set_multicast_loop_v4(true).unwrap();
        
        loop {
            let mut buf_reader = [0;50];

            println!("Reading from thred");
            let (_number_of_bytes, _src_addr) = list_socket.recv_from(&mut buf_reader).expect("Didn't receive data");

            let message = std::str::from_utf8(&buf_reader).unwrap();
            println!("Recieved {} bytes from {}, Messege contents: {}",_number_of_bytes, _src_addr, message);
            sleep(Duration::from_millis(100));
            
        };
    });

    join_sender.join().unwrap();
    join_lsitner.join().unwrap();
}

