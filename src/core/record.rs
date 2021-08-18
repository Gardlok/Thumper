use std::collections::VecDeque;
use std::iter::{Sum, ExactSizeIterator, Iterator};
// use std::
use itertools::Itertools;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{Track, Result, BEAT_CAP, LinearExt, LinearBeat};

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
    Optimal,    // Within 2% of expected turn around
    NotOptimal, // Beyond 2% difference in expected turn around
    OnlyOnce,   // Only one beat in records
    NotOnce,    // No beats
}


#[derive(Clone, Debug)]
pub struct Record {
    pub name: String,                 // Name is for reporting purposes
    pub id: i32,                      // Provided by the ID indexer
    pub freq: Duration,               // Expected duration between beats
    pub pattern: Vec<usize>,          // Best Expected pattern of the beats
    pub creation: SystemTime,         //
    pub deployment: SystemTime,       // Record's start time
    pub raw_track: Track,             // Queue of of current <BEAT_CAP> beats
    pub tuned_track: Track,           // A possibly manipulated copy of current raw_track  
}

impl Record {
    pub fn new(name: String, id: i32) -> Self {
        Record {
            name,
            id,
            creation: SystemTime::now(),
            deployment: SystemTime::now(),
            pattern: Vec::new(),
            freq: Duration::from_secs(0),
            raw_track: Track(VecDeque::new()),
            tuned_track: Track(VecDeque::new()),
        }
    }

    pub fn add_beat(&mut self, time: SystemTime) {
        self.raw_track.add(time);
    }

    pub fn set_deployment(&mut self, deployment: SystemTime) {
        self.deployment = deployment;
    }

    pub fn set_expected_freq(&mut self, expected: Duration) {
        self.freq = expected;
    }

    pub fn set_expected_pattern(&mut self, expected: Vec<usize>) {
        self.pattern = expected;
    }

    // Clear out any record of previous beats
    pub fn clear(&mut self) {
        self.raw_track.clear();
    }

    pub fn has_beat_since(&self, lrb: Option<&SystemTime>) -> bool {
        self.raw_track.has_beat_since(*lrb.unwrap_or(&SystemTime::now()))
    }



// ////////////////////////////////////////////////////////////////////////
// Expected to be refactored out
// /////////////////////////////////////////////////////////////

    // This "get_average" is based on a linear default pattern of constant beats 
    pub fn get_average(&self) -> Option<Duration> {

        // If there is one or less.current_track.we can't get an average.
        if self.raw_track.len() < 1 { return None };

        // If there is only one beat then return an average of 0
        if self.raw_track.len() == 1 { return Some(Duration::from_secs(0)) };

        // Sum the delay duration between beats
        let mut total_between_time: Duration = *self.raw_track.into_iter().linear().sum::<LinearBeat>();

        // Add the last duration which is duration from last beat to now
        //TODO: Hacky patch just to get it to work, reimplement this bit immediately
        let mut extra = 0;
        if let Ok(last_dur) = SystemTime::now().duration_since(*self.raw_track.back().unwrap()) {
            total_between_time += last_dur; 
            extra = 1;
        }

        // Calc and return the average delay duration between beats
        let mut number_of_delays = self.raw_track.len() as u32 - 1;
        number_of_delays += extra;
        Some(total_between_time / number_of_delays)
    }

    // Difference of average delay durations and expected  frequency
    pub fn get_avg_diff(&self) -> Option<i128> {
        if let Some(avg) = self.get_average() {
            Some(avg.as_millis() as i128 - self.freq.as_millis() as i128)
        } else {
            None
        }
    }

    // activity_rating provides a generalized health status
    pub fn get_activity_rating(&self) -> Result<ActivityRating> {
        // Optimal    -> actual freq within 1% margin of expected freq
        // NotOptimal -> actual freq outside more than 1% of expected freq
        // OnlyOnce   -> only one beat recorded, no actual frequency
        // NotOnce    -> Records of raw_track are empty

        // Calculate optimal margin range
        let exp_freq = self.freq.as_millis() as i128;
        let margin = self.freq.mul_f32(0.02).as_millis() as i128;
        let start = exp_freq - margin;
        let end = exp_freq + margin;

        // Determine if the real time freq average is optimal according
        // to the expected freq and return Activity Rating variant.
        if let Some(a) = self.get_average() {
            match a.as_millis() as i128 {
                0 => Ok(ActivityRating::OnlyOnce),
                n if (start..=end).contains(&n) => Ok(ActivityRating::Optimal),
                _ => Ok(ActivityRating::NotOptimal),
            }
        } else {
            Ok(ActivityRating::NotOnce)
        }
    }
    // Quick bool check whether the record is beating as expected
    pub fn is_optimal(&self) -> bool {
        if let Ok(ar) = self.get_activity_rating() {
            ar == ActivityRating::Optimal
        } else {
            false
        }
    }

    // Returns a list of beats associated with the record, if a last beat record is
    // provided then this will return beats that occured after that
    pub fn get_beats(&self, lbr: Option<&SystemTime>) -> Option<Vec<SystemTime>> {
        match self.raw_track.hack_track(lbr, None) {
            Some(raw_track) if raw_track.len() > 0 => {
                Some(raw_track.0.iter().map(|x| x.clone()).collect::<Vec<SystemTime>>())
            }
            _ => None,
        }
    }

    // // Returns a vec of the durations between beats
    pub fn get_beat_diffs(&self, lbr: Option<&SystemTime>) -> Option<Vec<Duration>> {
        // match self.raw_track.hack_track(None, None) {
        match self.raw_track.hack_track(lbr, None) {
        // match Some(&self.raw_track) {
            Some(n) if n.0.len() > 1 => {
                Some(self.raw_track.into_iter().linear().collect())
            },
            _ => return None,
        }
    }

    // A very very basic implementation of a self determination of beat frequency. This
    // requires a lot more logic to achieve accuracy. Maybe a confidence rating as well?
    // TODO: This needs to be much smarter
    // pub fn guess_freq(&self) -> Option<Duration> {
    //     match self.get_beats(None) {
    //         // If no beats
    //         None => None,
    //         // If one beat, assume duration between the beats deployed time and actual transmit time
    //         Some(beats) if beats.len() == 1 => self.deployment.duration_since(*beats[0]).ok(),
    //         // If two beats, assume duration to be similiar to duration between the two timestamps
    //         Some(beats) if beats.len() == 2 => beats[1].duration_since(*beats[0]).ok(),
    //         // If more than two, get an average of duration between timestamps
    //         Some(_) => self.get_average(),
    //         // Anything else
    //         _ => None,
    //     }
    // }

}



