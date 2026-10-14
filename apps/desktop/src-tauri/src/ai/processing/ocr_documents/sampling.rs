//! Adaptive v1 never substitutes old text for a different image. Omissions are gaps.
use super::super::visual::VisualImage;
use super::*;
use extraction::{FullFrame, FullFrameSet};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;

struct History {
    key: String,
    end: i64,
    selected: i64,
    hash: String,
    image: Arc<VisualImage>,
}
fn history() -> &'static Mutex<VecDeque<History>> {
    static HISTORY: OnceLock<Mutex<VecDeque<History>>> = OnceLock::new();
    HISTORY.get_or_init(|| Mutex::new(VecDeque::new()))
}
pub(in crate::ai) fn discard(jobs: &[String]) {
    history()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|entry| {
            !jobs
                .iter()
                .any(|job| entry.key.starts_with(&format!("{job}:")))
        });
}
pub(in crate::ai) struct Sample {
    pub frame: FullFrame,
    pub selected: bool,
}
impl Sample {
    pub(in crate::ai) fn should_recognize(&self, retry_timestamps: Option<&[i64]>) -> bool {
        self.selected
            || retry_timestamps
                .is_some_and(|timestamps| timestamps.contains(&self.frame.timestamp_ms))
    }
}

fn choose(time: i64, last: Option<i64>, exact: bool, changed: bool, observations: bool) -> bool {
    let Some(last) = last else {
        return true;
    };
    let elapsed = time.saturating_sub(last);
    if elapsed >= if observations { 60_000 } else { 30_000 } {
        return true;
    }
    if observations {
        elapsed >= 30_000 && changed
    } else {
        exact || changed
    }
}

