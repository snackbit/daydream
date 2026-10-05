use super::local_receipts::LocalRequestRegistration;
use super::remote::{transcribe_via_rest, RestTranscriptionRequest};
use super::{TranscriptionError, VerboseTranscription};
use std::sync::{atomic::AtomicBool, Arc};

pub(super) async fn transcribe_local_via_rest(
    request: RestTranscriptionRequest<'_>,
    processing_lease: Option<Arc<tokio::sync::OwnedRwLockReadGuard<()>>>,
) -> Result<VerboseTranscription, TranscriptionError> {
    let cancel = request
        .cancel
        .clone()
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let job_id = processing_lease
        .as_ref()
        .and_then(|_| request.audio_path.parent()?.file_name()?.to_str())
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        .map(str::to_string);
    let registration = LocalRequestRegistration::new(cancel.clone(), processing_lease, job_id);
    let request = RestTranscriptionRequest {
        cancel: Some(cancel),
        local_request_id: Some(registration.id()),
        ..request
    };
    let result = transcribe_via_rest(request).await;
    registration.drain(result.is_err()).await;
    result
}
