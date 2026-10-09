//! Publication is conditional on every consumed input, including uncited input.
use super::generation::{GeneratedDay, Input, Profile};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

pub fn profile_is_current(connection: &Connection, profile: &Profile) -> Result<bool, String> {
    let json: Option<String> = connection
        .query_row(
            "SELECT value_json FROM app_settings WHERE key='ai.recall'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "recall_storage_failed")?;
    let settings: crate::settings::RecallSettings = match json {
        Some(json) => serde_json::from_str(&json).map_err(|_| "recall_settings_unavailable")?,
        None => Default::default(),
    };
    let fingerprint = hex::encode(Sha256::digest(
        serde_json::to_vec(&settings.generation).map_err(|_| "recall_profile_invalid")?,
    ));
    Ok(settings.generation.enabled && fingerprint == profile.fingerprint)
}

pub fn inputs(evidence: &[super::types::RecallEvidence]) -> Vec<Input> {
    evidence
        .iter()
        .map(|item| Input {
            id: item.citation.evidence_id.clone(),
            revision: item.citation.content_revision.clone(),
            start_ms: item.citation.absolute_start_ms,
            end_ms: item.citation.absolute_end_ms,
            text: item.text.clone(),
        })
        .collect()
}

pub struct PublicationContext<'a> {
    pub day_key: &'a str,
    pub job_id: &'a str,
    pub profile: &'a Profile,
    pub now_ms: i64,
}

