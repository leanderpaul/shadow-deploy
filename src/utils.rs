use std::process;

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

pub fn exit(msg: &str) -> ! {
    log!("{}", msg);
    process::exit(1);
}
