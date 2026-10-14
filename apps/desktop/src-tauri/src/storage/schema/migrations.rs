use rusqlite::Connection;
use rusqlite_migration::Migrations;

mod adaptive_sampling;
mod ai_outputs;
mod audio_cleanup;
mod capture_runs;
mod initial;
mod live_processing;
mod model_transfers;
mod ocr_assets;
mod ocr_documents;
mod processing_overview;
mod processing_resilience;
mod processing_units;
mod protected_windows;
mod recall;
mod recall_speech;
mod recovery;
mod reprocessing;
mod scene_recording;
mod settings_processing;
mod source_video;
mod speech_languages;
mod speech_units;
mod studio_processing;
mod transcript_languages;
mod visual_context_lineage;
mod visual_frame_states;

pub(in crate::storage) fn apply_migrations(
    connection: &mut Connection,
) -> rusqlite_migration::Result<()> {
    // SQLite table rebuilds require this outside the migration transaction.
    // The new migration checks all references before committing.
    connection.pragma_update(None, "foreign_keys", "OFF")?;
    let result = migrations().to_latest(connection);
    connection.pragma_update(None, "foreign_keys", "ON")?;
    result
}

pub(crate) fn migrations() -> Migrations<'static> {
    let mut migrations = Vec::new();
    migrations.extend(initial::migrations());
    migrations.extend(scene_recording::migrations());
    migrations.extend(settings_processing::migrations());
    migrations.extend(ai_outputs::migrations());
    migrations.extend(ocr_assets::migrations());
    migrations.extend(protected_windows::migrations());
    migrations.extend(visual_frame_states::migrations());
    migrations.extend(ocr_documents::migrations());
    migrations.extend(processing_resilience::migrations());
    migrations.extend(processing_units::migrations());
    migrations.extend(live_processing::migrations());
    migrations.extend(source_video::migrations());
    migrations.extend(recall::migrations());
    migrations.extend(model_transfers::migrations());
    migrations.extend(recovery::migrations());
    migrations.extend(capture_runs::migrations());
    migrations.extend(recovery::output_ownership_migrations());
    migrations.extend(reprocessing::migrations());
    migrations.extend(transcript_languages::migrations());
    migrations.extend(speech_units::migrations());
    migrations.extend(recall_speech::migrations());
    migrations.extend(audio_cleanup::migrations());
    migrations.extend(studio_processing::migrations());
    migrations.extend(speech_languages::migrations());
    migrations.extend(processing_overview::migrations());
    migrations.extend(visual_context_lineage::migrations());
    migrations.extend(adaptive_sampling::migrations());
    Migrations::new(migrations)
}
