use super::*;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResizeLibraryRedactionInput {
    recording_id: String,
    redaction_id: String,
    start_offset_ms: i64,
    end_offset_ms: i64,
    expected_start_offset_ms: i64,
    expected_end_offset_ms: i64,
}

fn resize(
    connection: &mut rusqlite::Connection,
    input: &ResizeLibraryRedactionInput,
    duration: i64,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    if input.start_offset_ms < 0
        || input.end_offset_ms > duration
        || input.end_offset_ms <= input.start_offset_ms
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let changed = transaction.execute("UPDATE redactions SET start_offset_ms=?1,end_offset_ms=?2 WHERE id=?3 AND recording_id=?4 AND start_offset_ms=?5 AND end_offset_ms=?6",
        rusqlite::params![input.start_offset_ms,input.end_offset_ms,input.redaction_id,input.recording_id,input.expected_start_offset_ms,input.expected_end_offset_ms])?;
    if changed != 1 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    transaction.commit()
}

#[tauri::command(rename = "resizeLibraryRedaction")]
pub(crate) async fn resize_library_redaction(
    app: tauri::AppHandle,
    input: ResizeLibraryRedactionInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-redaction")?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|_| "Recording could not be read.")?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|_| "Recording segments could not be read.")?;
    let duration = timeline_duration_ms(&segments);
    if input.start_offset_ms < 0
        || input.end_offset_ms > duration
        || input.end_offset_ms <= input.start_offset_ms
    {
        return Err("Redaction boundaries must be within the recording.".into());
    }
    crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
        .map_err(|_| "Processing could not be stopped safely.")?;
    let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
    storage
        .with_connection_mut(|connection| resize(connection, &input, duration))
        .map_err(|_| "The redaction changed or could not be saved. Refresh and try again.")?;
    ai::emit_processing_state(&app, &recording.id);
    detail_for_recording(storage.inner(), &recording.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resizing_is_atomic_and_rejects_stale_or_invalid_bounds() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE redactions(id TEXT,recording_id TEXT,start_offset_ms INTEGER,end_offset_ms INTEGER,reason TEXT);INSERT INTO redactions VALUES('cut','recording',100,400,NULL);").unwrap();
        let mut input = ResizeLibraryRedactionInput {
            recording_id: "recording".into(),
            redaction_id: "cut".into(),
            start_offset_ms: 150,
            end_offset_ms: 600,
            expected_start_offset_ms: 100,
            expected_end_offset_ms: 400,
        };
        resize(&mut connection, &input, 1000).unwrap();
        assert!(resize(&mut connection, &input, 1000).is_err());
        input.expected_start_offset_ms = 150;
        input.expected_end_offset_ms = 600;
        input.end_offset_ms = 1200;
        assert!(resize(&mut connection, &input, 1000).is_err());
        let bounds: (i64, i64) = connection
            .query_row(
                "SELECT start_offset_ms,end_offset_ms FROM redactions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(bounds, (150, 600));
    }
}
