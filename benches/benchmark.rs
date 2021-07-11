
use criterion::{BenchmarkGroup, BenchmarkId, Throughput};
use criterion::{criterion_group, criterion_main, Criterion};

use thumper::{TheDJ, Beat, TE, Record, output::Report, BEAT_CAP, RECORD_CAP};

use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::collections::HashMap;

// Duration of a second seems to be popular
use lazy_static::lazy_static;
lazy_static! {
    static ref SEC: Duration = {
        Duration::from_secs(1)
    };
}

// Not yet implemented. This is to be used to benchmark the  output system
#[derive(Debug)]
pub struct BenchOutput {
    lrb_map: HashMap<i32, SystemTime>
}

impl Report for BenchOutput {
    fn duration(&self) -> Result<Duration, TE> {Ok(Duration::from_secs(1))}
    fn init(&self) -> Result<(), TE> {Ok(())}
    fn run(&mut self, record: &Record) -> Result<(), TE> {
        
        let time_since = self.lrb_map.entry(record.id).or_insert(UNIX_EPOCH) ;
        if let Some(beats) = record.get_beats(Some(time_since)) {
            let mut latest_beat = UNIX_EPOCH;
            for beat in beats {
                // let ts = beat.duration_since(UNIX_EPOCH).expect("Marty!").as_nanos() as i64;
                if beat > &latest_beat { latest_beat = beat.clone()};

                // Report beat as a timestamp
                unimplemented!();
            }

            // Update the last beat record so we know where to pickup next iteration
            self.lrb_map.insert(record.id, latest_beat);
        }
        Ok(())
    } 
    fn end(&self) -> Result<(), TE> { Ok(())}
}

// Bench group that focuses on the beat operations 
fn beats(c: &mut Criterion) {

    let mut group = c.benchmark_group("Beats");
    let dj = TheDJ::init().unwrap();  
    
    // Single beat /////////////////////////////////////////////////////
    let beat = dj.register("SingleBeatBench".to_string(), *SEC).unwrap();
    group.bench_function("single_beat", |b| b.iter(|| beat.now()));
    let _ = dj.clear_all();

    // Many beats ///////////////////////////////////////////////////////

    // Config
    let count = 99;
    let timeout = Duration::from_secs(120);

    // Set up    
    let mut beats: Vec<Beat> = Vec::new();
    for n in 0..count {
        beats.push(dj.register(format!("BenchTest#{}", n), *SEC).unwrap());
    }

    // Execute
    let func = || {
        beats.iter().for_each(|b| { let _ = b.now(); });
        dj.block_for_beats(count, timeout).expect("not blocked");
    };
    group.bench_function("many_beats", |b| b.iter(|| func()));
}


fn record(c: &mut Criterion) {

    let mut group = c.benchmark_group("Record");
    let dj = TheDJ::init().unwrap();  
    let beat = dj.register("RecordBench".to_string(), *SEC).unwrap();
    let id = beat.id;

    // Bench populate the record's track with beats ///////////////////////
    let populate = || {
        for i in 0..BEAT_CAP {
            let t = SystemTime::now().checked_add(*SEC * i as u32).unwrap();
            let _ = beat.from(t);
        }
    };
    group.bench_function("record_populating", |b| b.iter(|| populate));
    // let _ = dj.clear_all();

    // Bench simple record retrieval through the dj /////////////////////////////////////
    let retrieve = || {
        let _ = dj.get_record(id);
    };
    group.bench_function("record_retrieval", |b| b.iter(|| retrieve()));

    // From this point use the same one record object ///////////////////////////////////
    let record = dj.get_record(id).unwrap(); 

    // Bench verifying whether the record has been updated or not ///////////////////////
    let has_updated = || {
        // by not passing in the SystemTime, it will generate it's own.
        let _ = record.has_updated_since(None);
    };
    group.bench_function("has_updated", |b| b.iter(|| has_updated()));

    // Bench is_optimal, which covers get_activity_rating, and get_average //////////////
    let is_optimal = || {
        let _ = record.is_optimal();
    };
    group.bench_function("is_optimal", |b| b.iter(|| is_optimal()));

    // Benjh get_roster_activies ////////////////////////////////////////////////////////
    let get_roster_actives = || {
        let _ = dj.get_roster_actives();
    };
    group.bench_function("get_roster_actives", |b| b.iter(|| get_roster_actives()));

    // Bench regristation and unregistration ///////////////////////////////////////////
    let reg_to_unreg = || {
        let r = dj.register("reg_to_unreg".to_string(), *SEC);
        let _ = dj.unregister(r.unwrap().id);
    };
    group.bench_function("reg_to_unreg", |b| b.iter(|| reg_to_unreg()));

    ////////////////////////////////////////////////////////////////////////////////////
    ////////////////////////////////////////////////////////////////////////////////////
}


// Finally, specify which benches to run
criterion_group!(
    benches, 
    beats,
    record,
);
criterion_main!(benches);
