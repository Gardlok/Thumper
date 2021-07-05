
// #![allow(unused)]
use criterion::{BenchmarkGroup, BenchmarkId, Throughput};
use criterion::{criterion_group, criterion_main, Criterion};

// To not to
use criterion::*;

use beats::{TheDJ, Beat, BE, Record, output::Report};

use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::collections::HashMap;
// use std::result::Result::Err;

// use lazy_static::lazy_static;

// lazy_static! {
//     static ref SEC: Duration = {
//         Duration::from_secs(1)
//     };
// }

// const 1SEC: Duration = Duration::from_secs(1);

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
                let ts = beat.duration_since(UNIX_EPOCH).expect("Marty!").as_nanos() as i64;
                if beat > &latest_beat { latest_beat = beat.clone()};

                // Report beat as a timestamp

            }

            // Update the last beat record so we know where to pickup next iteration
            self.lrb_map.insert(record.id, latest_beat);
        }
        Ok(())
    } 
    fn end(&self) -> Result<(), BE> {Ok(())}
}



fn setup_and_single_beat(c: &mut Criterion) {

    let s = Duration::from_secs(1);
    let mut group = c.benchmark_group("First Group");

    // let dj = TheDJ::init().unwrap();
    // let beat = dj.register("BENCH1".to_string(), *SEC).unwrap();
    // let beat = dj.register("BENCH1".to_string(), s).unwrap();

    group.bench_function("Init DJ", |b| b.iter(|| { let _ = TheDJ::init(); }));
    // group.bench_function("Init Beat", |b| b.iter(|| { let _ = dj.register("benchinit".to_string(), *SEC); }));
    // group.bench_function("Init Beat", |b| b.iter(|| { let _ = dj.register("benchinit".to_string(), s); }));
    // group.bench_function("Send Single Beat", |b| b.iter(||  beat.now()));
    
    group.finish();
}




// fn bench_dj_and_beat_setup(c: &mut Criterion) {

//     // TODO: bench different amount of beat deployment

//     let func = || {
//         let dj = TheDJ::init().unwrap();  
//         let beat = dj.register("example".to_string(), *SEC).unwrap();
//     };

//     c.bench_function("single_beat", |b| b.iter(|| func()));
// }

fn single_beat(c: &mut Criterion) {
    let dj = TheDJ::init().unwrap();  
    let beat = dj.register("example".to_string(), Duration::from_secs(1)).unwrap();
    c.bench_function("single_beat", |b| b.iter(|| beat.now()));
}


criterion_group!(
    benches, 
    setup_and_single_beat,
    // single_beat,
);
criterion_main!(benches);
