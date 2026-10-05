mod matching;
mod monitor;
mod persistence;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use monitor::{start_window_protection_monitor, WindowProtectionMonitor};
pub(crate) use persistence::close_open_protection_spans;
