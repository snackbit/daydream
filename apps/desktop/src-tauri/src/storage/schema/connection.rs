use std::path::Path;

use rusqlite::Connection;

pub(in crate::storage) fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    enable_connection_pragmas(&connection)?;

    Ok(connection)
}

pub(in crate::storage) fn enable_connection_pragmas(
    connection: &Connection,
) -> rusqlite::Result<()> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;

    Ok(())
}
