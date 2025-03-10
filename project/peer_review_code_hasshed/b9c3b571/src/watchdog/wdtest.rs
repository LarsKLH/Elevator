mod watchdog;

use tokio::time::Duration;
use watchdog::watchdog::{Watchdog, ResetHandle};


#[tokio::main]
async fn main() {
    // Erstelle einen Watchdog mit einem 5-Sekunden-Timeout
    let watchdog = Watchdog::new(Duration::from_secs(5));
    let reset_handle = watchdog.get_reset_handle();

    // Task, die den Timer alle 2 Sekunden zurücksetzt
    tokio::spawn(async move {
        for _ in 0..3 {
            tokio::time::sleep(Duration::from_secs(4)).await;
            reset_handle.reset();
            println!("Watchdog wurde zurückgesetzt!");
        }
        println!("Reset task stopped!");
    });

    // Warten, bis der Timer ausläuft
    watchdog.await_timeout().await;

    // Diese Nachricht wird nur ausgegeben, wenn der Timer nicht rechtzeitig zurückgesetzt wurde
    println!("Watchdog expired!");
    return;
}