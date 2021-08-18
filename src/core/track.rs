use itertools::Itertools;
use std::iter::{Enumerate, Sum, FromIterator};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::convert::TryFrom;
use std::cmp::{Ordering};

use std::iter::{ExactSizeIterator, Iterator};

// use crate::{Result, TE};
use crate::{TE, BEAT_CAP};

// ////////////////////////////////////////////////////////////////////////
// Track 
// /////////////////////////////////////////////////////////////

// A track is a collection of raw beat timestamps and is owned by the 
// record. Defined here is more raw/primitive functions, specifically used
// on the collection of timestamps that represent a historical archive of
// _beats_.
// ////////////////////////////////////////////////////////////////////////
// 
// /////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct Track(pub VecDeque<SystemTime>);

impl<'a> IntoIterator for &'a Track {
    type Item = &'a SystemTime;
    type IntoIter =  TrackIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TrackIter {
            track: self,
            idx: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TrackIter<'a> {
    track: &'a Track,
    idx: usize,
}

impl<'a> Iterator for TrackIter<'a> {
    type Item = &'a SystemTime;
    fn next (&mut self) -> Option<Self::Item> {
        let res = match self.track.0.get(self.idx) {
            Some(timestamp) =>Some(timestamp) ,
            _ => None,
        };
        self.idx += 1;
        res
    }
}

impl Track {

    pub fn add(&mut self, timestamp: SystemTime) {
        // TODO: Proper validation on what's being pushed into vecdeque
        self.0.push_back(timestamp);
        while self.0.len() > BEAT_CAP {
            self.0.pop_front();
        }

    } 

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn back(&self) -> Option<&SystemTime> {
        self.0.back()
    }

    pub fn front(&self) -> Option<&SystemTime> {
        self.0.front()
    }

    pub fn hack_track(&self, from: Option<&SystemTime>, to: Option<&SystemTime>) -> Option<Self> {
        let bv = self.0.iter()
            // .filter(|b| b > from_ && b < to_)
            .filter(|b| from.is_none() || b > &from.unwrap_or(&UNIX_EPOCH))       // Performance?
            .filter(|b| to.is_none() || b < &to.unwrap_or(&SystemTime::now()))    //
            .map(|b| b.to_owned())
            .collect::<VecDeque<SystemTime>>();
        if bv.len() > 0 {
            return Some(Track(bv))
        } else {
            None
        }
    }
    
    pub fn has_beat_since(&self, since: SystemTime) -> bool {
        match self.0.back() {
            Some(b) if b > &since => true,
            _ => false,
        }
    }

}
// ////////////////////////////////////////////////////////////////////////
// Helping Iters
// /////////////////////////////////////////////////////////////


// Linear Beat iterator
// TODO: Refactor for simplification
pub struct LinearBeat(Duration);

pub struct LinearBeatsIter<I: Iterator> { 
    iter: Enumerate<I>,
    lbr: SystemTime ,
}

pub trait LinearExt<'a>: Iterator {
    fn linear(self) -> LinearBeatsIter<Self>
    where 
        Self: Sized
    {
        LinearBeatsIter {
            iter: self.enumerate(),
            lbr: SystemTime::now(),
        }
    }
}

// Derefs into a Duration
use std::ops::Deref;
impl Deref for LinearBeat {
    type Target = Duration;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Apply the linear iterator extension trait to the Track's iterator
impl<'a> LinearExt<'a> for TrackIter<'_> {}

// This iterator is built on top of the tracks iterator, it takes two
// timestamps and creates a duration (aka beat) between the two.
impl<'a, I>Iterator for LinearBeatsIter<I>
where 
    I: Iterator<Item = &'a SystemTime>  + Copy
{
    type Item = LinearBeat;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(mut next) => {
                
                // Skip the first timestamp, as one is not enough
                // to generate a duration
                if next.0 == 0 {
                    self.lbr = *next.1;
                    match self.iter.next() {
                        Some(n) => next = n,
                        _ => return None,
                    }
                
                }                

                // Compare the currently focused timestamp with the
                // last recorded timestamp a.k.a lbr
                match next.1.duration_since(self.lbr) {
                    Ok(d) => {
                        self.lbr = *next.1;
                        Some(LinearBeat(d))
                    },
                     _ => None,
                }
            },
            _ => None
 
        }
    }
}

// Linear beat iterator into a Vec of Durations
impl<'a> FromIterator<LinearBeat> for Vec<Duration> {
    fn from_iter<I: IntoIterator<Item=LinearBeat>>(iter: I) -> Self {
        let mut c = Vec::new();
        for i in iter {
            c.push(i.0);
        }
        c
    }
}

// Get a total duration from an iterator of linear beats
impl Sum<Self> for LinearBeat {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>
    {
        iter.fold(LinearBeat(Duration::from_secs(0)) , |a, b| LinearBeat(a.0.checked_add(b.0).unwrap()))
    }
}