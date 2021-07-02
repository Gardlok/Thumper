#![allow(dead_code)]
pub mod error;
pub mod output;
pub mod core;

pub use crate::output::{Output, Report, DM2OutputRunner, InfluxDB};
pub use crate::core::{TheDJ, DM2DJ};
pub use crate::core::{Track, TrackIter, CurrentTrack, CurrentTrackRef};
pub use crate::core::{Beat};
pub use crate::core::{Record, ActivityRating};
pub use crate::core::{Deck, DM2Deck, Arm};

pub use crate::error::{BE, Result};

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


// ////////////////////////////////////////////////////////////////
// 
// ///////////////////////////////////////////////////


