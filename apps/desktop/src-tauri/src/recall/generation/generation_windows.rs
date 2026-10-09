//! Successful bounded windows survive transient failures and restarts.
use super::generation::{self, GeneratedDay, Input, Profile};
use crate::storage::AppStorage;
use rusqlite::{params, OptionalExtension};
use std::sync::atomic::{AtomicBool, Ordering};

pub async fn generate(
    storage: &AppStorage,
    day_key: &str,
    job_id: &str,
    profile: &Profile,
    inputs: &[Input],
    cancelled: &AtomicBool,
) -> Result<GeneratedDay, String> {
    if inputs.is_empty() {
        return Err("recall_no_evidence".into());
    }
    let mut windows = Vec::<Vec<Input>>::new();
    let mut current = Vec::new();
    let mut bytes = 0usize;
    let mut hour = None;
    for input in inputs {
        let size = serde_json::to_vec(input)
            .map_err(|_| "recall_input_invalid")?
            .len();
        if size > 110_000 {
            return Err("recall_input_limit_exceeded".into());
        }
        let input_hour = input.start_ms.div_euclid(60 * 60 * 1000);
        if (bytes + size > 30_000 || hour != Some(input_hour)) && !current.is_empty() {
            windows.push(std::mem::take(&mut current));
            if windows.len() >= 64 {
                return Err("recall_day_window_limit_exceeded".into());
            }
            bytes = 0;
        }
        current.push(input.clone());
        hour = Some(input_hour);
        bytes += size;
    }
    if !current.is_empty() {
        windows.push(current)
    }
    if windows.len() > 64 {
        return Err("recall_day_window_limit_exceeded".into());
    }
    let total = windows.len();
    let mut output = GeneratedDay {
        summary: vec![],
        sessions: vec![],
        suggestions: vec![],
    };
    for (index, window) in windows.into_iter().enumerate() {
        if cancelled.load(Ordering::Acquire) {
            return Err("recall_cancelled".into());
        }
        let fingerprint = generation::input_fingerprint(&window)?;
        let checkpoint=storage.with_connection(|connection| {
            crate::storage::refresh_evidence(connection)?;
            if cancelled.load(Ordering::Acquire) || !super::generation_store::profile_is_current(connection,profile).map_err(|_|rusqlite::Error::InvalidQuery)? {return Err(rusqlite::Error::InvalidQuery)}
            let running=connection.query_row("SELECT status FROM recall_jobs WHERE id=?1",[job_id],|row|row.get::<_,String>(0))?=="running";
            if !running {return Err(rusqlite::Error::InvalidQuery)}
            for input in &window {
                let revision=connection.query_row("SELECT revision FROM recall_evidence WHERE id=?1",[&input.id],|row|row.get::<_,String>(0)).optional()?;
                if revision.as_deref()!=Some(input.revision.as_str()){return Err(rusqlite::Error::InvalidQuery)}
            }
            connection.query_row("SELECT output_json FROM recall_generation_windows WHERE day_key=?1 AND window_key=?2 AND config_fingerprint=?3",params![day_key,fingerprint,profile.fingerprint],|row|row.get::<_,String>(0)).optional()
        }).map_err(|_|"recall_input_changed")?;
        let generated = match checkpoint {
            Some(json) => generation::validate_output(&json, &window)?,
            None => {
                let generated = generation::generate(profile, &window, cancelled).await?;
                let json =
                    serde_json::to_string(&generated).map_err(|_| "recall_output_invalid")?;
                let lineage = serde_json::to_string(
                    &window
                        .iter()
                        .map(|input| serde_json::json!({"id":input.id,"revision":input.revision}))
                        .collect::<Vec<_>>(),
                )
                .map_err(|_| "recall_input_invalid")?;
                storage.with_connection(|connection| {
                    crate::storage::refresh_evidence(connection)?;
                    if !super::generation_store::profile_is_current(connection,profile).map_err(|_|rusqlite::Error::InvalidQuery)? {return Err(rusqlite::Error::InvalidQuery)}
                    let running=connection.query_row("SELECT status FROM recall_jobs WHERE id=?1",[job_id],|row|row.get::<_,String>(0))?=="running";
                    if !running || cancelled.load(Ordering::Acquire){return Err(rusqlite::Error::InvalidQuery)}
                    for input in &window {
                        let revision=connection.query_row("SELECT revision FROM recall_evidence WHERE id=?1",[&input.id],|row|row.get::<_,String>(0)).optional()?;
                        if revision.as_deref()!=Some(input.revision.as_str()){return Err(rusqlite::Error::InvalidQuery)}
                    }
                    connection.execute("INSERT OR REPLACE INTO recall_generation_windows(day_key,window_key,config_fingerprint,output_json,lineage_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6)",params![day_key,fingerprint,profile.fingerprint,json,lineage,super::worker::now_ms()])?;
                    Ok(())
                }).map_err(|_|"recall_input_changed")?;
                generated
            }
        };
        merge_window(&mut output, generated, &fingerprint);
        let json = serde_json::to_string(&output).map_err(|_| "recall_output_invalid")?;
        if json.len() > 128_000 {
            return Err("recall_output_too_large".into());
        }
        let _=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET progress=?2,updated_at_ms=?3 WHERE id=?1 AND status='running'",params![job_id,(index+1) as f64/total.max(1) as f64,super::worker::now_ms()]));
    }
    generation::validate_output(
        &serde_json::to_string(&output).map_err(|_| "recall_output_invalid")?,
        inputs,
    )
}

