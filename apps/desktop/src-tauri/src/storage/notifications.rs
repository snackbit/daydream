use rusqlite::{params, Connection, OptionalExtension, ToSql};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRow {
    pub actions_json: String,
    pub created_at_ms: i64,
    pub dismissed_at_ms: Option<i64>,
    pub id: String,
    pub message: String,
    pub source: String,
    pub status: String,
    pub title: String,
    pub topic: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveNotificationInput {
    pub actions_json: String,
    pub id: String,
    pub message: String,
    pub source: String,
    pub status: String,
    pub timestamp_ms: i64,
    pub title: String,
    pub topic: String,
}

pub fn save_notification(
    connection: &Connection,
    input: &SaveNotificationInput,
) -> rusqlite::Result<NotificationRow> {
    connection.execute(
        r#"
        INSERT INTO notifications (
            id,
            source,
            topic,
            status,
            title,
            message,
            actions_json,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
        ON CONFLICT(id) DO UPDATE SET
            source = excluded.source,
            topic = excluded.topic,
            status = excluded.status,
            title = excluded.title,
            message = excluded.message,
            actions_json = excluded.actions_json,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.id,
            input.source,
            input.topic,
            input.status,
            input.title,
            input.message,
            input.actions_json,
            input.timestamp_ms,
        ],
    )?;

    get_notification(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_notification(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<NotificationRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                source,
                topic,
                status,
                title,
                message,
                actions_json,
                created_at_ms,
                updated_at_ms,
                dismissed_at_ms
            FROM notifications
            WHERE id = ?1
            "#,
            [id],
            notification_row,
        )
        .optional()
}

pub fn list_notifications(connection: &Connection) -> rusqlite::Result<Vec<NotificationRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            source,
            topic,
            status,
            title,
            message,
            actions_json,
            created_at_ms,
            updated_at_ms,
            dismissed_at_ms
        FROM notifications
        ORDER BY updated_at_ms DESC, created_at_ms DESC
        "#,
    )?;

    let rows = statement.query_map([], notification_row)?;

    rows.collect()
}

pub fn dismiss_notification(
    connection: &Connection,
    id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    acknowledge_notifications(connection, &[id.to_string()], timestamp_ms)?;

    get_notification(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn acknowledge_notifications(
    connection: &Connection,
    ids: &[String],
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    let placeholders = (0..ids.len())
        .map(|index| format!("?{}", index + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let statement = format!(
        r#"
        UPDATE notifications
        SET dismissed_at_ms = COALESCE(dismissed_at_ms, ?1)
        WHERE dismissed_at_ms IS NULL
            AND id IN ({placeholders})
        "#,
    );
    let mut params: Vec<&dyn ToSql> = Vec::with_capacity(ids.len() + 1);
    params.push(&timestamp_ms);
    for id in ids {
        params.push(id);
    }

    connection.execute(&statement, params.as_slice())?;

    Ok(())
}

pub fn delete_notifications(connection: &Connection, ids: &[String]) -> rusqlite::Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }

    let placeholders = (0..ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let statement = format!(
        r#"
        DELETE FROM notifications
        WHERE id IN ({placeholders})
        "#,
    );
    let params = ids.iter().map(|id| id as &dyn ToSql).collect::<Vec<_>>();

    connection.execute(&statement, params.as_slice())
}

pub fn delete_all_notifications(connection: &Connection) -> rusqlite::Result<usize> {
    connection.execute("DELETE FROM notifications", [])
}

pub fn delete_notifications_older_than(
    connection: &Connection,
    cutoff_ms: i64,
) -> rusqlite::Result<usize> {
    connection.execute(
        r#"
        DELETE FROM notifications
        WHERE created_at_ms < ?1
        "#,
        [cutoff_ms],
    )
}

fn notification_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationRow> {
    Ok(NotificationRow {
        id: row.get(0)?,
        source: row.get(1)?,
        topic: row.get(2)?,
        status: row.get(3)?,
        title: row.get(4)?,
        message: row.get(5)?,
        actions_json: row.get(6)?,
        created_at_ms: row.get(7)?,
        updated_at_ms: row.get(8)?,
        dismissed_at_ms: row.get(9)?,
    })
}
