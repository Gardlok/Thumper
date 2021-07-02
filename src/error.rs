use std::result;
use thiserror::Error;

use crate::{DM2OutputRunner, DM2Deck};
pub use BeatsError as BE;

pub type Result<T> = result::Result<T, BE>;

#[derive(Error, Debug)]
pub enum BeatsError {

    #[error(transparent)]
    IOError(std::io::Error),

    #[error("Standard utf8 Error")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("MPSC send error: DM2Deck")]
    DM2DeckSendFail(#[from] std::sync::mpsc::SendError<DM2Deck>),

    #[error("MPSC send error: DM2OutputRunner")]
    DM2OutputRunner(#[from] std::sync::mpsc::SendError<DM2OutputRunner>),

    #[error("MPSC recv error")]
    ChannelRecvFail(#[from] std::sync::mpsc::RecvError),

    #[error("MPSC recv timeout error")]
    ChanRecvTimeout (#[from] std::sync::mpsc::RecvTimeoutError),

    #[error("The roster requested is empty")]
    EmptyRoster,

    #[error("The record requested does not exist")]
    MissingRecord,

    #[error("Record registration failure: {0}")]
	RegisterFail(&'static str),

    #[error("There are no new records to report")]
	NothingNewToReport,

    #[error("Failed to unregister the record")]
	UnregisterFail,

    // When the indexer has exhausted all available ids
    #[error("Maximum capacity reached")]
    MaximumCapacity,

    #[error(transparent)]
    EnvVarFail(#[from] std::env::VarError),

    #[error(transparent)]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    // When things get bad. Maybe received a response out of order
    #[error("Maximum Confusion")]
	MaximumConfusion,
    
}