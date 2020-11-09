use std::error::Error;
use std::fmt;
use std::convert::From;
use std::io::Error as IoError;
use std::str::Utf8Error;
use std::sync::mpsc::{SendError, RecvError, RecvTimeoutError};

use crate::DM2Deck;


#[derive(Debug)]
pub enum BeatsError {

    // standard io error
	Io(IoError),

    // stadard utf8 error
    Utf8(Utf8Error),

    // MPSC send error
    DM2DeckSendFail(SendError<DM2Deck>),

    // MPSC recv error
    ChannelRecvFail(RecvError),

    // MPSC recv timeout error
    ChanRecvTimeout(RecvTimeoutError),

    // When a request for roster is made and it's enpty
    EmptyRoster,

    // When a request for a record instance is made and the record does nnot exist
    MissingRecord,

    // When registering the record fails, usually by incorrect input data
	RegisterFail(&'static str),

    // When failing to drop a record from the registry
	UnregisterFail,

    // When the indexer has exhausted all available ids
    MaximumCapacity,

    // When things get bad. Maybe received a response out of order
	MaximumConfusion,
}

impl Error for BeatsError {}

impl fmt::Display for BeatsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BeatsError::Io(ref cause) => write!(f, "I/O Error: {}", cause),
            BeatsError::Utf8(ref cause) => write!(f, "UTF-8 Error: {}", cause),
            BeatsError::DM2DeckSendFail(ref cause) => write!(f, "DM2Deck: {}", cause),
            BeatsError::ChannelRecvFail(ref cause) => write!(f, "MPSC Recv Error: {}", cause),
            BeatsError::ChanRecvTimeout(ref cause) => write!(f, "MPSC Recv Error: {}", cause),
            BeatsError::EmptyRoster => write!(f, "The Roster is empty"),
            BeatsError::MissingRecord => write!(f, "Cannot find the record"),
            BeatsError::RegisterFail(e) => write!(f, "Register Error: {}", e),
            BeatsError::UnregisterFail => write!(f, "Unregister fail"),
            BeatsError::MaximumCapacity => write!(f, "Max capacity reached"),
            BeatsError::MaximumConfusion => write!(f, "Unknown Error"),
        }
    }
}

impl From<IoError> for BeatsError {
    fn from(cause: IoError) -> BeatsError {
        BeatsError::Io(cause)
    }
}

impl From<Utf8Error> for BeatsError {
    fn from(cause: Utf8Error) -> BeatsError {
        BeatsError::Utf8(cause)
    }
}

impl From<SendError<DM2Deck>> for BeatsError {
    fn from(cause: SendError<DM2Deck>) -> BeatsError {
        BeatsError::DM2DeckSendFail(cause)
    }
}

impl From<RecvError> for BeatsError {
    fn from(cause: RecvError) -> BeatsError {
        BeatsError::ChannelRecvFail(cause)
    }
}

impl From<RecvTimeoutError> for BeatsError {
    fn from(cause: RecvTimeoutError) -> BeatsError {
        BeatsError::ChanRecvTimeout(cause)
    }
}