fn merge_window(output: &mut GeneratedDay, mut window: GeneratedDay, fingerprint: &str) {
    use sha2::{Digest, Sha256};
    for (kind, statements) in [
        ("summary", &mut window.summary),
        ("session", &mut window.sessions),
        ("suggestion", &mut window.suggestions),
    ] {
        for statement in statements {
            let namespace = serde_json::to_vec(&(fingerprint, kind, &statement.id))
                .expect("string tuple serializes");
            statement.id = format!("{kind}:{}", hex::encode(Sha256::digest(namespace)));
        }
    }
    output.summary.extend(window.summary);
    output.sessions.extend(window.sessions);
    output.suggestions.extend(window.suggestions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn valid_windows_with_repeated_provider_ids_merge_to_unique_stable_ids() {
        let inputs = vec![
            Input {
                id: "first".into(),
                revision: "1".into(),
                start_ms: 0,
                end_ms: 10,
                text: "First synthetic input".into(),
            },
            Input {
                id: "second".into(),
                revision: "1".into(),
                start_ms: 20,
                end_ms: 30,
                text: "Second synthetic input".into(),
            },
        ];
        let window = |input: &Input| GeneratedDay {
            summary: vec![generation::Statement {
                id: "summary-1".into(),
                text: input.text.clone(),
                category: None,
                citations: vec![generation::Citation {
                    evidence_id: input.id.clone(),
                    start_ms: input.start_ms,
                    end_ms: input.end_ms,
                    quote: input.text.clone(),
                }],
            }],
            sessions: vec![],
            suggestions: vec![],
        };
        let mut output = GeneratedDay {
            summary: vec![],
            sessions: vec![],
            suggestions: vec![],
        };
        for input in &inputs {
            let parsed = generation::validate_output(
                &serde_json::to_string(&window(input)).unwrap(),
                std::slice::from_ref(input),
            )
            .unwrap();
            merge_window(
                &mut output,
                parsed,
                &generation::input_fingerprint(std::slice::from_ref(input)).unwrap(),
            );
        }
        assert_ne!(output.summary[0].id, output.summary[1].id);
        assert!(
            generation::validate_output(&serde_json::to_string(&output).unwrap(), &inputs).is_ok()
        );
        let mut repeated = GeneratedDay {
            summary: vec![],
            sessions: vec![],
            suggestions: vec![],
        };
        merge_window(
            &mut repeated,
            window(&inputs[0]),
            &generation::input_fingerprint(&inputs[..1]).unwrap(),
        );
        assert_eq!(output.summary[0].id, repeated.summary[0].id);
    }
    #[test]
    fn reuses_persisted_window_without_calling_provider_and_privacy_purges_checkpoint() {
        use crate::storage;
        let storage = storage::test_storage(std::path::PathBuf::from("/tmp/daydream-window-test"));
        let mut settings = crate::settings::RecallSettings::default();
        settings.generation.enabled = true;
        settings.generation.auth_mode = "none".into();
        settings.generation.base_url = Some("http://127.0.0.1:1".into());
        settings.generation.model = Some("synthetic".into());
        let profile = Profile {
            enabled: true,
            base_url: settings.generation.base_url.clone().unwrap(),
            model: "synthetic".into(),
            api_key: None,
            fingerprint: hex::encode(Sha256::digest(
                serde_json::to_vec(&settings.generation).unwrap(),
            )),
        };
        let (fixture, inputs, job) = storage
            .with_connection(|connection| {
                let fixture = storage::seed_recall_fixture(connection);
                storage::refresh_evidence(connection)?;
                let evidence = storage::list_evidence_range(connection, 0, i64::MAX)?;
                let inputs = super::super::generation_store::inputs(&evidence);
                crate::storage::save_app_setting(
                    connection,
                    &crate::storage::SaveAppSettingInput {
                        key: "ai.recall".into(),
                        value_json: serde_json::to_string(&settings).unwrap(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                )?;
                let fingerprint = generation::input_fingerprint(&inputs).unwrap();
                let job = super::super::jobs::enqueue(
                    connection,
                    "day",
                    &fingerprint,
                    &profile.fingerprint,
                    1,
                )
                .unwrap();
                connection.execute(
                    "UPDATE recall_jobs SET status='running' WHERE id=?1",
                    [&job],
                )?;
                let first = &inputs[0];
                let output = GeneratedDay {
                    summary: vec![generation::Statement {
                        id: "summary".into(),
                        text: "Synthetic saved window".into(),
                        category: None,
                        citations: vec![generation::Citation {
                            evidence_id: first.id.clone(),
                            start_ms: first.start_ms,
                            end_ms: first.end_ms,
                            quote: first.text.clone(),
                        }],
                    }],
                    sessions: vec![],
                    suggestions: vec![],
                };
                let lineage = serde_json::to_string(
                    &inputs
                        .iter()
                        .map(|input| serde_json::json!({"id":input.id,"revision":input.revision}))
                        .collect::<Vec<_>>(),
                )
                .unwrap();
                connection.execute(
                    "INSERT INTO recall_generation_windows VALUES('day',?1,?2,?3,?4,1)",
                    params![
                        fingerprint,
                        profile.fingerprint,
                        serde_json::to_string(&output).unwrap(),
                        lineage
                    ],
                )?;
                Ok((fixture, inputs, job))
            })
            .unwrap();
        let output = tauri::async_runtime::block_on(generate(
            &storage,
            "day",
            &job,
            &profile,
            &inputs,
            &AtomicBool::new(false),
        ))
        .unwrap();
        assert_eq!(output.summary[0].text, "Synthetic saved window");
        storage
            .with_connection(|connection| {
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: uuid::Uuid::now_v7().to_string(),
                        recording_id: fixture.recording.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 5000,
                        track_ids_json: format!("[\"{}\"]", fixture.track),
                        created_at_ms: 2,
                        reason: None,
                    },
                )?;
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM recall_generation_windows",
                        [],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0
                );
                Ok(())
            })
            .unwrap();
        assert!(tauri::async_runtime::block_on(generate(
            &storage,
            "day",
            &job,
            &profile,
            &inputs,
            &AtomicBool::new(false)
        ))
        .is_err());
    }
}
