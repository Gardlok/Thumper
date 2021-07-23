use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::iter::{Iterator, ExactSizeIterator};

use crate::{Result, BEAT_CAP, CurrentTrack, Track};
// use crate::{Result, BEAT_CAP, CurrentTrack, TrackIter, Track};

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
    deployment: SystemTime,
    pub freq: Duration,               // Expected duration between beats
    pub current_track: CurrentTrack,  // Queue of of past <BEAT_CAP> beats
    track_index: usize,
}

impl Record {

    pub fn new(name: String, id: i32) -> Self {
        Record {
            name,
            id,
            deployment: SystemTime::now(),
            freq: Duration::from_secs(0),
            current_track: VecDeque::new(),
            track_index: 0,
        }
    }

    pub fn set_expected_freq(&mut self, expected: Duration) {
        self.freq = expected;
    }

    // Add a new beat to the queue of.current_track.and then remove any beats
    // older than the newest <BEAT_CAP> from the queue
    pub fn add_beat(&mut self, time: SystemTime) {
        // TODO: Proper validation on what's being pushed into vecdeque
        self.current_track.push_back(time);
        while self.current_track.len() > BEAT_CAP { self.current_track.pop_front(); };
    }

    pub fn set_deployment(&mut self, deployment: SystemTime) {
        self.deployment = deployment;
    }

    // Last beat recorded
    pub fn get_last(&self) -> Option<&SystemTime> {
        self.current_track.back()
    }

    // Currently the oldest beat in record
    pub fn get_first_remembered(&self) -> Option<&SystemTime> {
        self.current_track.front()
    }

    // Difference of average delay durations and expected  frequency
    pub fn get_avg_diff(&self) -> Option<i128> {
        if let Some(avg) = self.current_track.get_average() {
            Some( avg.as_millis() as i128 - self.freq.as_millis() as i128) 
        } else { None }
    }

    // activity_rating provides a generalized health status
    pub fn get_activity_rating(&self) -> Result<ActivityRating> {

        // Optimal    -> actual freq within 1% margin of expected freq
        // NotOptimal -> actual freq outside more than 1% of expected freq
        // OnlyOnce   -> only one beat recorded, no actual frequency
        // NotOnce    -> Records of current_track are empty

        // Calculate optimal margin range
        let exp_freq = self.freq.as_millis() as i128;
        let margin = self.freq.mul_f32(0.02).as_millis() as i128;
        let start = exp_freq - margin;
        let end = exp_freq + margin;

        // Determine if the real time freq average is optimal according
        // to the expected freq and return Activity Rating variant.
        if let Some(a) = self.current_track.get_average()  {
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
        self.current_track.clear();
    }

    pub fn has_updated_since(&self, lrb: Option<&SystemTime>) -> bool {
        self.current_track.has_updated_since(*lrb.unwrap_or(&SystemTime::now()))
    }

    // Returns a list of beats associated with the record, if a last beat record is
    // provided then this will return beats that occured after that
    pub fn get_beats(&self, lbr: Option<&SystemTime>) -> Option<Vec<&SystemTime>> {
        match self.current_track.get_since(lbr.unwrap_or(&UNIX_EPOCH)) {
            Some(current_track) if current_track.len() > 0 => {
                // Build and return the vec of beats
                let mut bv = Vec::new();
                current_track.iter().for_each(|beat| bv.push(*beat));
                Some(bv)
            },
            _ => None,
        } 
    }

    // Returns a vec of the durations between beats
    pub fn get_beat_diffs(&self, lbr: Option<&SystemTime>) -> Option<Vec<Duration>> {
        match self.current_track.get_since(lbr.unwrap_or(&UNIX_EPOCH)) {
            Some(n) if n.len() > 1 => {
                Some(self.current_track.get_diffs())
            },
            _ => return None,
        }
    }

    // A very very basic implementation of a self determination of beat frequency. This
    // requires a lot more logic to achieve accuracy. Maybe a confidence rating as well?
    // TODO: This needs to be much smarter
    pub fn guess_freq(&self) -> Option<Duration> {
        match self.get_beats(None) {
            // If no beats
            None => None,
            // If one beat, assume duration between the beats deployed time and actual transmit time
            Some(beats) if beats.len() == 1 => { self.deployment.duration_since(*beats[0]).ok() },
            // If two beats, assume duration to be similiar to duration between the two timestamps
            Some(beats) if beats.len() == 2 => { beats[1].duration_since(*beats[0]).ok() },
            // If more than two, get an average of duration between timestamps
            Some(_) => self.current_track.get_average(),
            // Anything else
            _ => None,
        }
    }

}


