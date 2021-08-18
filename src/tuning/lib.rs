use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::sync::{Arc, mpsc, RwLock};
use std::collections::HashMap;


use crate::{DM2Deck, TE, Record};

// ////////////////////////////////////////////////////////////////
// Type less
// ///////////////////////////////////////////////////
type AtomicRecordMap = Arc<RwLock<HashMap<i32, Record>>>;

// ////////////////////////////////////////////////////////////////
// Enumeration for Messaging the Tuner
// ///////////////////////////////////////////////////
#[derive(Debug)]
pub enum DM2AutoTuner {
    RegisterAttunment(Box<dyn Attunement>),
    ClearAttunements,
}

// ////////////////////////////////////////////////////////////////
// Primitive level tunings that can be applied
// ///////////////////////////////////////////////////
#[derive(Debug)]
pub enum BetterTo {
    GoOver,
    GoUnder,
}

#[derive(Debug)]
pub enum Tuning {
    ExpectedFreqMillis(u32),
    VariantThresholdPercent(u8),
    LenienceDirection(BetterTo),
    SampleRateMillis(u32),
    WarmupMillis(u32),
}


#[derive(PartialEq, Debug, Clone)]
pub enum ConfidenceLevel {
    Very,        // Most confident
    Likely,      //
    Maybe,       //
    Nil,         //
    UserDefined, // Least confident
}





// ////////////////////////////////////////////////////////////////
// Attunement Trait
// ///////////////////////////////////////////////////
pub trait Attunement: Send {
    fn init(&mut self, record: &Record)   -> Result<(), TE>;
    fn apply(&self, record: &mut Record)  -> Result<(), TE>;
}

impl std::fmt::Debug for dyn Attunement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "TUNER-PLACEHOLDER-TODO")
    }
}

// ////////////////////////////////////////////////////////////////
// 
// ///////////////////////////////////////////////////
// Auto tuner is an effort to better understand the incoming data from
// the beats. Tuning to the beats allows us to get a more accurate status
// read from the beat data. 
// 
//
pub struct AutoTuner {
    pub atomic_record_map: AtomicRecordMap,
    pub rx: mpsc::Receiver<DM2AutoTuner>,
}

impl AutoTuner {
    pub fn run(self) {
        let mut attunements: Vec<Box<dyn Attunement>> = Vec::new();
        loop {

            // First check if any new instructions have arrived and 
            // process them accordingly
            match self.rx.recv_timeout(Duration::from_secs(1)) {
                Ok(msg) => {
                    match msg {
                        DM2Tuner::RegisterAttunement(attunement) => {
                            if attunement.init().is_ok() {
                                attunements.push(attunement);
                            } else {
                                println!("Could not init attunments");
                            }
                        },
                        DM2Tuner::ClearAttunements => {},
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {},
                Err(e) => {
                    println!("I knew it! {:?}", e);
                    break;
                }
            }

            // If there are no attunements to use then there is no need to proceed
            // if attunements.is_empty() { continue };

            let arm_copy = self.atomic_record_map.read().and_then(|arm| Ok(arm.clone()));
            if let Ok(arm) = arm_copy {
                for attunement in attunements.iter() {
                    for (id, record) in arm.iter_mut() {
                        let initial_score = record.get_activity_rating();
                        match attunement.run(record) {
                            Ok(_) => {},
                            // Err(TE::NothingNewToReport) => {},
                            Err(e) => {
                                println!("This error {:?}", e);
                            },
                        }
                        let new_score = record.get_activity_rating();
                        println!("Old: {:?}\nNew:{:?}", initial_score, new_score);
                    }
                }
            };
        }
    }
}

struct 
