#[macro_use]
extern crate lazy_static;

#[macro_export]
macro_rules! log {
    () => {
        print!("\n")
    };
    ($($arg:tt)*) => {{
        print!("[{}]: ", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
        println!($($arg)*);
    }};
}

mod cloud;
mod config;
mod executor;
mod utils;

use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() {
  let config = config::get();

  loop {
    let event = cloud::get_event(&config.queue_name).await;
    if event.is_some() {
      executor::handle_event(event.unwrap()).await;
    }
    thread::sleep(Duration::from_millis(config.interval));
  }
}
