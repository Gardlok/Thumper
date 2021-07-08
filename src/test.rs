use crate::*;
use crate::error::ThumperError as TE;

use smol::{io, prelude::*};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::time::{SystemTime, Duration};
use std::sync::{Arc, Mutex};
use futures::stream::FuturesUnordered;
use rand;


// Mock test task which supports a seperate counter
async fn run_task(id: i32, sender: Sender<TestCall>, beat: Beat) {
    for i in 1..=id {
        assert!(beat.now().is_ok(), "Cannot send beat to the runtime");
        assert!(sender.send(TestCall::TestBeat(id)).is_ok(), "Counter fail");
        smol::Timer::after(Duration::from_secs(i as u64)).await; 
    }
    assert!(sender.send(TestCall::TestFinished).is_ok(), "Task completed counter");
}


// Mock test task without seperate counter, which is more like a real scenario 
async fn run_task2(delay_dur: Duration, total_dur: Duration, beat: Beat) {
    let count = total_dur.as_millis() / delay_dur.as_millis();
    for _ in 1..=count {
        let _ = beat.now();
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
				let b = dj.register(format!("test_beat_{:?}", i), d).unwrap();
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
            assert_eq!(dj.get_record(i).unwrap().current_track.len(), 1 + i as usize);
        }

        Ok(())
    }

    #[derive(Debug)]
    pub struct TestReport {complete: Arc<Mutex<bool>>}

    impl TestReport { 
        fn test(&self) -> Result<()> {
            assert!(self.complete.lock().and_then(|mut c| Ok(*c = true)).is_ok());
            Ok(()) 
        }
    }

    impl Report for TestReport {
        fn duration(&self)        -> Result<Duration> {Ok(Duration::from_secs(0))}
        fn init(&self)            -> Result<()> { Ok(()) }
        fn run(&mut self, _: &Record) -> Result<()> { self.test() }
        fn end(&self)             -> Result<()> { self.test() }
    }

    #[test]
    fn output_test() -> io::Result<()> {

        // Init the dj with reporting output
        let dj = TheDJ::init_with_reporting().unwrap();
        let complete = Arc::new(Mutex::new(false));
        let complete2 = complete.clone();

        assert!(dj.add_report(Box::new(TestReport{complete: complete2})).is_ok());

        let beat = dj.register(String::from("test_beat"), Duration::from_secs(1)).unwrap();
        let _ = beat.now();

        // Wait a moment for the report to run
        std::thread::sleep(Duration::from_secs(3));

        assert!(*complete.lock().unwrap());
        Ok(())
    }



    #[test]
    fn influxdb_test() -> io::Result<()> {
        let number_of_beats = 15;

        // Init the dj
        let dj = TheDJ::init_with_reporting().unwrap();  
        if let Err(e) = InfluxDB::new("http://192.168.2.14:8086".to_string(), "TestMeasure".to_string()) 
            .and_then(|influxdb| dj.add_report(Box::new(influxdb)))
            {
                assert!(false, format!("InfluxDB error: {:?}", e));
            };
 
        // Send beats to the deck
        let beat_delay_duration = Duration::from_secs(1_u64);
        if let Ok(b) = dj.register(String::from("test_beat"), beat_delay_duration)  {
            let t = SystemTime::now();
            for n in 1..number_of_beats {
                let mut t_ = t.checked_add(beat_delay_duration * n ).unwrap();
                if let Err(e) = b.from(flucuate_timestamp(&mut t_, beat_delay_duration)) {
                    assert!(false, format!("Beat send error: {:?}", e));
                }
            }
        };

        // Wait a moment for the report to run
        std::thread::sleep(Duration::from_secs(5));

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
        assert_eq!(n.current_track.len(), 0);
        assert_eq!(n.get_activity_rating().unwrap(), ActivityRating::NotOnce);

        // Test one time
        n.add_beat(now);
        assert_eq!(n.get_activity_rating().unwrap(), ActivityRating::OnlyOnce);
        assert!(!n.is_optimal());
        n.clear();

        // Test _diff
        let offset = 5;
        let offset_dur = Duration::from_secs(td + offset);
        for i in 0..tc {
            n.add_beat(now.checked_add(Duration::from_secs(i * (td + offset))).unwrap());
        }
        assert_eq!(n.get_avg_diff().unwrap(), offset as i128 * 1000) ;
        assert_eq!(n.get_beat_diffs(None).unwrap(), vec![offset_dur; (tc - 1)  as usize]) ;
        assert_eq!(n.get_beat_diffs(n.get_first_remembered()).unwrap(), vec![offset_dur; tc as usize - 1]) ;

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
                    if dj.get_record(id).unwrap().current_track.len() > 0 { 
                        n += 1;
                    } else {
                        break;
                    }
                }

                // println!("roster contains: {}", n); 
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

fn flucuate_timestamp(timestamp: &mut SystemTime, expected_interval: Duration) -> SystemTime {
    use rand::prelude::*;
    let threshold_percent = 0.01;
    let threshold = rand::thread_rng().gen_range(threshold_percent, 1.0);
    let t = &mut timestamp.checked_add(expected_interval).unwrap();
    match rand::random() {  
       true => t.checked_add(Duration::from_secs_f64(threshold)).unwrap(),
       false => t.checked_sub(Duration::from_secs_f64(threshold)).unwrap(), 
    }
}     



