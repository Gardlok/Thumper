use thumper::{TheDJ, Beat};
use std::time::Duration;


// Mock test task 
async fn run_task(beat: Beat) {
    let delay_dur = Duration::from_secs(1);
    for _ in 1..=100 {
        let _ = beat.now();
        smol::Timer::after(delay_dur).await; 
    }
}

fn main() {

    // Init the dj
    let dj = TheDJ::init().unwrap();  

    // Init runtime that executes and monitors a mock task
    smol::block_on(async {
        
        // Init a task to monitor
        let d = Duration::from_secs(1);
        let b = dj.register("example".to_string(), d).unwrap();
        smol::spawn(async move{run_task(b).await}).detach();

        // Watch the monitor
        loop {
            println!("Status: {:?}", dj.get_record(0).unwrap().get_activity_rating());
            smol::Timer::after(Duration::from_secs(2)).await; 
        }

    });

}
