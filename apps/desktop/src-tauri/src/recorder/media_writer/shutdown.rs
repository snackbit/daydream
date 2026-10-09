use super::*;

impl MediaWriterSegment {
    #[cfg(test)]
    pub(crate) fn hold_audio_pad_for_test(&self) -> Result<(), RecorderError> {
        let source = self.pipeline.by_name("desktop_audio_src").ok_or_else(|| {
            dependency_error("synthetic_audio_missing", "Synthetic audio source missing.")
        })?;
        let pad = source.static_pad("src").ok_or_else(|| {
            dependency_error(
                "synthetic_audio_missing",
                "Synthetic audio source pad missing.",
            )
        })?;
        let (held, ready) = mpsc::channel();
        pad.add_probe(gst::PadProbeType::BUFFER, move |_, _| {
            let _ = held.send(());
            loop {
                thread::park();
            }
        });
        ready.recv_timeout(Duration::from_secs(1)).map_err(|_| {
            dependency_error(
                "synthetic_audio_lock_missing",
                "Synthetic audio streaming pad did not lock.",
            )
        })
    }
    #[cfg(test)]
    pub(crate) fn set_null_for_test(&self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
    #[cfg(test)]
    pub(crate) fn legacy_audio_eos_for_test(&self, timestamp_ms: u64) -> Result<(), RecorderError> {
        self.apply_cutoff(timestamp_ms)?;
        // Reproduce the original dependency: desktop EOS was awaited before
        // microphone EOS and before video appsrc EOS. Never tear down this
        // blocked graph in the parent process; the fixture kills/reaps it.
        send_audio_source_eos(&self.pipeline, &self.audio_branches[..1])
    }
    #[cfg(test)]
    pub(crate) fn abort(mut self) {
        if let Some(stop) = self.stop_frames.take() {
            let _ = stop.send(FramePumpStop {
                stop_at_ms: self.started_at_ms,
            });
        }
        let _ = self.pipeline.set_state(gst::State::Null);
        if self
            .frame_pump_result
            .recv_timeout(FRAME_PUMP_STOP_TIMEOUT)
            .is_ok()
        {
            if let Some(worker) = self.frame_pump.take() {
                let _ = worker.join();
            }
        }
    }

    pub(crate) fn stop(mut self, stop_at_ms: u64) -> Result<Vec<FragmentEvent>, RecorderError> {
        info!(stop_at_ms, "media_writer_stop_requested");
        let audio_cutoff_probes = install_audio_cutoff_probes(
            &self.pipeline,
            &self.audio_branches,
            self.started_at_ms,
            stop_at_ms,
        )?;
        if let Some(stop_frames) = self.stop_frames.take() {
            let _ = stop_frames.send(FramePumpStop { stop_at_ms });
            trace!("media_writer_frame_pump_stop_signaled");
        }

        match self
            .frame_pump_completion
            .take()
            .map(Ok)
            .unwrap_or_else(|| self.frame_pump_result.recv_timeout(FRAME_PUMP_STOP_TIMEOUT))
        {
            Ok(Ok(())) => trace!("media_writer_frame_pump_stopped"),
            Ok(Err(error)) => {
                error!(error = %error, "media_writer_frame_pump_failed");
                let _ = self.pipeline.set_state(gst::State::Null);
                return Err(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    format!("Failed to feed GStreamer video frames: {error}"),
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                error!("media_writer_frame_pump_stop_timeout");
                let _ = self.pipeline.set_state(gst::State::Null);
                return Err(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    "Timed out waiting for the GStreamer frame writer to stop.",
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error!("media_writer_frame_pump_result_disconnected");
                let _ = self.pipeline.set_state(gst::State::Null);
                return Err(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    "GStreamer frame writer stopped without reporting a result.",
                ));
            }
        }
        if let Some(frame_pump) = self.frame_pump.take() {
            let _ = frame_pump.join();
        }

        let _ = self.appsrc.end_of_stream();
        for source in &self.extra_video_sources {
            let _ = source.end_of_stream();
        }
        trace!("media_writer_appsrc_eos_sent");

        if let Err(error) = send_audio_source_eos(&self.pipeline, &self.audio_branches) {
            let _ = self.pipeline.set_state(gst::State::Null);
            return Err(error);
        }

        trace!("media_writer_waiting_for_bus_eos");
        let result = wait_for_eos_or_error(
            &self.bus,
            &self.open_fragments,
            &self.fragment_seen,
            &self.reference_eos,
        );
        let _ = self.pipeline.set_state(gst::State::Null);
        log_audio_cutoff_results(&audio_cutoff_probes);
        match &result {
            Ok(()) => info!("media_writer_stop_finalized"),
            Err(error) => error!(
                error_code = %error.code,
                error_category = ?error.category,
                "media_writer_stop_failed"
            ),
        }
        if let Ok(()) = &result {
            if let Some(benchmark) = &self.benchmark {
                benchmark.finish();
            }
        }

        result.map(|_| self.drain_fragments())
    }
}

fn wait_for_eos_or_error(
    bus: &gst::Bus,
    open: &Mutex<std::collections::BTreeMap<std::path::PathBuf, bool>>,
    seen: &AtomicBool,
    reference_eos: &AtomicBool,
) -> Result<(), RecorderError> {
    let deadline = std::time::Instant::now() + EOS_TIMEOUT;
    loop {
        // Async splitmux finalization can consume sink EOS internally. Closed
        // notifications confirm every opened muxer finished writing its file.
        if reference_eos.load(Ordering::SeqCst)
            && seen.load(Ordering::SeqCst)
            && open.lock().is_ok_and(|fragments| {
                !fragments.is_empty() && fragments.values().all(|closed| *closed)
            })
        {
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let Some(message) = bus.timed_pop(duration_to_clock_time(remaining)) else {
            warn!("media_writer_eos_timeout");
            return Err(dependency_error(
                "recorder_media_writer_finalize_failed",
                "Timed out waiting for GStreamer to finalize the segment.",
            ));
        };
        match message.view() {
            gst::MessageView::Eos(_eos) => {
                info!("media_writer_eos_received");
                return Ok(());
            }
            gst::MessageView::Error(error) => {
                error!(
                    source = %error
                        .src()
                        .map(|source| source.path_string())
                        .unwrap_or_else(|| "unknown".into()),
                    error = %error.error(),
                    "media_writer_bus_error"
                );
                return Err(dependency_error(
                    "recorder_media_writer_failed",
                    format!(
                        "GStreamer error from {}: {}.",
                        error
                            .src()
                            .map(|source| source.path_string())
                            .unwrap_or_else(|| "unknown".into()),
                        error.error()
                    ),
                ));
            }
            _ => {}
        }
    }
}
