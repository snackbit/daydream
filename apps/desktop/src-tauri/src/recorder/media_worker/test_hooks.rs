use super::*;

impl MediaWriterSegment {
    pub(in crate::recorder::media_worker) fn hold_audio_pad_for_test(&mut self) {
        let Self::Process(process) = self else {
            panic!("Synthetic process required");
        };
        protocol::send(
            &mut process.control,
            &Request::TestHoldAudioPad {
                generation: process.generation,
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match process
                .responses
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap()
            {
                Response::TestAudioPadHeld { generation } if generation == process.generation => {
                    return
                }
                Response::Fragments { events, .. } => process.fragments.extend(events),
                other => panic!("Unexpected synthetic lock response: {other:?}"),
            }
        }
    }
    pub(in crate::recorder::media_worker) fn null_is_blocked_for_test(&mut self) -> bool {
        let Self::Process(process) = self else {
            panic!("Synthetic process required");
        };
        protocol::send(
            &mut process.control,
            &Request::TestNull {
                generation: process.generation,
            },
        )
        .unwrap();
        loop {
            match process
                .responses
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
            {
                Response::TestNullStarted { generation } if generation == process.generation => {
                    break
                }
                Response::Fragments { events, .. } => process.fragments.extend(events),
                other => panic!("Unexpected synthetic Null response: {other:?}"),
            }
        }
        matches!(
            process.responses.recv_timeout(Duration::from_millis(200)),
            Err(mpsc::RecvTimeoutError::Timeout)
        )
    }
    #[cfg(test)]
    pub(in crate::recorder::media_worker) fn legacy_eos_for_test(
        &mut self,
        timestamp_ms: u64,
    ) -> Result<(), RecorderError> {
        let Self::Process(process) = self else {
            return Err(failure("synthetic_worker_required"));
        };
        protocol::send(
            &mut process.control,
            &Request::TestLegacyEos {
                generation: process.generation,
                timestamp_ms,
            },
        )
        .map_err(|_| failure("synthetic_fixture_transport_failed"))?;
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            match process
                .responses
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                Ok(Response::TestLegacyEos { generation, result })
                    if generation == process.generation =>
                {
                    return result
                }
                Ok(Response::Fragments { generation, events })
                    if generation == process.generation =>
                {
                    process.fragments.extend(events)
                }
                Ok(Response::Failed { error, .. }) => return Err(error),
                Err(_) => return Err(failure("synthetic_legacy_probe_did_not_finish")),
                _ => {}
            }
        }
    }

    #[cfg(test)]
    pub(in crate::recorder::media_worker) fn stall_for_test(&mut self) {
        if let Self::Process(process) = self {
            let _ = protocol::send(
                &mut process.control,
                &Request::TestStall {
                    generation: process.generation,
                },
            );
        }
    }
}
