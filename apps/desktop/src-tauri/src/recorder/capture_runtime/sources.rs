use super::super::types::CaptureSourceKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CaptureRuntimeSourceHandle {
    BackendId(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureRuntimeSource {
    pub(crate) display_name: String,
    pub(crate) handle: CaptureRuntimeSourceHandle,
    pub(crate) height: Option<u32>,
    pub(crate) kind: CaptureSourceKind,
    pub(crate) source_id: String,
    pub(crate) width: Option<u32>,
}