pub(in crate::ai) fn select<R: tauri::Runtime>(
    app: &AppHandle<R>,
    chunk: &ProcessingChunkRow,
    frames: &FullFrameSet,
    fingerprint: &str,
    observations: bool,
    cancel: &AtomicBool,
) -> Result<Vec<Sample>, AiError> {
    let storage = app.state::<AppStorage>();
    let cuts = storage
        .with_connection(|connection| storage::list_redactions(connection, &chunk.recording_id))
        .map_err(storage_error)?;
    let revision = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(
                &cuts
                    .iter()
                    .map(|cut| (
                        &cut.id,
                        cut.start_offset_ms,
                        cut.end_offset_ms,
                        &cut.track_ids_json
                    ))
                    .collect::<Vec<_>>()
            )
            .map_err(|_| ai_error(
                "ai_processing_config_invalid",
                "Sampling configuration is invalid."
            ))?
        )
    );
    select_frames(chunk, frames, fingerprint, &revision, observations, cancel)
}
pub(super) fn select_frames(
    chunk: &ProcessingChunkRow,
    frames: &FullFrameSet,
    fingerprint: &str,
    revision: &str,
    observations: bool,
    cancel: &AtomicBool,
) -> Result<Vec<Sample>, AiError> {
    let key = format!(
        "{}:{}:{}:{fingerprint}:{revision}",
        chunk.job_id,
        chunk.track_id.as_deref().unwrap_or("legacy"),
        chunk.source_kind
    );
    let mut prior = {
        let mut entries = history().lock().map_err(|_| {
            ai_error(
                "ai_processing_worker_state_failed",
                "Sampling history unavailable.",
            )
        })?;
        entries
            .iter()
            .position(|entry| entry.key == key)
            .and_then(|index| entries.remove(index))
    };
    if prior
        .as_ref()
        .is_some_and(|prior| Some(prior.end) != chunk.start_offset_ms)
    {
        prior = None;
    }
    let mut samples = Vec::with_capacity(frames.len());
    for index in 0..frames.len() {
        check_processing_cancel(cancel)?;
        if observations
            && prior.as_ref().is_some_and(|prior| {
                frames.timestamp(index).saturating_sub(prior.selected) < 30_000
            })
        {
            let frame = frames.frame(index)?;
            if let Some(prior) = &mut prior {
                prior.end = frame.end_ms;
            }
            samples.push(Sample {
                frame,
                selected: false,
            });
            continue;
        }
        let (frame, image) = frames.decode(index)?;
        let exact = prior
            .as_ref()
            .is_some_and(|prior| prior.hash == frame.pixel_hash);
        let changed = !exact
            && prior
                .as_ref()
                .is_none_or(|prior| super::super::visual::material_change(&prior.image, &image));
        let selected = choose(
            frame.timestamp_ms,
            prior.as_ref().map(|prior| prior.selected),
            exact,
            changed,
            observations,
        );
        if selected && !exact || prior.is_none() {
            prior = Some(History {
                key: key.clone(),
                end: frame.end_ms,
                selected: frame.timestamp_ms,
                hash: frame.pixel_hash.clone(),
                image,
            });
        } else if let Some(prior) = &mut prior {
            prior.end = frame.end_ms;
            if selected {
                prior.selected = frame.timestamp_ms;
            }
        }
        samples.push(Sample { frame, selected });
    }
    if let Some(prior) = prior {
        let mut entries = history().lock().map_err(|_| {
            ai_error(
                "ai_processing_worker_state_failed",
                "Sampling history unavailable.",
            )
        })?;
        const HISTORY_BYTES: usize = 128 * 1024 * 1024;
        if prior.image.pixels.len() > HISTORY_BYTES {
            return Ok(samples);
        }
        while entries.len() >= 4
            || entries
                .iter()
                .map(|entry| entry.image.pixels.len())
                .sum::<usize>()
                + prior.image.pixels.len()
                > HISTORY_BYTES
        {
            entries.pop_front();
        }
        entries.push_back(prior);
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::processing::worker;
    #[test]
    fn explicit_retry_recognizes_checkpoint_even_when_reset_history_would_omit_it() {
        let storage = storage::test_storage(PathBuf::new());
        let mut chunk = storage
            .with_connection(|connection| {
                Ok(crate::ai::tests::test_transcription_chunk(connection).0)
            })
            .unwrap();
        chunk.source_kind = "ocr".into();
        chunk.start_offset_ms = Some(0);
        chunk.end_offset_ms = Some(30_000);
        select_frames(
            &chunk,
            &frames(0, &[0, 0, 0, 0, 0, 32]),
            "retry",
            "eligible",
            false,
            &AtomicBool::new(false),
        )
        .unwrap();
        chunk.start_offset_ms = Some(30_000);
        chunk.end_offset_ms = Some(60_000);
        let original = select_frames(
            &chunk,
            &frames(30_000, &[33, 34, 35, 36, 37, 38]),
            "retry",
            "eligible",
            false,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(original[5].selected, "55 seconds is the carried checkpoint");
        discard(std::slice::from_ref(&chunk.job_id));
        let restarted = select_frames(
            &chunk,
            &frames(30_000, &[33, 34, 35, 36, 37, 38]),
            "retry",
            "eligible",
            false,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(
            !restarted[5].selected,
            "restarting shifts the sampling checkpoint"
        );
        let failed_id = Uuid::now_v7().to_string();
        storage.with_connection(|connection| {
            connection.execute("UPDATE processing_chunks SET source_kind='ocr',start_offset_ms=30000,end_offset_ms=60000,status='failed' WHERE id=?1",[&chunk.id])?;
            connection.execute("INSERT INTO visual_frame_processing_states(id,chunk_id,job_id,recording_id,segment_id,source_kind,frame_timestamp_ms,source_frame_ref,region_x,region_y,region_width,region_height,status,progress,error_code,output_count,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,?5,'ocr',55000,'synthetic-checkpoint',0,0,64,64,'failed',0,'ai_ocr_tesseract_failed',0,1,1)",rusqlite::params![failed_id,chunk.id,chunk.job_id,chunk.recording_id,chunk.segment_id])?;
            Ok(())
        }).unwrap();
        worker::retry_processing_units_for_action(
            &storage,
            &worker::RetryProcessingUnitsInput {
                job_id: chunk.job_id.clone(),
                chunk_id: Some(chunk.id.clone()),
                unit_id: Some(failed_id),
                source_kind: Some("ocr".into()),
                use_current_settings: false,
            },
        )
        .unwrap();
        let selection = worker::load_unit_selection(&storage, &chunk.id).unwrap();
        assert_eq!(
            selection.frame_timestamps.as_deref(),
            Some([55_000].as_slice())
        );
        assert!(restarted[5].should_recognize(selection.frame_timestamps.as_deref()));
        assert!(!restarted[4].should_recognize(selection.frame_timestamps.as_deref()));
        assert!(!restarted[5].should_recognize(None));
        discard(std::slice::from_ref(&chunk.job_id));
    }
    fn frames(start: i64, images: &[usize]) -> FullFrameSet {
        let root =
            std::env::temp_dir().join(format!("daydream-sampling-synthetic-{}", Uuid::now_v7()));
        fs::create_dir_all(&root).unwrap();
        let paths = images
            .iter()
            .enumerate()
            .map(|(index, changed)| {
                let path = root.join(format!("{index}.png"));
                let mut pixels = vec![255; 64 * 64 * 4];
                for pixel in 0..*changed {
                    pixels[pixel * 4..pixel * 4 + 3].fill(0);
                }
                let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), 64, 64);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&pixels)
                    .unwrap();
                path
            })
            .collect();
        extraction::synthetic_frames(root, paths, start, start + images.len() as i64 * 5000, 0.2)
    }
    #[test]
    fn decoded_samples_carry_history_only_across_contiguous_eligible_configuration() {
        let storage = storage::test_storage(PathBuf::new());
        let mut chunk = storage
            .with_connection(|connection| {
                Ok(crate::ai::tests::test_transcription_chunk(connection).0)
            })
            .unwrap();
        chunk.source_kind = "ocr".into();
        chunk.start_offset_ms = Some(0);
        chunk.end_offset_ms = Some(10_000);
        let initial = select_frames(
            &chunk,
            &frames(0, &[0, 1]),
            "profile",
            "privacy-v1",
            false,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            initial
                .iter()
                .map(|sample| sample.selected)
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        chunk.start_offset_ms = Some(10_000);
        chunk.end_offset_ms = Some(15_000);
        assert!(
            !select_frames(
                &chunk,
                &frames(10_000, &[1]),
                "profile",
                "privacy-v1",
                false,
                &AtomicBool::new(false)
            )
            .unwrap()[0]
                .selected
        );
        chunk.start_offset_ms = Some(15_000);
        chunk.end_offset_ms = Some(20_000);
        assert!(
            select_frames(
                &chunk,
                &frames(15_000, &[16]),
                "profile",
                "privacy-v1",
                false,
                &AtomicBool::new(false)
            )
            .unwrap()[0]
                .selected
        );
        chunk.start_offset_ms = Some(20_000);
        chunk.end_offset_ms = Some(25_000);
        assert!(
            select_frames(
                &chunk,
                &frames(20_000, &[16]),
                "profile",
                "privacy-v2",
                false,
                &AtomicBool::new(false)
            )
            .unwrap()[0]
                .selected
        );
        chunk.start_offset_ms = Some(30_000);
        chunk.end_offset_ms = Some(35_000);
        assert!(
            select_frames(
                &chunk,
                &frames(30_000, &[1]),
                "profile",
                "privacy-v1",
                false,
                &AtomicBool::new(false)
            )
            .unwrap()[0]
                .selected
        );
        assert_eq!(
            select_frames(
                &chunk,
                &frames(30_000, &[1]),
                "profile",
                "privacy-v1",
                false,
                &AtomicBool::new(true)
            )
            .err()
            .unwrap()
            .code,
            "ai_processing_cancelled"
        );
        discard(std::slice::from_ref(&chunk.job_id));
    }
    #[test]
    fn adaptive_ocr_reuses_exact_pixels_and_checks_small_changes_and_checkpoints() {
        assert!(choose(0, None, false, false, false));
        assert!(choose(5000, Some(0), true, false, false));
        assert!(choose(5000, Some(0), false, true, false));
        assert!(!choose(5000, Some(0), false, false, false));
        assert!(choose(30_000, Some(0), false, false, false));
    }
    #[test]
    fn observations_have_independent_spacing_and_heartbeat() {
        assert!(choose(0, None, false, false, true));
        assert!(!choose(5000, Some(0), false, true, true));
        assert!(choose(30_000, Some(0), false, true, true));
        assert!(!choose(30_000, Some(0), true, false, true));
        assert!(choose(60_000, Some(0), true, false, true));
    }
}
