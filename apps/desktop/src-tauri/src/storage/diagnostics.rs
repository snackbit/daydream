use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;

use super::{path_to_string, schema::CORE_TABLES, AppStorage};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseDiagnostics {
    available: bool,
    error: Option<String>,
    path: String,
    schema_version: Option<i64>,
    table_counts: BTreeMap<String, i64>,
}

pub fn collect_database_diagnostics(storage: &AppStorage) -> DatabaseDiagnostics {
    match storage.connection.lock() {
        Ok(connection) => match collect_database_diagnostics_from_connection(&connection) {
            Ok((schema_version, table_counts)) => DatabaseDiagnostics {
                available: true,
                error: None,
                path: path_to_string(&storage.path),
                schema_version: Some(schema_version),
                table_counts,
            },
            Err(error) => DatabaseDiagnostics {
                available: false,
                error: Some(error.to_string()),
                path: path_to_string(&storage.path),
                schema_version: None,
                table_counts: BTreeMap::new(),
            },
        },
        Err(error) => DatabaseDiagnostics {
            available: false,
            error: Some(format!("Database connection lock is poisoned: {error}")),
            path: path_to_string(&storage.path),
            schema_version: None,
            table_counts: BTreeMap::new(),
        },
    }
}

pub(super) fn collect_database_diagnostics_from_connection(
    connection: &Connection,
) -> rusqlite::Result<(i64, BTreeMap<String, i64>)> {
    let schema_version = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let mut table_counts = BTreeMap::new();

    for table in CORE_TABLES {
        table_counts.insert(table.to_string(), count_table_rows(connection, table)?);
    }

    Ok((schema_version, table_counts))
}

fn count_table_rows(connection: &Connection, table: &str) -> rusqlite::Result<i64> {
    let statement = format!("SELECT COUNT(*) FROM {table}");

    connection.query_row(&statement, [], |row| row.get(0))
}
