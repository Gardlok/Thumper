
use std::time::SystemTime;
use std::sync::mpsc;

use crate::{Result, BE, DM2Deck};

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
    pub fn now(&self) -> Result<()> {
        if let Err(e) = self.sender.send(
			DM2Deck::Ping(self.id, SystemTime::now())
		) { 
			Err(BE::DM2DeckSendFail(e)) 
		} else { 
			Ok(()) 
		}
    }

    pub fn from(&self, timestamp: SystemTime) -> Result<()> {
        if let Err(e) = self.sender.send(
			DM2Deck::Ping(self.id, timestamp)
		) { 
			Err(BE::DM2DeckSendFail(e)) 
		} else { 
			Ok(()) 
		}
    }
}