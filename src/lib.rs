#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};
use std::thread;
use std::result;
use std::sync::mpsc;

mod error;
use error::BeatsError as BE;

type Result<T> = result::Result<T, BE>;

#[cfg(test)]
mod test;


// ////////////////////////////////////////////////////////////////////////
// Constants 
// /////////////////////////////////////////////////////////////
const RECORD_CAP: usize = 1000;
const BEAT_CAP: usize = 100;


// ////////////////////////////////////////////////////////////////////////
// ID Indexer 
// /////////////////////////////////////////////////////////////

// Used to keep a running index of heart beats we monitor
struct Indexer {
    next_index: i32,
    in_use: Vec<i32>,
}

impl Indexer {

    fn new() -> Indexer{
        Indexer {next_index: 0, in_use: Vec::new()}
    }

    fn next(&mut self) -> Result<i32>  {
        if let Some(n) = (0..RECORD_CAP as i32)
            .into_iter()
            .filter(|x| !self.in_use.contains(x))
            .next() 
        {
            self.in_use.push(n);
            Ok(n)
        } else { Err(BE::MaximumCapacity) }
    }

    fn remove(&mut self, job_id: i32) {
        self.in_use.retain(|&x| x != job_id);
    }
}


// ////////////////////////////////////////////////////////////////////////
// Beat 
// /////////////////////////////////////////////////////////////

// This will be owned by the process/loop we are going to monitor. It is used to 
// send heart beats back to the monitoring runtime.
pub struct Beat {
    sender: mpsc::Sender<DM2Deck>,
    pub id: i32,
}

impl Beat {
    pub fn send(&self) -> Result<()> {
        if let Err(e) = self.sender.send(
			DM2Deck::Ping(self.id, SystemTime::now())
		) { 
			Err(BE::DM2DeckSendFail(e)) 
		} else { 
			Ok(()) 
		}
    }
}


// ////////////////////////////////////////////////////////////////////////
// Record
// /////////////////////////////////////////////////////////////

// A record is going to be our data structure for the process/loop we are
// monitoring. The Records are owned by the runtime, but can be cloned out 
// for further analysis. 
//
// TODO: Consider this should change so that it is never cloned out, instead
// the runtime would provide the data within it as requested. This would add
// more overhead to the runtime however. 

// Enumerator to indicate activity level of the record
#[derive(PartialEq, Debug)]
pub enum ActivityRating {
    Optimal,     // Within 2% of expected turn around 
    NotOptimal,  // Beyond 2% difference in expected turn around
    OnlyOnce,    // Only one beat in records
    NotOnce,     // No beats
}

#[derive(Clone, Debug)]
pub struct Record {
    pub name: String,                 // Name is for reporting purposes
    pub id: i32,                      // Provided by the ID indexer
    pub freq: Duration,               // Expected duration between beats
    pub beats: VecDeque<SystemTime>,  // Queue of of past <BEAT_CAP> beats
}

impl Record {

    pub fn new(name: String, id: i32,  freq: Duration) -> Self {
        Record {
            name,
            id,
            freq,
            beats: VecDeque::new(),
        }
    }

    // Add a new beat to the queue of beats and then remove any beats
    // older than the newest <BEAT_CAP> from the queue
    pub fn add_beat(&mut self, time: SystemTime) {
        self.beats.push_back(time);
        while self.beats.len() > BEAT_CAP { self.beats.pop_front(); };
    }

    // Average of delay durations between beats
    pub fn get_avg(&self) -> Option<Duration> {

        // If there is one or less beats we can't get an average.
        if self.beats.len() < 1 { return None };

        // If there is only one beat then return an average of 0
        if self.beats.len() == 1 { return Some(Duration::from_secs(0)) };


        // Sum the durations of delays between beats
        let mut total_between_time = Duration::from_millis(0);
        for (i, t) in self.beats.iter().enumerate() {
            if i != 0 {
                if let Ok(time_diff) = t.duration_since(self.beats[i - 1]) {
                    total_between_time += time_diff
                } else {
                    // Else what?
                }
            }
        }

        // Add the last duration which is duration from last beat to now
        //TODO: Hacky patch just to get it to work, reimplement this bit immediately
        let mut extra = 0;
        if let Ok(last_dur) = SystemTime::now().duration_since(*self.beats.back().unwrap()) {
            total_between_time += last_dur; 
            extra = 1;
        }

        // Calc and return the average delay duration between beats
        let mut number_of_delays = self.beats.len() as u32;
        number_of_delays += extra;
        Some(total_between_time / number_of_delays )
    }

