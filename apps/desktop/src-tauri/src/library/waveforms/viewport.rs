use super::{jobs, tiles::waveform_tile_bin_ms};
use crate::{
    library::{
        LibraryTrackKind, LibraryWaveformTileRequest, LibraryWaveformTileTrack,
        LibraryWaveformTiles,
    },
    storage::AppStorage,
};
use std::collections::BTreeMap;

pub(crate) fn read(
    storage: &AppStorage,
    input: &LibraryWaveformTileRequest,
) -> Result<LibraryWaveformTiles, String> {
    if !input.viewport_width_px.is_finite()
        || input.viewport_width_px <= 0.0
        || input.end_timeline_ms <= input.start_timeline_ms
    {
        return Err("Choose a valid waveform range.".into());
    }
    let (_, sources) = jobs::sources(storage, &input.recording_id)?;
    let bin_ms = waveform_tile_bin_ms(input);
    let mut scales = BTreeMap::<String, f64>::new();
    for source in &sources {
        if let Ok(maximum) = source.maximum() {
            let value = scales.entry(source.source_kind.clone()).or_default();
            *value = value.max(maximum);
        }
    }
    let mut result = LibraryWaveformTiles {
        pending: sources.iter().any(|source| source.maximum().is_err()),
        bin_ms,
        error: None,
        recording_id: input.recording_id.clone(),
        tracks: Vec::new(),
    };
    for role in ["desktop-audio", "microphone"] {
        let scale = scales.get(role).copied().unwrap_or(1.0).max(f64::EPSILON);
        let mut track = LibraryWaveformTileTrack {
            kind: if role == "microphone" {
                LibraryTrackKind::Microphone
            } else {
                LibraryTrackKind::DesktopAudio
            },
            scale_amplitude: scale,
            tiles: Vec::new(),
            track_id: role.into(),
        };
        for source in sources.iter().filter(|source| {
            source.source_kind == role
                && source.start_ms < input.end_timeline_ms
                && source.end_ms > input.start_timeline_ms
        }) {
            match source.tile(
                input.start_timeline_ms,
                input.end_timeline_ms,
                bin_ms,
                scale,
            ) {
                Ok(tile) => track.tiles.push(tile),
                Err(error) => {
                    if error == "waveform_cache_invalid" && source.path.is_file() {
                        std::fs::remove_file(&source.path).map_err(|_| {
                            "Waveform cache could not be repaired. Retry in Processing."
                        })?;
                    }
                    result.pending = true;
                    let failed = storage.with_connection(|connection| connection.query_row(
                        "SELECT EXISTS(SELECT 1 FROM waveform_sources s JOIN processing_chunks c ON c.id=s.chunk_id WHERE s.source_key=?1 AND c.status IN('failed','completed-with-errors'))",
                        [source.path.to_string_lossy().as_ref()], |row| row.get::<_, bool>(0),
                    )).unwrap_or(false);
                    if failed {
                        result.error = Some(
                            "Waveform preparation failed. Retry Waveforms in Processing.".into(),
                        );
                    }
                }
            }
        }
        if sources.iter().any(|source| source.source_kind == role) {
            result.tracks.push(track);
        }
    }
    if result.pending {
        storage.with_connection(|connection| connection.execute("INSERT INTO waveform_admission(recording_id) VALUES(?1) ON CONFLICT(recording_id) DO UPDATE SET generation=generation+1", [&input.recording_id]).map(|_| ())).map_err(|_| "Waveform preparation could not be queued.")?;
    }
    Ok(result)
}
