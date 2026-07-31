use core::sync::atomic::{AtomicU64, Ordering};

use crate::println;


const PIT_FREQUENCY_HZ: u64 = 1_193_182;
const PIT_DIVISOR: u64 = 65536;
static  TICKS: AtomicU64 = AtomicU64::new(0);

pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}



pub fn ticks_to_ms(ticks: u64) -> u64 {
    (ticks * PIT_DIVISOR * 1000) / PIT_FREQUENCY_HZ
}

pub fn uptime_ms() -> u64 {
    ticks_to_ms(ticks())
}