    // Last beat recorded
    pub fn get_last(&self) -> Option<&SystemTime> {
        self.beats.back()
    }

    // Difference of average delay durations and expected  frequency
    pub fn get_avg_diff(&self) -> Option<i128> {
        if let Some(avg) = self.get_avg() {
            Some( avg.as_millis() as i128 - self.freq.as_millis() as i128) 
        } else { None }
    }

    // activity_rating provides a generalized health status
    pub fn get_activity_rating(&self) -> Result<ActivityRating> {

        // Optimal    -> actual freq within 1% margin of expected freq
        // NotOptimal -> actual freq outside more than 1% of expected freq
        // OnlyOnce   -> only one beat recorded, no actual frequency
        // NotOnce    -> Records of beats empty

        // Calculate optimal margin range
        let exp_freq = self.freq.as_millis() as i128;
        let margin = self.freq.mul_f32(0.02).as_millis() as i128;
        let start = exp_freq - margin;
        let end = exp_freq + margin;


        // Determine if the real time freq average is optimal according
        // to the expected freq and return Activity Rating variant.
        if let Some(a) = self.get_avg()  {
            //println!("{:?} in {} -> {}", a, start, end);
            match a.as_millis() as i128{
                0 => Ok(ActivityRating::OnlyOnce),
                n if (start..=end).contains(&n) => Ok(ActivityRating::Optimal),
                _ => Ok(ActivityRating::NotOptimal),
            }
        } else { Ok(ActivityRating::NotOnce) } 

    }


    // Quick bool check whether the record is beating as expected
    pub fn is_optimal(&self) -> bool {
        if let Ok(ar) = self.get_activity_rating() {
            ar == ActivityRating::Optimal
        } else { false }
    }

    // Clear out any record of previous beats
    pub fn clear(&mut self) {
        self.beats.clear();
    }

}


// ////////////////////////////////////////////////////////////////
// MPSC Call Variants 
// ///////////////////////////////////////////////////

// Calls will be used between the runtime thread and the DJ's thread

// Calls made to the Deck 
#[derive(Debug)]
pub enum DM2Deck {
    Ping(i32, SystemTime),
    Registration(String, Duration),
    Deregistration(i32),
    RequestRecordClone(i32),
    RequestRoster,
}

// Calls made to the DJ
#[derive(Debug)]
pub enum DM2DJ {
    ID(Result<i32>),
    RecordClone(Option<Record>),
    Roster(Option<Vec<i32>>),
}


// ////////////////////////////////////////////////////////////////
// The DJ & Deck
// ///////////////////////////////////////////////////

// The DJ manages the seperated runtime thread from within the main (calling) 
// thread. It can be owned by a main controlling instance where the concurrent 
// tasks/loops spawn from and need to be monitored.
// The DJ should:
//      - Spin up the runtime thread 
//      - Provide API to the runtime through use of channels
//      - Spin up beats which are distrobuted to the concurrent tasks/loops

pub struct TheDJ {
    rt_tx: mpsc::Sender<DM2Deck>,
    rt_rx: mpsc::Receiver<DM2DJ>,
}

impl TheDJ {

