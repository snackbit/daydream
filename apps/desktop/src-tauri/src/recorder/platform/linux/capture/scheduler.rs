use super::*;

pub(super) struct PipeWireCaptureSchedulerState {
    pub(super) benchmark: Option<RecorderBenchmarkSession>,
    pub(super) composed_frames: FrameSender,
    pub(super) frame_interval_ms: u64,
    pub(super) gpu_compositor: Option<LinuxGpuFrameCompositor>,
    pub(super) latest_frames: SharedPipeWireFrameSampler,
    pub(super) next_composed_at_ms: Option<u64>,
    pub(super) plan: SceneCompositorPlan,
    pub(super) scheduled_frame_log: PipeWireScheduledFrameLog,
    pub(super) timing: SharedCaptureTiming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ComposedFrameSendOutcome {
    Sent,
    Stopped,
}

pub(super) fn run_pipewire_capture_scheduler(
    plan: SceneCompositorPlan,
    fps: f64,
    composed_frames: FrameSender,
    timing: SharedCaptureTiming,
    latest_frames: SharedPipeWireFrameSampler,
    benchmark: Option<RecorderBenchmarkSession>,
    shutdown_receiver: mpsc::Receiver<()>,
) -> Result<(), RecorderError> {
    let frame_interval_ms = frame_interval_ms(fps);
    let mut state = PipeWireCaptureSchedulerState {
        benchmark,
        composed_frames,
        frame_interval_ms,
        gpu_compositor: None,
        latest_frames,
        next_composed_at_ms: None,
        plan,
        scheduled_frame_log: PipeWireScheduledFrameLog::new(Instant::now()),
        timing,
    };
    let idle_sleep = Duration::from_millis(frame_interval_ms.clamp(1, 50));

    loop {
        if scheduler_shutdown_requested(&shutdown_receiver) {
            return Ok(());
        }

        let now = now_ms();
        let Some(timestamp_ms) = next_scheduler_timestamp(&mut state, now)? else {
            match shutdown_receiver.recv_timeout(idle_sleep) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
            }
        };
        if scheduler_shutdown_requested(&shutdown_receiver) {
            return Ok(());
        }
        if compose_latest_pipewire_frames(&mut state, timestamp_ms, &shutdown_receiver)?
            == ComposedFrameSendOutcome::Stopped
        {
            return Ok(());
        }
    }
}

fn scheduler_shutdown_requested(shutdown_receiver: &mpsc::Receiver<()>) -> bool {
    matches!(
        shutdown_receiver.try_recv(),
        Ok(()) | Err(mpsc::TryRecvError::Disconnected)
    )
}

pub(super) fn next_scheduler_timestamp(
    state: &mut PipeWireCaptureSchedulerState,
    now_ms: u64,
) -> Result<Option<u64>, RecorderError> {
    let has_frames = state
        .latest_frames
        .lock()
        .map_err(|_error| capture_adapter_failed_error("PipeWire capture sampler lock poisoned."))?
        .has_frames();
    if !has_frames {
        return Ok(None);
    }

    let Some(next_composed_at_ms) = state.next_composed_at_ms else {
        state.next_composed_at_ms = Some(now_ms.saturating_add(state.frame_interval_ms));
        record_scheduler_tick(&state.timing, 0);
        return Ok(Some(now_ms));
    };
    if now_ms < next_composed_at_ms {
        return Ok(None);
    }

    let late_by_ms = now_ms.saturating_sub(next_composed_at_ms);
    let missed_deadlines = if late_by_ms > state.frame_interval_ms {
        late_by_ms / state.frame_interval_ms
    } else {
        0
    };
    let timestamp_ms = if late_by_ms > state.frame_interval_ms {
        now_ms
    } else {
        next_composed_at_ms
    };
    state.next_composed_at_ms = Some(timestamp_ms.saturating_add(state.frame_interval_ms));
    record_scheduler_tick(&state.timing, missed_deadlines);

    Ok(Some(timestamp_ms))
}

