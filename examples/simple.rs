use beats::{TheDJ, Beat, Record, ActivityRating};
use smol::{io, net, prelude::*, Unblock};
use smol::Timer;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::time::{SystemTime, Duration};
use futures::stream::FuturesUnordered;


// Mock test task without seperate counter, which is more like a real scenario 
async fn run_task(beat: Beat) {
    let delay_dur = Duration::from_secs(1);
    for _ in 1..=100 {
        let _ = beat.send();
        smol::Timer::after(delay_dur).await; 
    }
}

fn main() {

    // Init the dj
    let dj = TheDJ::init().unwrap();  

    // Init runtime that performs and monitors a mock task
    smol::block_on(async {
        
        // Init a task to monitor
        let d = Duration::from_secs(1);
        let b = dj.register("example".to_string(), d).unwrap();
        smol::spawn(async move{run_task(b).await}).detach();

        // Watch the monitor
        loop {
            println!("Status: {:?}", dj.get_record(0).unwrap().get_activity_rating());
            //println!("avg {:?}", dj.get_record(0).unwrap().get_avg());
            smol::Timer::after(Duration::from_secs(2)).await; 
        }

    });

}
