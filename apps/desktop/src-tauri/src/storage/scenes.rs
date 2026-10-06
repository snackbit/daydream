use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRecorderSceneInput {
    pub canvas_height: i64,
    pub canvas_width: i64,
    pub desktop_audio_source_id: Option<String>,
    pub id: String,
    pub layout_mode: String,
    pub microphone_audio_source_id: Option<String>,
    pub name: String,
    pub sources: Vec<SaveRecorderSceneSourceInput>,
    pub timestamp_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRecorderSceneSourceInput {
    pub crop_bottom: i64,
    pub crop_left: i64,
    pub crop_right: i64,
    pub crop_top: i64,
    pub display_name: String,
    pub height: i64,
    pub id: String,
    pub preserve_aspect_ratio: bool,
    pub source_id: String,
    pub source_height: Option<i64>,
    pub source_kind: String,
    pub source_width: Option<i64>,
    pub width: i64,
    pub x: i64,
    pub y: i64,
    pub z_index: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecorderSceneRow {
    pub independent_video: bool,
    pub canvas_height: i64,
    pub canvas_width: i64,
    pub desktop_audio_source_id: Option<String>,
    pub id: String,
    pub layout_mode: String,
    pub microphone_audio_source_id: Option<String>,
    pub name: String,
    pub sources: Vec<RecorderSceneSourceRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecorderSceneSourceRow {
    pub crop_bottom: i64,
    pub crop_left: i64,
    pub crop_right: i64,
    pub crop_top: i64,
    pub display_name: String,
    pub height: i64,
    pub id: String,
    pub preserve_aspect_ratio: bool,
    pub source_id: String,
    pub source_height: Option<i64>,
    pub source_kind: String,
    pub source_width: Option<i64>,
    pub width: i64,
    pub x: i64,
    pub y: i64,
    pub z_index: i64,
}

pub fn save_recorder_scene(
    connection: &mut Connection,
    input: &SaveRecorderSceneInput,
) -> rusqlite::Result<RecorderSceneRow> {
    save_recorder_scene_with_mode(connection, input, false)
}

pub fn save_recorder_scene_with_mode(
    connection: &mut Connection,
    input: &SaveRecorderSceneInput,
    independent_video: bool,
) -> rusqlite::Result<RecorderSceneRow> {
    let transaction = connection.transaction()?;
    save_recorder_scene_rows(&transaction, input, independent_video)?;
    transaction.commit()?;
    get_recorder_scene(connection, &input.id)
}

pub fn save_recorder_scene_rows(
    transaction: &Connection,
    input: &SaveRecorderSceneInput,
    independent_video: bool,
) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO recorder_scenes (
            id,
            name,
            canvas_width,
            canvas_height,
            layout_mode,
            desktop_audio_source_id,
            microphone_audio_source_id,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            canvas_width = excluded.canvas_width,
            canvas_height = excluded.canvas_height,
            layout_mode = excluded.layout_mode,
            desktop_audio_source_id = excluded.desktop_audio_source_id,
            microphone_audio_source_id = excluded.microphone_audio_source_id,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.id,
            input.name,
            input.canvas_width,
            input.canvas_height,
            input.layout_mode,
            input.desktop_audio_source_id,
            input.microphone_audio_source_id,
            input.timestamp_ms,
        ],
    )?;

    transaction.execute(
        "DELETE FROM recorder_scene_sources WHERE scene_id = ?1",
        [&input.id],
    )?;

    for source in &input.sources {
        transaction.execute(
            r#"
            INSERT INTO recorder_scene_sources (
                id,
                scene_id,
                source_id,
                source_kind,
                display_name,
                x,
                y,
                width,
                height,
                source_width,
                source_height,
                crop_left,
                crop_top,
                crop_right,
                crop_bottom,
                z_index,
                preserve_aspect_ratio,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18)
            "#,
            params![
                source.id,
                input.id,
                source.source_id,
                source.source_kind,
                source.display_name,
                source.x,
                source.y,
                source.width,
                source.height,
                source.source_width,
                source.source_height,
                source.crop_left,
                source.crop_top,
                source.crop_right,
                source.crop_bottom,
                source.z_index,
                source.preserve_aspect_ratio,
                input.timestamp_ms,
            ],
        )?;
    }

    transaction.execute(
        "UPDATE recorder_scenes SET independent_video=?2 WHERE id=?1",
        params![input.id, independent_video],
    )?;
    Ok(())
}

pub fn get_recorder_scene(connection: &Connection, id: &str) -> rusqlite::Result<RecorderSceneRow> {
    let mut scene = connection.query_row(
        r#"
        SELECT
            id,
            name,
            canvas_width,
            canvas_height,
            layout_mode,
            desktop_audio_source_id,
            microphone_audio_source_id,
            independent_video
        FROM recorder_scenes
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(RecorderSceneRow {
                independent_video: row.get(7)?,
                id: row.get(0)?,
                name: row.get(1)?,
                canvas_width: row.get(2)?,
                canvas_height: row.get(3)?,
                layout_mode: row.get(4)?,
                desktop_audio_source_id: row.get(5)?,
                microphone_audio_source_id: row.get(6)?,
                sources: Vec::new(),
            })
        },
    )?;

    scene.sources = list_recorder_scene_sources(connection, id)?;

    Ok(scene)
}

