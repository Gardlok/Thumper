use std::time::{Duration, SystemTime};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

// use crate::{Record, DM2DJ, Indexer, ConfidenceLevel, DM2OutputRunner};
use crate::{Record, DM2DJ, Indexer, DM2OutputRunner};


// ////////////////////////////////////////////////////////////////
// Type less
// ///////////////////////////////////////////////////
pub type Arm = Arc<RwLock<HashMap<i32, Record>>>;

// ////////////////////////////////////////////////////////////////
// Enumeration for Messaging the Deck
// ///////////////////////////////////////////////////
#[derive(Debug)]
pub enum DM2Deck {
    Ping(i32, SystemTime),
    Registration(String),
    Deploy(i32, SystemTime),
    // SetExpectedFreq(i32, Duration, ConfidenceLevel),
    SetExpectedFreq(i32, Duration),
    Deregistration(i32),
    Init()
}

// ////////////////////////////////////////////////////////////////
// The Deck Runtime
// ///////////////////////////////////////////////////
pub struct Deck;

impl Deck {
    pub fn run(rx: Receiver<DM2Deck>, 
               dj_tx: Sender<DM2DJ>,
               outputrunner_tx: Sender<DM2OutputRunner>,
            ) {

        // Spawn a new thread owning the core data.
        // Currently we don't have errors bubbling out and will need
        // to implement that. TODO !!
        thread::spawn(move || {
                
            // Init the indexer, which manages distribution of ID numbers
            let mut indexer = Indexer::new();

            // Record map and atomic variants
            let mut rm: HashMap<i32, Record> = HashMap::new();
            let arm = Arc::new(RwLock::new(rm.clone()));
            let arm2 = arm.clone();

            loop {    
                if let Ok(call) = rx.recv() { 
                    match call {
                        DM2Deck::Init() => {
                            if let Err(e) =  dj_tx.send(DM2DJ::ARM(arm2.clone())) {
                                panic!("TX to DJ failed: {:?}", e)
                            } 
                        },
                        DM2Deck::Deploy(id, time) => {
                            if let Some(n) = rm.get_mut(&id) {
                                n.set_deployment(time);
                            } else {
                                continue
                            }
                        },
                        // DM2Deck::SetExpectedFreq(id, expected, confidence_level) => {
                        DM2Deck::SetExpectedFreq(id, expected) => {
                            if let Some(n) = rm.get_mut(&id) {
                                // n.set_expected_freq(expected, confidence_level);
                                n.set_expected_freq(expected);
                            } else {
                                continue
                            }
                        },
                        DM2Deck::Ping(id, time) => {
                            if let Some(n) = rm.get_mut(&id) {
                                n.add_beat(time);
                            } else {
                                continue
                            }
                        },
                        DM2Deck::Registration(name) => {
                            match indexer.next() {
                                Ok(id) => { 
                                    rm.insert(id, Record::new(name, id)); 
                                    if let Err(_e) =  dj_tx.send(DM2DJ::ID(Ok(id))) {
                                        rm.remove(&id);
                                        break;
                                    }
                                },
                                Err(e) => { let _ = dj_tx.send(DM2DJ::ID(Err(e))); },
                            }
                        }
                        DM2Deck::Deregistration(id) => {
                            if let Some(_) = rm.remove(&id) {
                                indexer.remove(id);
                            } else {
                                continue
                            };
                        },
                    };

                    // At this point we assume _some_ changes have been made and will need to
                    // update the atomic record map.
                    if let Ok(mut arm) = arm.write() {
                        *arm = rm.clone();
                    }

                } else { break };
            };
        });

    }
}
