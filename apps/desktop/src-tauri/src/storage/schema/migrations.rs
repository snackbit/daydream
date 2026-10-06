use rusqlite::Connection;
use rusqlite_migration::Migrations;

mod ai_outputs;
mod initial;
mod live_processing;
mod ocr_assets;
mod ocr_documents;
mod processing_resilience;
mod processing_units;
mod protected_windows;
mod scene_recording;
mod settings_processing;
mod source_video;
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

pub(in crate::storage) fn migrations() -> Migrations<'static> {
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
    Migrations::new(migrations)
}
