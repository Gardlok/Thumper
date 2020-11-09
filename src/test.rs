use beats::{TheDJ, Beat, Record, ActivityRating};
use smol::{io, net, prelude::*, Unblock};
use smol::Timer;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::time::{SystemTime, Duration};
use futures::stream::FuturesUnordered;


// Mock test task which supports a seperate counter
async fn run_task(id: i32, sender: Sender<TestCall>, beat: Beat) {
    for i in 1..=id {
        assert!(beat.send().is_ok(), "Cannot send beat to the runtime");
        assert!(sender.send(TestCall::TestBeat(id)).is_ok(), "Counter fail");
        smol::Timer::after(Duration::from_secs(i as u64)).await; 
    }
    assert!(sender.send(TestCall::TestFinished).is_ok(), "Task completed counter");
}


// Mock test task without seperate counter, which is more like a real scenario 
async fn run_task2(delay_dur: Duration, total_dur: Duration, beat: Beat) {
    //println!("Starting #{} with delay: {:?}", id, delay_dur);
    let count = total_dur.as_millis() / delay_dur.as_millis();
    for _ in 1..=count {
        let _ = beat.send();
        smol::Timer::after(delay_dur).await; 
    }
}

enum TestCall {
    TestBeat(i32),
    TestFinished,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_test() -> io::Result<()> {

		let test_count: i32 = 5;

        // Test map    k: Record Id, v: Running beat count
        let mut tm: HashMap<i32, u64> = HashMap::new();  
        
        // Finished task counter
        let mut fc = 0; 
        
        // Comms between async jobs and main
        let (tx, rx) = channel::<TestCall>(); 

        // Init the dj
        let dj = TheDJ::init().unwrap();  
 
        // Init runtime that performs and monitors the  mock tasks
        smol::block_on(async {
            for i in 1..=test_count {
				let d = Duration::from_secs(i as u64);
				let b = dj.register(format!("test{:?}", i), d).unwrap();
                let s = tx.clone();
                smol::spawn(async move{run_task(i, s, b).await}).detach();
			}
        });

        // Manage the incoming calls from the mocked tasks in the async runtime
        loop {
            if fc >= test_count {break};
            match rx.recv_timeout(Duration::from_secs(test_count as u64* 2)) {
                Ok(TestCall::TestBeat(id)) => *tm.entry(id).or_insert(0) += 1,
                Ok(TestCall::TestFinished) => fc += 1,
                Err(e) => assert!(false, format!("{:?}", e)),
            }
        }

        // Assert all the things
        assert_eq!(tm.len(), test_count as usize);
        assert_eq!(dj.get_roster().unwrap().len(), test_count as usize);
        assert_eq!(test_count, fc);
        
        for i in 0..test_count {
            assert_eq!(dj.get_record(i).unwrap().beats.len(), 1 + i as usize);
        }

        Ok(())
    }


    #[test]
    fn record_test() -> io::Result<()> {

        let tc = 5;
        let td = 3;
        let now = SystemTime::now();
        let mut n = Record::new("foo".to_string(), 0, Duration::from_secs(td));

        // Test get_avg
        for i in 0..tc {
            n.add_beat(now.checked_add(Duration::from_secs(i * td)).unwrap());
        }
        assert_eq!(n.get_activity_rating().unwrap(), ActivityRating::Optimal);

        // Test clear
        n.clear();
        assert_eq!(n.beats.len(), 0);
        assert_eq!(n.get_activity_rating().unwrap(), ActivityRating::NotOnce);

        // Test one time
        n.add_beat(now);
        assert_eq!(n.get_activity_rating().unwrap(), ActivityRating::OnlyOnce);
        assert!(!n.is_optimal());
        n.clear();

        // Test get_avg_diff
        let offset = 5;
        for i in 0..tc {
            n.add_beat(now.checked_add(Duration::from_secs(i * (td + offset))).unwrap());
        }
        assert_eq!(n.get_avg_diff().unwrap(), offset as i128 * 1000) ;

        // Test get_last
        n.add_beat(now);
        assert_eq!(n.get_last().unwrap(), &now);

        // Does it still average?
        assert!(n.get_activity_rating().is_ok());

        Ok(())

    }

    #[test]
    fn a_little_stress() -> io::Result<()> {
        
		let test_count: i32 = 900;
        let durations: Vec<u64> = vec![500, 1000, 1500, 2000, 2500];
        let total_dur = Duration::from_secs(10);

        // Init the dj
        let dj = TheDJ::init().unwrap();  

        // Futures pool
        let mut futs = FuturesUnordered::new();
 
        // Init runtime that performs and monitors the  mock tasks
        smol::block_on(async {
            for i in 1..=test_count {
                let m = durations.get(i as usize % durations.len()).unwrap();
                let d = Duration::from_millis(*m);
                let b = dj.register(format!("test{:?}", i), d).unwrap();
                futs.push(run_task2(d, total_dur, b));
            }
            while let Some(()) = futs.next().await { }

            // Manage the incoming calls from the mocked tasks in the async runtime
            loop {
                let mut n = 0_i32;
                for id in dj.get_roster().unwrap(){
                    if dj.get_record(id).unwrap().beats.len() > 0 { 
                        n += 1;
                    } else {
                        break;
                    }
                }

                println!("roster contains: {}", n); 
                if n >= test_count {break};
                smol::Timer::after(Duration::from_secs(3)).await; 
            }
        });

        // Assert all the things
        assert_eq!(dj.get_roster().unwrap().len(), test_count as usize);

        Ok(())
    }
}

// ///////////////////////////////////////////////////////////////////////////
// Helpers
// /////////////////////////////////////////////////////////////////////

fn sum_each_int(n: u64) -> u64 {
    n * (n + 1) / 2
}
