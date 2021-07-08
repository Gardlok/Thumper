use itertools::Itertools;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

// ////////////////////////////////////////////////////////////////////////
// Track 
// /////////////////////////////////////////////////////////////

// A track is a ollection of raw beat timestamps and is owned by the 
// record. Defined here is more raw/primitive functions, specifically used
// on the collection of timestamps that represent a historical archive of
// _beats_.

// ////////////////////////////////////////////////////////////////////////
// Type less 
// /////////////////////////////////////////////////////////////
pub type CurrentTrack = VecDeque<SystemTime>;
pub type CurrentTrackRef<'a> = VecDeque<&'a SystemTime>;

// ////////////////////////////////////////////////////////////////////////
// Iterator
// /////////////////////////////////////////////////////////////
// pub struct TrackIter<'a> {
//     pub track: &'a VecDeque<SystemTime>,
//     pub index: usize,
// }

// impl<'a> Iterator for TrackIter<'a> {
//     type Item = SystemTime;
//     fn next(&mut self) -> Option<Self::Item> {
//         if self.index < self.track.len() {
//             self.index += 1;
//             self.track.get(self.index - 1).and_then(|x| Some(x.clone()))
//         } else {
//             None
//         }
//     }
// }

// ////////////////////////////////////////////////////////////////////////
// Trait
// /////////////////////////////////////////////////////////////
pub trait Track {
    fn get_diffs(&self) -> Vec<Duration>;
    fn get_since(&self, since: &SystemTime) -> Option<VecDeque<&SystemTime>>;
    fn has_updated_since(&self, since: SystemTime) -> bool;
    fn get_average(&self) -> Option<Duration>;
}

impl Track for CurrentTrack {
    fn get_diffs(&self) -> Vec<Duration> {
        self.iter()
            .collect::<Vec<_>>()
            .windows(2)
            .map(|t| t[1].duration_since(*t[0]).unwrap_or(Duration::from_secs(0)))
            .collect::<Vec<Duration>>()
    }

    fn get_since(&self, since: &SystemTime) -> Option<VecDeque<&SystemTime>> {
        let mut bv = VecDeque::new();
        self.iter()
            .filter(|b| b > &&since)
            .for_each(|b| bv.push_back(b));
        if bv.len() > 0 {
            return Some(bv)
        } else {
            None
        }
    }
    
    fn has_updated_since(&self, since: SystemTime) -> bool {
        match self.back() {
            Some(b) if b > &since => true,
            _ => false,
        }
    }

    fn get_average(&self) -> Option<Duration> {

        // If there is one or less.current_track.we can't get an average.
        if self.len() < 1 { return None };

        // If there is only one beat then return an average of 0
        if self.len() == 1 { return Some(Duration::from_secs(0)) };

        // Sum the delay duration between beats
        let mut total_between_time: Duration = self.iter()
            .tuple_windows()
            .filter_map(|(a, b)| b.duration_since(*a).ok())
            .sum();

        // Add the last duration which is duration from last beat to now
        //TODO: Hacky patch just to get it to work, reimplement this bit immediately
        let mut extra = 0;
        if let Ok(last_dur) = SystemTime::now().duration_since(*self.back().unwrap()) {
            total_between_time += last_dur; 
            extra = 1;
        }

        // Calc and return the average delay duration between beats
        let mut number_of_delays = self.len() as u32 - 1;
        number_of_delays += extra;
        Some(total_between_time / number_of_delays)
    }
}