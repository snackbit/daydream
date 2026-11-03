//! A small foreground WAL pool, independent from the processing writer.
use rusqlite::{Connection, OpenFlags};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
    time::Duration,
};

#[derive(Default)]
pub(super) struct ReadConnections {
    slots: [Mutex<Option<Connection>>; 2],
    next: AtomicUsize,
}

impl ReadConnections {
    pub(super) fn read<T>(
        &self,
        path: &Path,
        callback: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let start = self.next.fetch_add(1, Ordering::Relaxed) % self.slots.len();
        let mut slot = None;
        for index in 0..self.slots.len() {
            if let Ok(available) = self.slots[(start + index) % self.slots.len()].try_lock() {
                slot = Some(available);
                break;
            }
        }
        let mut slot: MutexGuard<'_, Option<Connection>> = match slot {
            Some(slot) => slot,
            None => self.slots[start]
                .lock()
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
        };
        if slot.is_none() {
            let connection = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            connection.busy_timeout(Duration::from_millis(250))?;
            connection.pragma_update(None, "query_only", true)?;
            *slot = Some(connection);
        }
        callback(slot.as_ref().ok_or(rusqlite::Error::InvalidQuery)?)
    }
}
