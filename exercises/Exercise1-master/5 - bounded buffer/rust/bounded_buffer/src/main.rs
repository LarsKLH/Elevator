use std::{sync::mpsc, thread, thread::sleep, time::{self, Duration}};



fn main() {

    let  prod_sleep_time: Duration = time::Duration::from_millis(400);
    let con_sleep_time: Duration = time::Duration::from_millis(600);

    let (tx, rx) = mpsc::sync_channel(5);

    let producer = thread::spawn(move || {
        for i in 0 .. 10 {
            sleep(prod_sleep_time);
            println!("[producer] sending: {}", i);
            tx.send(i).unwrap();
        }
    });

    let consumer = thread::spawn(move ||{
        loop {
            sleep(con_sleep_time);
            let i = rx.recv();

            match i {
                Ok(i) => println!("[consumer] recieving: {}", i),
                Err(_) => break,
            }
            
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();




    println!("Prog end clean!");
}