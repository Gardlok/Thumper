
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::env;
use std::collections::HashMap;

use reqwest::header::{HeaderValue, AUTHORIZATION};

use crate::{TE, Record, output::Report};

// InfluxDB //////////////////////////////////////////
// After trying:
// https://docs.rs/influxdb/0.4.0/influxdb/
// https://docs.rs/influx_db_client/0.5.0/influx_db_client/
// with too much trouble from each, I decided to implement a simple
// reqwest solution probably as a temp solution untill a better
// option presents itself.
#[derive(Debug)]
pub struct InfluxDB {
    address: String,
    name: String,
    auth: String,
    org: String,
    bucket: String,
    lrb_map: HashMap<i32, SystemTime>
}

impl InfluxDB {
    pub fn new(address: String, name: String) -> Result<InfluxDB, TE> {
        Ok(InfluxDB{
            address,
            name, 
            auth: env::var("B_TOKEN")?,
            org: env::var("B_ORG")?, 
            bucket: env::var("B_BUCKET")?, 
            lrb_map: HashMap::new(),
        })
    }
}

impl Report for InfluxDB {
    fn duration(&self) -> Result<Duration, TE> {Ok(Duration::from_secs(1))}
    fn init(&self) -> Result<(), TE> {Ok(())}
    fn run(&mut self, record: &Record) -> Result<(), TE> {
        let addy = format!("{}/api/v2/write?org={}&bucket={}", &self.address, &self.org, &self.bucket);
        let header_value = HeaderValue::from_str(&format!("Token {}", &self.auth))?;
        
        let time_since = self.lrb_map.entry(record.id).or_insert(UNIX_EPOCH) ;

        if let Some(beats) = record.get_beats(Some(time_since)) {
            let mut latest_beat = UNIX_EPOCH;
            // Syntax <measurement>[,<tag_key>=<tag_value>[,<tag_key>=<tag_value>]] <field_key>=<field_value>[,<field_key>=<field_value>] [<timestamp>]
            for beat in beats {
                // TODO: Do something else if fails
                let ts = beat.duration_since(UNIX_EPOCH).expect("Marty!").as_nanos() as i64;
                let msg = format!("{},beatname={} expected={} {}", self.name, record.name, record.freq.as_secs(), ts);
                let client = reqwest::blocking::Client::new();
                client.post(&addy)
                    .body(msg)
                    .header(AUTHORIZATION, &header_value)
                    .send()?;
                if beat > &latest_beat { latest_beat = beat.clone()};
            }

            // Update the last beat record so we know where to pickup next iteration
            self.lrb_map.insert(record.id, latest_beat);
        }
        Ok(())
    } 
    fn end(&self) -> Result<(), TE> {Ok(())}
}
