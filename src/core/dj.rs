use std::time::Duration;
use std::thread;
use std::sync::mpsc;

use crate::{Deck, DM2Deck, BE, Result, Record, Arm, DM2OutputRunner, Report, Output, Beat};

// ////////////////////////////////////////////////////////////////
// The DJ 
// ///////////////////////////////////////////////////

// The DJ manages the seperated runtime thread from within the main (calling) 
// thread. It can be owned by a main controlling instance where the concurrent 
// tasks/loops spawn from and need to be monitored.
// The DJ should:
//      - Spin up the runtime thread 
//      - Provide API to the runtime through use of channels
//      - Set up any output ( What about triggers and callbacks)
//      - Spin up beats which are distrobuted to the concurrent tasks/loops

pub struct TheDJ {
    rt_tx: mpsc::Sender<DM2Deck>,
    rt_rx: mpsc::Receiver<DM2DJ>,
    outputrunner_tx: mpsc::Sender<DM2OutputRunner>,
    atomic_record_map: Option<Arm>,
}

// Calls made to the DJ
#[derive(Debug)]
pub enum DM2DJ {
    ID(Result<i32>),
    ARM(Arm),
}


impl TheDJ {

    // Init with or without output reporting
    pub fn init()                -> Result<TheDJ> { Self::init_(false) }
    pub fn init_with_reporting() -> Result<TheDJ> { Self::init_(true) }

    fn init_(should_report: bool) -> Result<TheDJ> {

        // Create the channelS that connects the threads
        let (dj_tx, dj_rx) = mpsc::channel();
        let (deck_tx, deck_rx) = mpsc::channel();  
        let deck_tx_2 = deck_tx.clone();
        let deck_tx_3 = deck_tx.clone();
        let (outputrunner_tx, outputrunner_rx) = mpsc::channel();  

        // Spin up the Deck, where the core data is stored/processed
        Deck::run(deck_rx, dj_tx);

        // This will tell the deck to update the atomic record map every 1 second
        thread::spawn(move || {
            loop {
                if let Err(e) = deck_tx_2.send(DM2Deck::UpdateAtomicRecordMap) {
                    panic!("Could not send reqwuest to update: {:?}", e);
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        // Init the DJ 
        let mut the_dj = TheDJ { 
            rt_tx: deck_tx_3.clone(),
            rt_rx: dj_rx,
            outputrunner_tx,
            atomic_record_map: None,
        };

        // Get the new DJ a rwlock read only link of the atomic record map
        if let Err(e) = the_dj.rt_tx.send(DM2Deck::Init()) {
            return Err(BE::DM2DeckSendFail(e));
        } else {
            match the_dj.rt_rx.recv() {
                Ok(DM2DJ::ARM(arm)) => {
                    let arm_ = Some(arm.clone());
                    the_dj.atomic_record_map = arm_
                },
                Err(e) => return Err(BE::ChannelRecvFail(e)),
                _ => return Err(BE::MaximumConfusion),
            }; 
        }

        // If reporting, init the output runtime
        if should_report {
            let arm_ = the_dj.atomic_record_map.clone().expect("ARM not initialized");
            thread::spawn(move  || {
                let output_runner = Output {
                    atomic_record_map:arm_, 
                    rt_tx: deck_tx.clone(),
                    outputrunner_rx: outputrunner_rx, 
                };
                output_runner.run();
            });
        }

        // Return the instance of TheDJ  to caller
        Ok(the_dj)
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
            return Err(BE::RegisterFail ("Error: Incorrect register data"))
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

    // TODO optimize
    // Returns a single record 
    pub fn get_record(&self, id: i32) -> Result<Record> {
        if let Ok(record_map) = self.atomic_record_map.as_ref().expect("You have no ARM here").read() {
            if let Some(record) = record_map.get(&id) {
                return Ok(record.clone());
            }
        }
        Err(BE::MissingRecord)
    }
	
    // Returns a list of record ids
    pub fn get_roster(&self) -> Result<Vec<i32>> {
        if let Ok(record_map) = self.atomic_record_map.as_ref().expect("You have no ARM here").read() { 
            let mut roster = Vec::new();
            record_map.iter().for_each(|x| roster.push(x.1.id));
            if !roster.is_empty() {
                return Ok(roster)
            }
            return Err(BE::EmptyRoster)
        }
        Err(BE::MaximumConfusion)
    }

    // Add an output stream
    // Eventually we'll be able to remove/stop a current running output
    // when that is ready this function should return an ID
    pub fn add_report(&self, report: Box<dyn Report>) -> Result<()> {
        self.outputrunner_tx.send(DM2OutputRunner::RegisterOutput(report))?;
        Ok(())
    }

}