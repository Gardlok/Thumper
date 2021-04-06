use std::thread;
use std::time::{SystemTime, Duration};
use std::sync::mpsc;

use influx_db_client::{
    Client, Point, Points, Value, Precision, point, points
};

use crate::{Result, DM2Deck, BE};

pub struct BeatData {
    name: String,
    time: SystemTime,
    offset: Duration,
    expected: Duration,
}


pub trait Report {
    pub fn init(&self);
    pub fn send(&self, beats: Vec<BeatData>);
    pub fn stop(&self);
}

pub struct Output<T: Report> {
    pub reports: Vec<Box<dyn T>>,
    pub rt_tx: mpsc::Sender<DM2Deck>,
}

impl<T> Output<T>
where 
    T: Report,
{
    pub fn new(rt_tx: mpsc::Sender<DM2Deck>) -> Self { 
        reports: Vec::new(),
        rt_tx,
    }
    pub fn add(&self, T) { reports.push(T) }
    pub fn run(&mut self) { 

        let rt_tx = self.rt_tx.clone();

        // It's time to consider async
        thread::spawn(move || {
            loop {
           
                if let Err(e) = rt_tx.send(DM2Deck::RequestAllRecords) {
                    return Err(BE::DM2DeckSendFail(e))
                }
                match self.rt_rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(DM2DJ::Records(Some(records))) => Ok(records),
                    Ok(_) => Err(BE::MaximumConfusion),
                    Err(e) => Err(BE::ChanRecvTimeout(e))
                }

                self.reports.iter()
                .for_each(|x| x.send())

            }
        });
        

    }
}


// ////////////////////////////////////////////////////////////////
// Outputs
// ///////////////////////////////////////////////////


pub struct InfluxDB {
    con: Option<Client>,
}

impl Report for InfluxDB {
    fn init(&self) {
        // default with "http://127.0.0.1:8086"
        self.con = Client::default();
    }
    fn send(&self, beats: Vec<BeatData>) {

        let mut points = Vec::new();

        for beat in beats.iter() {
            let point = Point::new("beat1")
                .add_tag("name", beat.name)
                .add_field("beatdur", beat.offset.as_secs());
            
            points.push(point);

        }

        // if Precision is None, the default is second
        // Multiple write
        self.con.write_points(points, Some(Precision::Seconds), None).await.unwrap();

    } 
    fn stop(&self) {}

}