pub fn get_latest_recorder_scene(
    connection: &Connection,
) -> rusqlite::Result<Option<RecorderSceneRow>> {
    let scene = connection
        .query_row(
            r#"
            SELECT
                id,
                name,
                canvas_width,
                canvas_height,
                layout_mode,
                desktop_audio_source_id,
                microphone_audio_source_id,
                independent_video
            FROM recorder_scenes
            ORDER BY updated_at_ms DESC, id ASC
            LIMIT 1
            "#,
            [],
            |row| {
                Ok(RecorderSceneRow {
                    independent_video: row.get(7)?,
                    id: row.get(0)?,
                    name: row.get(1)?,
                    canvas_width: row.get(2)?,
                    canvas_height: row.get(3)?,
                    layout_mode: row.get(4)?,
                    desktop_audio_source_id: row.get(5)?,
                    microphone_audio_source_id: row.get(6)?,
                    sources: Vec::new(),
                })
            },
        )
        .optional()?;

    scene
        .map(|mut scene| {
            scene.sources = list_recorder_scene_sources(connection, &scene.id)?;
            Ok(scene)
        })
        .transpose()
}

fn list_recorder_scene_sources(
    connection: &Connection,
    scene_id: &str,
) -> rusqlite::Result<Vec<RecorderSceneSourceRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            source_id,
            source_kind,
            display_name,
            x,
            y,
            width,
            height,
            source_width,
            source_height,
            crop_left,
            crop_top,
            crop_right,
            crop_bottom,
            z_index,
            preserve_aspect_ratio
        FROM recorder_scene_sources
        WHERE scene_id = ?1
        ORDER BY z_index ASC, id ASC
        "#,
    )?;

    let rows = statement.query_map([scene_id], |row| {
        Ok(RecorderSceneSourceRow {
            id: row.get(0)?,
            source_id: row.get(1)?,
            source_kind: row.get(2)?,
            display_name: row.get(3)?,
            x: row.get(4)?,
            y: row.get(5)?,
            width: row.get(6)?,
            height: row.get(7)?,
            source_width: row.get(8)?,
            source_height: row.get(9)?,
            crop_left: row.get(10)?,
            crop_top: row.get(11)?,
            crop_right: row.get(12)?,
            crop_bottom: row.get(13)?,
            z_index: row.get(14)?,
            preserve_aspect_ratio: row.get(15)?,
        })
    })?;

    rows.collect()
}
