
use std::time::{Duration, SystemTime};
use std::sync::mpsc;

// use crate::{Result, TE, DM2Deck, ConfidenceLevel::UserDefined};
use crate::{Result, TE, DM2Deck};

// ////////////////////////////////////////////////////////////////////////
// Beat 
// /////////////////////////////////////////////////////////////

// This will be owned by the process/loop we are going to monitor. It is used to 
// send heart beats back to the monitoring runtime.
pub struct Beat {
    pub sender: mpsc::Sender<DM2Deck>,
    pub id: i32,
}

impl Beat {

	// I'd like to determine when the Beat has changed ownership, or moved
	// This is a temp solution until we figure something better out
	pub fn deploy(&self) -> Result<()> {
		match self.sender.send(DM2Deck::Deploy(self.id, SystemTime::now())) {
			Err(e) => Err(TE::DM2DeckSendFail(e)),
			_ => Ok(()),
		}
	}

    pub fn now(&self) -> Result<()> {
        if let Err(e) = self.sender.send(
			DM2Deck::Ping(self.id, SystemTime::now())
		) { 
			Err(TE::DM2DeckSendFail(e)) 
		} else { 
			Ok(()) 
		}
    }

    pub fn from(&self, timestamp: SystemTime) -> Result<()> {
        if let Err(e) = self.sender.send(
			DM2Deck::Ping(self.id, timestamp)
		) { 
			// Err(TE::DM2DeckSendFail(e)) 
			Err(TE::DM2DeckSendFail(e)) 
		} else { 
			Ok(()) 
		}
    }

	pub fn set_expected_freq(&self, expected: Duration) -> Result<()> {
		// match self.sender.send(DM2Deck::SetExpectedFreq(self.id, expected, UserDefined)) {
		match self.sender.send(DM2Deck::SetExpectedFreq(self.id, expected)) {
			Err(e) => Err(TE::DM2DeckSendFail(e)),
			_ => Ok(()),
		}
	}

	pub fn set_deployment(&self, deployment: SystemTime) -> Result<()> {
		match self.sender.send(DM2Deck::Deploy(self.id, deployment)) {
			Err(e) => Err(TE::DM2DeckSendFail(e)),
			_ => Ok(()),
		}
	}

}

// Upon the Beat being dropped...remove it from the record map
impl Drop for Beat {
	fn drop(&mut self) {
		let _ = self.sender.send(DM2Deck::Deregistration(self.id));
    }
}
		