pub fn publish(
    connection: &mut Connection,
    context: PublicationContext<'_>,
    consumed: &[Input],
    current: &[Input],
    output: &GeneratedDay,
) -> Result<(), String> {
    let PublicationContext {
        day_key,
        job_id,
        profile,
        now_ms,
    } = context;
    if super::generation::input_fingerprint(consumed)?
        != super::generation::input_fingerprint(current)?
    {
        return Err("recall_input_changed".into());
    }
    let tx = connection
        .transaction()
        .map_err(|_| "recall_storage_failed")?;
    if !profile_is_current(&tx, profile)? {
        return Err("recall_profile_changed".into());
    }
    let job: Option<(String, String)> = tx
        .query_row(
            "SELECT status,config_fingerprint FROM recall_jobs WHERE id=?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| "recall_storage_failed")?;
    if job != Some(("running".into(), profile.fingerprint.clone())) {
        return Err("recall_job_stale".into());
    }
    for input in consumed {
        let revision: Option<String> = tx
            .query_row(
                "SELECT revision FROM recall_evidence WHERE id=?1",
                [&input.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "recall_storage_failed")?;
        if revision.as_deref() != Some(input.revision.as_str()) {
            return Err("recall_input_changed".into());
        }
    }
    let fingerprint = super::generation::input_fingerprint(consumed)?;
    let json = serde_json::to_string(output).map_err(|_| "recall_output_invalid")?;
    tx.execute("INSERT INTO recall_generations(day_key,input_fingerprint,profile_fingerprint,output_json,status,updated_at) VALUES(?1,?2,?3,?4,'ready',?5) ON CONFLICT(day_key) DO UPDATE SET input_fingerprint=excluded.input_fingerprint,profile_fingerprint=excluded.profile_fingerprint,output_json=excluded.output_json,status='ready',updated_at=excluded.updated_at",params![day_key,fingerprint,profile.fingerprint,json,now_ms]).map_err(|_| "recall_storage_failed")?;
    tx.execute(
        "DELETE FROM recall_generation_inputs WHERE day_key=?1",
        [day_key],
    )
    .map_err(|_| "recall_storage_failed")?;
    for input in consumed {
        tx.execute(
            "INSERT INTO recall_generation_inputs(day_key,evidence_id,revision) VALUES(?1,?2,?3)",
            params![day_key, input.id, input.revision],
        )
        .map_err(|_| "recall_storage_failed")?;
    }
    tx.execute("UPDATE recall_jobs SET status='succeeded',progress=1,error_code=NULL,updated_at_ms=?2 WHERE id=?1",params![job_id,now_ms]).map_err(|_| "recall_storage_failed")?;
    tx.commit().map_err(|_| "recall_storage_failed".to_string())
}

pub fn load(
    connection: &Connection,
    day_key: &str,
    current: &[Input],
    _profile_fingerprint: Option<&str>,
) -> Result<Option<GeneratedDay>, String> {
    let stored: Option<(String,String,String)> = connection.query_row("SELECT input_fingerprint,profile_fingerprint,output_json FROM recall_generations WHERE day_key=?1 AND status='ready'",[day_key],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|_| "recall_storage_failed")?;
    let Some((fingerprint, _profile, json)) = stored else {
        return Ok(None);
    };
    let mut query = connection
        .prepare("SELECT evidence_id,revision FROM recall_generation_inputs WHERE day_key=?1")
        .map_err(|_| "recall_storage_failed")?;
    let lineage = query
        .query_map([day_key], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| "recall_storage_failed")?;
    let mut consumed = Vec::new();
    let current_by_id: std::collections::HashMap<_, _> = current
        .iter()
        .map(|input| (input.id.as_str(), input))
        .collect();
    for row in lineage {
        let (id, revision) = row.map_err(|_| "recall_storage_failed")?;
        let Some(input) = current_by_id
            .get(id.as_str())
            .filter(|input| input.revision == revision)
        else {
            return Ok(None);
        };
        let persisted = connection
            .query_row(
                "SELECT revision FROM recall_evidence WHERE id=?1",
                [&id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| "recall_storage_failed")?;
        if persisted.as_deref() != Some(revision.as_str()) {
            return Ok(None);
        }
        consumed.push(Input::clone(*input));
    }
    if consumed.is_empty() || fingerprint != super::generation::input_fingerprint(&consumed)? {
        return Ok(None);
    }
    Ok(Some(super::generation::validate_output(&json, &consumed)?))
}

pub fn is_fresh(
    connection: &Connection,
    day_key: &str,
    current: &[Input],
    profile: Option<&str>,
) -> Result<bool, String> {
    let stored=connection.query_row("SELECT input_fingerprint,profile_fingerprint FROM recall_generations WHERE day_key=?1 AND status='ready'",[day_key],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).optional().map_err(|_|"recall_storage_failed")?;
    let Some((input, config)) = stored else {
        return Ok(false);
    };
    Ok(input == super::generation::input_fingerprint(current)? && profile == Some(config.as_str()))
}

/// Synthetic only: provider tests never consume private recordings or feedback.
pub async fn test_provider(profile: &Profile) -> Result<(), String> {
    let fixture = vec![Input {
        id: "synthetic-evidence".into(),
        revision: "synthetic-1".into(),
        start_ms: 0,
        end_ms: 1000,
        text: "Synthetic test: a person reviewed a sample document.".into(),
    }];
    super::generation::generate(
        profile,
        &fixture,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .await
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::super::generation::{Citation, Statement};
    use super::*;
    fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE recall_evidence(id TEXT PRIMARY KEY,revision TEXT); CREATE TABLE recall_jobs(id TEXT PRIMARY KEY,status TEXT,config_fingerprint TEXT,progress REAL,error_code TEXT,updated_at_ms INTEGER); CREATE TABLE recall_generations(day_key TEXT PRIMARY KEY,input_fingerprint TEXT,profile_fingerprint TEXT,output_json TEXT,status TEXT,updated_at INTEGER); CREATE TABLE recall_generation_inputs(day_key TEXT,evidence_id TEXT,revision TEXT); INSERT INTO recall_evidence VALUES('cited','1'),('uncited','1'); INSERT INTO recall_jobs VALUES('job','running','profile',0,NULL,0);").unwrap();
        connection
            .execute_batch("CREATE TABLE app_settings(key TEXT PRIMARY KEY,value_json TEXT);")
            .unwrap();
        let settings = settings();
        connection
            .execute(
                "INSERT INTO app_settings VALUES('ai.recall',?1)",
                [serde_json::to_string(&settings).unwrap()],
            )
            .unwrap();
        let fingerprint = hex::encode(Sha256::digest(
            serde_json::to_vec(&settings.generation).unwrap(),
        ));
        connection
            .execute(
                "UPDATE recall_jobs SET config_fingerprint=?1",
                [fingerprint],
            )
            .unwrap();
        connection
    }
    fn settings() -> crate::settings::RecallSettings {
        let mut settings = crate::settings::RecallSettings::default();
        settings.generation.enabled = true;
        settings.generation.base_url = Some("http://localhost".into());
        settings.generation.model = Some("test".into());
        settings.generation.auth_mode = "none".into();
        settings
    }
    fn fixture() -> (Vec<Input>, GeneratedDay, Profile) {
        let inputs = ["cited", "uncited"]
            .iter()
            .map(|id| Input {
                id: (*id).into(),
                revision: "1".into(),
                start_ms: 0,
                end_ms: 100,
                text: "Synthetic".into(),
            })
            .collect();
        let output = GeneratedDay {
            summary: vec![Statement {
                id: "summary".into(),
                text: "Sample".into(),
                category: None,
                citations: vec![Citation {
                    evidence_id: "cited".into(),
                    start_ms: 0,
                    end_ms: 100,
                    quote: "Synthetic".into(),
                }],
            }],
            sessions: vec![],
            suggestions: vec![],
        };
        let profile = Profile {
            enabled: true,
            base_url: "http://localhost".into(),
            model: "test".into(),
            api_key: None,
            fingerprint: hex::encode(Sha256::digest(
                serde_json::to_vec(&settings().generation).unwrap(),
            )),
        };
        (inputs, output, profile)
    }
    #[test]
    fn uncited_change_rejects_late_publication_and_withholds_stored_output() {
        let mut connection = connection();
        let (inputs, output, profile) = fixture();
        publish(
            &mut connection,
            PublicationContext {
                day_key: "day",
                job_id: "job",
                profile: &profile,
                now_ms: 1,
            },
            &inputs,
            &inputs,
            &output,
        )
        .unwrap();
        assert!(
            load(&connection, "day", &inputs, Some(&profile.fingerprint))
                .unwrap()
                .is_some()
        );
        let mut current = inputs.clone();
        current[1].revision = "2".into();
        assert!(
            load(&connection, "day", &current, Some(&profile.fingerprint))
                .unwrap()
                .is_none()
        );
        connection
            .execute("UPDATE recall_jobs SET status='running'", [])
            .unwrap();
        connection
            .execute(
                "UPDATE recall_evidence SET revision='2' WHERE id='uncited'",
                [],
            )
            .unwrap();
        assert!(publish(
            &mut connection,
            PublicationContext {
                day_key: "day",
                job_id: "job",
                profile: &profile,
                now_ms: 2
            },
            &inputs,
            &inputs,
            &output,
        )
        .is_err());
    }
    #[test]
    fn cancellation_preserves_successful_generation() {
        let mut connection = connection();
        let (inputs, output, profile) = fixture();
        publish(
            &mut connection,
            PublicationContext {
                day_key: "day",
                job_id: "job",
                profile: &profile,
                now_ms: 1,
            },
            &inputs,
            &inputs,
            &output,
        )
        .unwrap();
        connection
            .execute("UPDATE recall_jobs SET status='cancelled'", [])
            .unwrap();
        assert!(publish(
            &mut connection,
            PublicationContext {
                day_key: "day",
                job_id: "job",
                profile: &profile,
                now_ms: 2
            },
            &inputs,
            &inputs,
            &output,
        )
        .is_err());
        assert!(
            load(&connection, "day", &inputs, Some(&profile.fingerprint))
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn appended_evidence_and_failed_regeneration_preserve_eligible_previous_output() {
        let mut connection = connection();
        let (inputs, output, profile) = fixture();
        publish(
            &mut connection,
            PublicationContext {
                day_key: "day",
                job_id: "job",
                profile: &profile,
                now_ms: 1,
            },
            &inputs,
            &inputs,
            &output,
        )
        .unwrap();
        connection
            .execute("INSERT INTO recall_evidence VALUES('appended','1')", [])
            .unwrap();
        let mut current = inputs.clone();
        current.push(Input {
            id: "appended".into(),
            revision: "1".into(),
            start_ms: 200,
            end_ms: 300,
            text: "New synthetic input".into(),
        });
        connection
            .execute(
                "UPDATE recall_jobs SET status='failed',error_code='recall_provider_network'",
                [],
            )
            .unwrap();
        assert!(
            load(&connection, "day", &current, Some(&profile.fingerprint))
                .unwrap()
                .is_some()
        );
        assert!(!is_fresh(&connection, "day", &current, Some(&profile.fingerprint)).unwrap());
        assert!(load(&connection, "day", &inputs, Some("new-profile"))
            .unwrap()
            .is_some());
        assert!(!is_fresh(&connection, "day", &inputs, Some("new-profile")).unwrap());
        connection
            .execute("UPDATE recall_generations SET status='invalidated'", [])
            .unwrap();
        assert!(
            load(&connection, "day", &current, Some(&profile.fingerprint))
                .unwrap()
                .is_none()
        );
    }
}
