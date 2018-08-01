#[macro_use]
extern crate log;

macro_rules! crit {
    ($($msg:expr),*) => {{
        error!("CRITICAL ERROR");
        error!($($msg),*);
        debug_assert!(false, "Crashing in debug because this should never happen");
    }};
}

mod iter;
mod node;
mod types;
mod vsread;

pub use types::VSRead;