pub(super) fn compose_latest_pipewire_frames(
    state: &mut PipeWireCaptureSchedulerState,
    timestamp_ms: u64,
    shutdown_receiver: &mpsc::Receiver<()>,
) -> Result<ComposedFrameSendOutcome, RecorderError> {
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    let snapshot_started = Instant::now();
    let latest_snapshot = latest_pipewire_frame_snapshot(&state.latest_frames)?;
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::Snapshot,
            snapshot_started.elapsed(),
            Some(timestamp_ms),
            Some(latest_snapshot.len()),
            latest_snapshot
                .values()
                .map(|frame| frame.format_label().to_string())
                .collect(),
            None,
        );
    }
    let reused_latest_frame = latest_snapshot
        .values()
        .any(|frame| frame.timestamp_ms() < timestamp_ms);
    let compose_started = Instant::now();
    let materialize_started = Instant::now();
    let latest_frames =
        materialized_latest_frames(&mut state.gpu_compositor, &state.timing, &latest_snapshot)?;
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::Materialize,
            materialize_started.elapsed(),
            Some(timestamp_ms),
            Some(latest_frames.len()),
            latest_frames
                .values()
                .map(|frame| frame.format.label().to_string())
                .collect(),
            Some(
                latest_frames
                    .values()
                    .map(|frame| frame.data.len() as u64)
                    .sum(),
            ),
        );
    }
    let compose_stage = if state.plan.is_full_canvas_passthrough(&latest_frames) {
        RecorderBenchmarkStage::Passthrough
    } else {
        RecorderBenchmarkStage::Compose
    };
    let compose_only_started = Instant::now();
    let composed = state
        .plan
        .compose_latest_frames(&latest_frames, timestamp_ms)?;
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            compose_stage,
            compose_only_started.elapsed(),
            Some(timestamp_ms),
            Some(latest_frames.len()),
            vec![composed.format.label().to_string()],
            Some(composed.data.len().try_into().unwrap_or(u64::MAX)),
        );
    }
    state.plan.validate_composed_frame(&composed)?;
    let send_started = Instant::now();
    let send_outcome = send_composed_frame_with_backpressure(
        &state.composed_frames,
        composed,
        writer_backpressure_timeout(state.frame_interval_ms),
        shutdown_receiver,
    )?;
    if send_outcome == ComposedFrameSendOutcome::Stopped {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::SendWait,
            send_started.elapsed(),
            Some(timestamp_ms),
            None,
            Vec::new(),
            None,
        );
    }
    let compose_duration_ms = compose_started
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    record_composed_frame(&state.timing, timestamp_ms, reused_latest_frame);
    record_compose_duration(&state.timing, compose_duration_ms);
    if let Some(log) = state.scheduled_frame_log.record_frame(
        timestamp_ms,
        reused_latest_frame,
        compose_duration_ms,
        Instant::now(),
    ) {
        debug!(
            average_compose_duration_ms = log.average_compose_duration_ms,
            frames_since_last_log = log.frames_since_last_log,
            last_timestamp_ms = log.last_timestamp_ms,
            max_compose_duration_ms = log.max_compose_duration_ms,
            reused_frames_since_last_log = log.reused_frames_since_last_log,
            window_ms = log.window_ms,
            "pipewire_capture_scheduled_frame_composed"
        );
    }

    Ok(ComposedFrameSendOutcome::Sent)
}

fn materialized_latest_frames(
    gpu_compositor: &mut Option<LinuxGpuFrameCompositor>,
    timing: &SharedCaptureTiming,
    latest_frames: &HashMap<String, PipeWireCapturedFrame>,
) -> Result<HashMap<String, CapturedFrame>, RecorderError> {
    if latest_frames
        .values()
        .any(|frame| matches!(frame, PipeWireCapturedFrame::DmaBuf(_)))
        && gpu_compositor.is_none()
    {
        *gpu_compositor = Some(LinuxGpuFrameCompositor::new()?);
    }

    let mut frames = HashMap::with_capacity(latest_frames.len());
    for (source_id, frame) in latest_frames {
        let frame = match frame {
            PipeWireCapturedFrame::Cpu(frame) => frame.clone(),
            PipeWireCapturedFrame::DmaBuf(frame) => {
                let frame = gpu_compositor
                    .as_ref()
                    .expect("GPU compositor initialized")
                    .read_frame_to_rgba(frame)?;
                record_dmabuf_frame_read_back_to_cpu(timing);
                frame
            }
        };
        frames.insert(source_id.clone(), frame);
    }

    Ok(frames)
}

fn latest_pipewire_frame_snapshot(
    latest_frames: &SharedPipeWireFrameSampler,
) -> Result<HashMap<String, PipeWireCapturedFrame>, RecorderError> {
    latest_frames
        .lock()
        .map_err(|_error| capture_adapter_failed_error("PipeWire capture sampler lock poisoned."))?
        .latest_frames()
        .iter()
        .map(|(source_id, frame)| Ok((source_id.clone(), frame.try_clone()?)))
        .collect()
}

fn send_composed_frame_with_backpressure(
    sender: &FrameSender,
    frame: CapturedFrame,
    timeout: Duration,
    shutdown_receiver: &mpsc::Receiver<()>,
) -> Result<ComposedFrameSendOutcome, RecorderError> {
    let started = Instant::now();
    let mut frame = frame;
    loop {
        if scheduler_shutdown_requested(shutdown_receiver) {
            return Ok(ComposedFrameSendOutcome::Stopped);
        }
        match sender.try_send(frame) {
            Ok(()) => return Ok(ComposedFrameSendOutcome::Sent),
            Err(FrameTrySendError::Full(returned_frame)) => {
                if started.elapsed() >= timeout {
                    return Err(capture_adapter_failed_error(format!(
                        "Timed out sending composed frame to the media writer after {}ms.",
                        timeout.as_millis()
                    )));
                }
                if scheduler_shutdown_requested(shutdown_receiver) {
                    return Ok(ComposedFrameSendOutcome::Stopped);
                }
                frame = returned_frame;
                thread::sleep(Duration::from_millis(5));
            }
            Err(FrameTrySendError::Disconnected) => {
                return Err(capture_adapter_failed_error(
                    "Capture frame receiver is no longer available.",
                ));
            }
            Err(FrameTrySendError::InvalidFrame(error)) => return Err(error),
        }
    }
}

fn writer_backpressure_timeout(frame_interval_ms: u64) -> Duration {
    Duration::from_millis(frame_interval_ms.saturating_mul(5).max(1_000))
}
