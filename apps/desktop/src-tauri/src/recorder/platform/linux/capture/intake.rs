use super::*;

pub(super) fn stream_configs_from_plan(
    plan: &SceneCompositorPlan,
) -> Result<Vec<PipeWireStreamConfig>, RecorderError> {
    plan.inputs
        .iter()
        .map(stream_config_from_input)
        .collect::<Result<Vec<_>, _>>()
}

fn stream_config_from_input(
    input: &SceneCompositorInput,
) -> Result<PipeWireStreamConfig, RecorderError> {
    let CaptureRuntimeSourceHandle::BackendId(handle) = &input.runtime_handle;
    let Some(node_id) = handle.strip_prefix("pipewire-node-") else {
        return Err(capture_source_missing_error(&input.source_id));
    };
    let node_id = node_id
        .parse::<u32>()
        .map_err(|_error| capture_source_missing_error(&input.source_id))?;

    Ok(PipeWireStreamConfig {
        node_id,
        source_id: input.source_id.clone(),
    })
}

pub(super) fn capture_stream_frame(
    stream: &pw::stream::Stream,
    user_data: &mut PipeWireStreamData,
    state: &Rc<RefCell<PipeWireCaptureState>>,
) -> Result<(), RecorderError> {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return Ok(());
    };
    let datas = buffer.datas_mut();
    if datas.is_empty() {
        return Ok(());
    }

    log_negotiated_stream_format(user_data, &datas[0]);
    let frame =
        captured_pipewire_frame_from_data(&user_data.source_id, user_data.format, &mut datas[0])?;
    let (frame_source_id, frame_storage, frame_timestamp_ms) = match &frame {
        PipeWireCapturedFrame::Cpu(frame) => (
            frame.source_id.clone(),
            CaptureFrameStorageKind::CpuReadable,
            frame.timestamp_ms,
        ),
        PipeWireCapturedFrame::DmaBuf(frame) => (
            frame.source_id.clone(),
            CaptureFrameStorageKind::DmaBuf,
            frame.timestamp_ms,
        ),
    };
    let mut state = state.borrow_mut();
    record_capture_frame(
        &state.timing,
        &frame_source_id,
        frame_storage,
        frame_timestamp_ms,
    );
    state
        .latest_frames
        .lock()
        .map_err(|_error| capture_adapter_failed_error("PipeWire capture sampler lock poisoned."))?
        .push(frame)?;
    if let Some(log) =
        state
            .latest_frame_log
            .record_frame(&frame_source_id, frame_timestamp_ms, Instant::now())
    {
        debug!(
            frames_since_last_log = log.frames_since_last_log,
            source_count = log.latest_timestamps_by_source.len(),
            latest_timestamps_by_source = ?log.latest_timestamps_by_source,
            window_ms = log.window_ms,
            "pipewire_capture_latest_frame_updated"
        );
    }

    Ok(())
}

fn log_negotiated_stream_format(user_data: &mut PipeWireStreamData, data: &spa::buffer::Data) {
    if user_data.logged_negotiated_format {
        return;
    }
    let size = user_data.format.size();
    let Ok(frame_format) = captured_frame_format_from_pipewire(user_data.format.format()) else {
        return;
    };
    let stride = data.chunk().stride();
    let storage = storage_from_pipewire_data_type(data.type_());

    info!(
        source_id = %user_data.source_id,
        pipewire_format = ?user_data.format.format(),
        runtime_format = %frame_format.label(),
        storage = ?storage,
        width = size.width,
        height = size.height,
        stride,
        "pipewire_capture_stream_format_negotiated"
    );
    user_data.logged_negotiated_format = true;
}
