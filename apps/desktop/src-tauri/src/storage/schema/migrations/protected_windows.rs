use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
    CREATE TABLE recording_protection_spans (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        rule_id TEXT NOT NULL,
        rule_label TEXT NOT NULL,
        action TEXT NOT NULL CHECK (action IN ('pause', 'redact')),
        status TEXT NOT NULL CHECK (status IN ('open', 'closed', 'materialized')),
        start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
        end_offset_ms INTEGER CHECK (
            end_offset_ms IS NULL
            OR (
                end_offset_ms >= 0
                AND end_offset_ms >= start_offset_ms
            )
        ),
        opened_at_ms INTEGER NOT NULL CHECK (opened_at_ms >= 0),
        closed_at_ms INTEGER CHECK (closed_at_ms IS NULL OR closed_at_ms >= opened_at_ms),
        redaction_id TEXT,
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (redaction_id) REFERENCES redactions(id) ON DELETE SET NULL
    );

    CREATE INDEX idx_recording_protection_spans_recording_status
        ON recording_protection_spans(recording_id, status);
    CREATE INDEX idx_recording_protection_spans_recording_range
        ON recording_protection_spans(recording_id, start_offset_ms, end_offset_ms);
    "#,
    )]
}
