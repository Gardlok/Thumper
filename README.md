# Thumper
Concurrent tasks' health monitoring 

or

End-point integrity validation

Determine if an asynchronous task is _ticking_ as it should be. This is an attempt to implement system health status monitoring. This collects and analyzes metrics delivered from ticks/beats that occur either in various parts of your program, system, and/or network. This is currently very alpha/experimental and should not be used in a critical production environment yet. Expect possible breaking changes frequently. 

Usage:
```rust
async fn example_foo(beat: Beat) {
  loop {
    beat.send()
    ...
  }
}

fn main() {
  let dj = TheDJ::init().unwrap()
  let beat = dj.register("example_foo", Duration::from_secs(1));
  example_foo(beat).await;
  println!("{:?}", dj.get_record(0).unwrap().get_activity_rating());
}
```

Status varients:
```rust
pub enum ActivityRating {
    Optimal,     // Within 2% of expected turn around
    NotOptimal,  // Beyond 2% difference in expected turn around
    OnlyOnce,    // Only one beat in records
    NotOnce,     // No beats
}
```
There is also a `record.is_optimal()` bool for quick checks.