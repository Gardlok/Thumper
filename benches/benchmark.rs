
use criterion::{BenchmarkGroup, BenchmarkId, Throughput};
use criterion::{criterion_group, criterion_main, Criterion};


use wings::{TheDJ, Beat, BE, Record, output::Report};

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
    fn duration(&self) -> Result<Duration, BE> {Ok(Duration::from_secs(1))}
    fn init(&self) -> Result<(), BE> {Ok(())}
    fn run(&mut self, record: &Record) -> Result<(), BE> {
        
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
    fn end(&self) -> Result<(), BE> {Ok(())}
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


// Finally, specify which benches to run
criterion_group!(
    benches, 
    beats,
);
criterion_main!(benches);