    pub fn init() -> Result<TheDJ> {

        // Create the channelS that connects the calling thread and our thread
        let (dj_tx, dj_rx) = mpsc::channel();
        let (deck_tx, deck_rx) = mpsc::channel();  


        // Spawn a new thread containing the data and the runtime listening for
        // calls. Currently we don't have errors bubbling out and will need
        // to implement that. TODO
        thread::spawn(move || {

            // A.K.A The Deck
                
            // Init the indexer, which manages distribution of ID numbers
            let mut indexer = Indexer::new();
            // Map of records
            let mut rm: HashMap<i32, Record> = HashMap::new();

            // Listens for calls that currently support:
            //      - Heart beats from concurrent tasks we monitor
            //      - Admin of the data collected
            //      - Register/Drop concurrent tasks we monitor
            //      - Requests for the collected data

            loop {    
                if let Ok(call) = deck_rx.recv() { match call {

                    DM2Deck::Ping(id, time) => {
                        if let Some(n) = rm.get_mut(&id) {
                            n.add_beat(time);
                        }
                    },

                    DM2Deck::Registration(name, freq) => {
                        match indexer.next() {
                            Ok(id) => { 
                                rm.insert(id, Record::new(name, id, freq)); 
                                if let Err(_e) =  dj_tx.send(DM2DJ::ID(Ok(id))) {
                                    rm.remove(&id);
                                    // report error (?)
                                    break;
                                }
                            },
                            Err(e) => { let _ = dj_tx.send(DM2DJ::ID(Err(e))); },
                        }
                    }

                    DM2Deck::Deregistration(id) => {
                        if let Some(_) = rm.remove(&id) {
                            indexer.remove(id);
                        };
                    },

                    DM2Deck::RequestRecordClone(id) => {

                        // TODO: I know there is an easier way to do this
                        let response = match rm.get_mut(&id) {
                            Some(record) => Some(record.clone()),
                            None => None,
                        };
                        if let Err(_e) =  dj_tx.send(DM2DJ::RecordClone(response)) {
                            //println!("{:?}", _e);
                            // report error
                        }
                    },

                    DM2Deck::RequestRoster => {
                        let mut roster = Vec::new();
                        rm.iter().for_each(|x| roster.push(x.1.id));
                        let response = match roster.is_empty() {
                            false => Some(roster),
                            _ => None,
                        };
                        if let Err(_e) = dj_tx.send(DM2DJ::Roster(response)) {
                            //println!("{:?}", _e);
                            // report error
                        };
                    },
                    _ => break,
                }} else { break };
            };
        });

        // Now that the runtime is going, return the instance of TheDJ to caller
        Ok(TheDJ { 
            rt_tx: deck_tx.clone(),
            rt_rx: dj_rx,
        })

    }

    // /////////////////////////////////////////////////////////////////// //
    // The following functions make calls to the runtime setup by the init //
    // function. They will wait and listen for reponse data if the request //
    // requires it and then return said data back to original requester.   //
    // /////////////////////////////////////////////////////////////////// //

    // Add a record to the record map and return an assoiciated Beat struct
    pub fn register(&self, name: String, freq: Duration) -> Result<Beat> {

        // Verify input data
        if name.len() == 0 || freq.as_millis() == 0 {
            return Err(BE::RegisterFail("Error: Incorrect register data"))
        }

        // Make a registration call and create a new Beat with the returned id
        // and a cloned copy of the runtime call sender. For pings.
        if let Err(e) = self.rt_tx.send(DM2Deck::Registration(name, freq)) {
            Err(BE::DM2DeckSendFail(e))
        } else {
            match self.rt_rx.recv() {
                Ok(DM2DJ::ID(Ok(id))) => Ok(Beat{id, sender: self.rt_tx.clone()}),
                Ok(DM2DJ::ID(Err(e))) => Err(e),
                Err(e) => Err(BE::ChannelRecvFail(e)),
                _ => Err(BE::MaximumConfusion),
            } 
        }
    }

    // Remove a record from the record map
    pub fn unregister(&self, id: i32) -> Result<()> {
        if let Err(e) = self.rt_tx.send(DM2Deck::Deregistration(id)) {
            Err(BE::DM2DeckSendFail(e))
        } else {Ok(())}
    }

    // Request from the runtime and return a cloned snap shot of the record and 
    // it's data
    pub fn get_record(&self, id: i32) -> Result<Record> {
        if let Err(e) = self.rt_tx.send(DM2Deck::RequestRecordClone(id)) {
            return Err(BE::DM2DeckSendFail(e))
        }
        match self.rt_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(DM2DJ::RecordClone(Some(record))) => Ok(record),
            Ok(DM2DJ::RecordClone(None)) => Err(BE::MissingRecord),
            Ok(_) => Err(BE::MaximumConfusion),
            Err(e) => Err(BE::ChanRecvTimeout(e))
        }
    }
	
    // Request and return a roster of the records
    pub fn get_roster(&self) -> Result<Vec<i32>> {
        if let Err(e) = self.rt_tx.send(DM2Deck::RequestRoster) {
            return Err(BE::DM2DeckSendFail(e))
        }
        match self.rt_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(DM2DJ::Roster(Some(roster))) => Ok(roster),
            Ok(DM2DJ::Roster(None)) => Err(BE::EmptyRoster),
            Ok(_) => Err(BE::MaximumConfusion),
            Err(e) => Err(BE::ChanRecvTimeout(e))
        }

    }

}


