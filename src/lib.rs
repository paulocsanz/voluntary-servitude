#[macro_use]
extern crate log;

macro_rules! crit {
    ($($x: expr),*) => {{
        error!("CRITICAL ERROR");
        error!($($x),*);
        debug_assert!(false, "This should never happen but it did, something is broken and should be fixed");
    }}
}

mod iter;
mod node;
mod types;
mod vsread;

pub use types::VSRead;
