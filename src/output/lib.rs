use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::sync::{Arc, mpsc, RwLock};
use std::collections::HashMap;

use crate::{DM2Deck, TE, Record};

// ////////////////////////////////////////////////////////////////
// Type less
// ///////////////////////////////////////////////////
type AtomicRecordMap = Arc<RwLock<HashMap<i32, Record>>>;
type ReportsAndTheirLBRs = Vec<(Box<dyn Report>, SystemTime)>;

// ////////////////////////////////////////////////////////////////
// Enumeration for Messaging OutputRunner
// ///////////////////////////////////////////////////
#[derive(Debug)]
pub enum DM2OutputRunner {
    RegisterOutput(Box<dyn Report>),
    StopOutput,
}

// ////////////////////////////////////////////////////////////////
// Report Trait
// ///////////////////////////////////////////////////
// This trait is used to build a report object which consumes historical
// data collected in the program's main loop, and then processes and delivers
// it to it's destination. The destination is described when building the
// report object. This object will be called from the Output runtime to execute.
pub trait Report: Send {
    fn duration(&self)                 -> Result<Duration, TE>;
    fn init(&self)                     -> Result<(), TE>;
    fn run(&mut self, record: &Record) -> Result<(), TE>;
    fn end(&self)                      -> Result<(), TE>;
}

impl std::fmt::Debug for dyn Report {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "PLACEHOLDER-TODO")
    }
}

// ////////////////////////////////////////////////////////////////
// Report Wrapper for additional metric tracking 
// ///////////////////////////////////////////////////
pub struct ReportWrapper {
    report: Box<dyn Report>,
    freq: Duration,
    last: SystemTime,
}

// ////////////////////////////////////////////////////////////////
// Output Runtime
// ///////////////////////////////////////////////////
// Output manages the reports and their execution. It may contain multiple
// reports. It has a runtime that will listen for messages, as well as sending
// a 'reminder' message to the main runtime to update it's copy of the records
// of beats 
pub struct Output {
    pub atomic_record_map: AtomicRecordMap,
    // pub rt_tx: mpsc::Sender<DM2Deck>,                     
    pub outputrunner_rx: mpsc::Receiver<DM2OutputRunner>,
}

impl Output {
    pub fn run(self) {
        let mut reports: Vec<ReportWrapper> = Vec::new();
        let mut lrb_map: HashMap<i32, SystemTime> = HashMap::new();
        loop {
            // Determine if there is any pending messages for this loop to act on
            match self.outputrunner_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(msg) => {
                    match msg {
                        DM2OutputRunner::RegisterOutput(report) => {
                            // Init the report and add it to roster of reports
                            if report.init().is_ok() {
                                let f = report.duration().unwrap_or(Duration::from_secs(0));
                                let r = ReportWrapper {
                                   report: report,
                                   freq: f,
                                   last: UNIX_EPOCH, 
                                };
                                reports.push(r);
                            } else {
                                println!("Could not init report");
                            }
                        },
                        DM2OutputRunner::StopOutput => {},
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {},
                Err(e) => {
                    println!("I knew it! {:?}", e);
                    break;
                }
            }

            // If there are no reports to use then there is no need to proceed
            if reports.is_empty() { continue };

            // Run reports
            let arm_copy = self.atomic_record_map.read().and_then(|arm| Ok(arm.clone()));

            // Hopefully at this point self.atomic_record_map has it's read lock released
            // this may require further testing
            if let Ok(arm) = arm_copy {
                
                // Iter through reports (These are wrapped with RecordWrapper, that adds 
                // a layer of extra metric information needed to track LBR or Last Beat Record)
                for rw in reports.iter_mut() {
                    
                    // If report hasn't waited long enough to run again then no need to proceed
                    if SystemTime::now() < rw.last.checked_add(rw.freq).unwrap_or(UNIX_EPOCH) { continue };
                    
                    // Update the lrb_map with new beats since last iteration, if any.
                    for (id, record) in arm.iter() {
                        match lrb_map.entry(*id) {
                            std::collections::hash_map::Entry::Occupied(o) => {
                                if !record.has_beat_since(Some(o.into_mut())) {
                                    // In this scenario the record exist but there are no updates to report
                                    continue
                                }
                            },
                            std::collections::hash_map::Entry::Vacant(v) => {
                                if let Some(timestamp) = record.raw_track.back() {
                                    // Insert the new record to lrb
                                    v.insert(*timestamp);
                                }
                            },
                        }

                        // Run the report
                        match rw.report.run(record) {
                            Ok(_) => {},
                            Err(TE::NothingNewToReport) => {},
                            Err(e) => {
                                println!("This error {:?}", e);
                            },
                        }
                    }

                    // Set the last timestamp this report was run
                    rw.last = SystemTime::now();
                }
            };
        }
    }
}
