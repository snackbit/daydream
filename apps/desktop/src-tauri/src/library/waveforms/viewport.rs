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
    let bin_ms = waveform_tile_bin_ms(input);
    let mut scales=storage.with_read_connection(|c|{
        let mut q=c.prepare("SELECT source_kind,MAX(maximum) FROM waveform_cache_entries WHERE recording_id=?1 GROUP BY source_kind")?;
        let rows=q.query_map([&input.recording_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,f64>(1)?)))?.collect::<rusqlite::Result<BTreeMap<_,_>>>();rows
    }).map_err(|_|"Waveform metadata could not be read.")?;
    let catalog_roles = scales
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let use_catalog = !scales.is_empty();
    let (_, sources) = if use_catalog {
        jobs::sources_in_range(
            storage,
            &input.recording_id,
            input.start_timeline_ms,
            input.end_timeline_ms,
        )?
    } else {
        jobs::sources(storage, &input.recording_id)?
    };
    let mut pending = false;
    for source in &sources {
        if let Ok(maximum) = source.maximum() {
            if !catalog_roles.contains(&source.source_kind) {
                let value = scales.entry(source.source_kind.clone()).or_default();
                *value = value.max(maximum);
            }
        } else if source.start_ms < input.end_timeline_ms && source.end_ms > input.start_timeline_ms
        {
            pending = true;
        }
    }
    let mut result = LibraryWaveformTiles {
        pending,
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
                    let repair = if error == "waveform_pyramid_invalid" {
                        Some(source.path.with_extension("pyramid-v1"))
                    } else if error == "waveform_cache_invalid" {
                        Some(source.path.clone())
                    } else {
                        None
                    };
                    if let Some(path) = repair.filter(|p| p.is_file()) {
                        std::fs::remove_file(&path).map_err(|_| {
                            "Waveform cache could not be repaired. Retry in Processing."
                        })?;
                        if error == "waveform_cache_invalid" {
                            let _ = std::fs::remove_file(source.path.with_extension("pyramid-v1"));
                        }
                    }
                    result.pending = true;
                    let failed = storage.with_read_connection(|connection| connection.query_row(
                        "SELECT EXISTS(SELECT 1 FROM waveform_sources s JOIN processing_chunks c ON c.id=s.chunk_id JOIN processing_jobs j ON j.id=c.job_id WHERE s.source_key=?1 AND (c.status IN('failed','completed-with-errors') OR j.status IN('paused','cancelled')))",
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
    if result.pending || sources.iter().any(|source| !super::pyramid::ready(source)) {
        super::admission_requests::request(storage, &input.recording_id)?;
    }
    Ok(result)
}
