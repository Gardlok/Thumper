# beats-lite
Concurrent tasks' health monitoring

Sometimes I've wanted to know if some asynchronous task is _ticking_ as it should be. This is an attempt to implement health status monitoring. This is currently very alpha/experimental and should not be used in a critical production environment yet. 

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

TODO:
- Implement allowing of user to configure the "CAP" defaults
- Implement allowing of user to configure acceptable threshold of
  beat frequency
