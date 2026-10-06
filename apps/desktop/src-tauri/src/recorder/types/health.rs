use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendComponentHealth {
    pub(crate) available: bool,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderMediaWriterHealth {
    pub(crate) available: bool,
    pub(crate) capabilities: RecorderMediaWriterCapabilities,
    pub(crate) error: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) writer: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderMediaWriterCapabilities {
    pub(crate) appsrc: bool,
    pub(crate) audio_convert: bool,
    pub(crate) audio_resample: bool,
    pub(crate) file_sink: bool,
    pub(crate) h264_parse: bool,
    pub(crate) matroska_muxer: bool,
    #[serde(default)]
    pub(crate) split_muxer: bool,
    pub(crate) opus_encoder: bool,
    pub(crate) pulse_source: bool,
    pub(crate) queue: bool,
    pub(crate) video_convert: bool,
    pub(crate) x264_encoder: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderAudioHealth {
    pub(crate) available: bool,
    pub(crate) default_source_id: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPortalHealth {
    pub(crate) available: bool,
    pub(crate) cursor_modes: Vec<String>,
    pub(crate) error: Option<String>,
    pub(crate) screen_cast_version: Option<u32>,
    pub(crate) source_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderGpuCaptureHealth {
    pub(crate) available: bool,
    pub(crate) cleanup_error: Option<String>,
    pub(crate) dmabuf_import_available: bool,
    pub(crate) egl_available: bool,
    pub(crate) egl_client_apis: Vec<String>,
    pub(crate) egl_platform: Option<String>,
    pub(crate) egl_vendor: Option<String>,
    pub(crate) egl_version: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) modifier_import_available: Option<bool>,
    pub(crate) opengl_available: bool,
    pub(crate) renderer: Option<String>,
    pub(crate) vendor: Option<String>,
    pub(crate) version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendHealth {
    pub(crate) desktop_audio: RecorderAudioHealth,
    pub(crate) gpu_capture: RecorderGpuCaptureHealth,
    pub(crate) media_writer: RecorderMediaWriterHealth,
    pub(crate) microphone: RecorderAudioHealth,
    pub(crate) pipewire: RecorderBackendComponentHealth,
    pub(crate) portal: RecorderPortalHealth,
}
