use rusqlite::Connection;
use rusqlite_migration::Migrations;

mod ai_outputs;
mod initial;
mod ocr_assets;
mod scene_recording;
mod settings_processing;

pub(in crate::storage) fn apply_migrations(
    connection: &mut Connection,
) -> rusqlite_migration::Result<()> {
    migrations().to_latest(connection)
}

pub(in crate::storage) fn migrations() -> Migrations<'static> {
    let mut migrations = Vec::new();
    migrations.extend(initial::migrations());
    migrations.extend(scene_recording::migrations());
    migrations.extend(settings_processing::migrations());
    migrations.extend(ai_outputs::migrations());
    migrations.extend(ocr_assets::migrations());
    Migrations::new(migrations)
}
