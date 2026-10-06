use gstreamer as gst;

use super::super::{
    errors::dependency_error,
    types::{RecorderError, RecorderMediaWriterCapabilities, RecorderMediaWriterHealth},
};
use super::types::MediaSegmentInput;

const REQUIRED_VIDEO_ELEMENTS: &[&str] = &[
    "appsrc",
    "queue",
    "videoconvert",
    "x264enc",
    "h264parse",
    "matroskamux",
    "splitmuxsink",
    "filesink",
];
const REQUIRED_AUDIO_ELEMENTS: &[&str] = &["pulsesrc", "audioconvert", "audioresample", "opusenc"];
const GSTREAMER_PACKAGE_HINTS: &[GstreamerPackageHint] = &[
    GstreamerPackageHint::new(
        "splitmuxsink",
        "gstreamer1.0-plugins-good",
        "gstreamer1-plugins-good",
    ),
    GstreamerPackageHint::new(
        "appsrc",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "audioconvert",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "audioresample",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new("filesink", "libgstreamer1.0-0", "gstreamer1"),
    GstreamerPackageHint::new(
        "h264parse",
        "gstreamer1.0-plugins-bad",
        "gstreamer1-plugins-bad-free",
    ),
    GstreamerPackageHint::new(
        "matroskamux",
        "gstreamer1.0-plugins-good",
        "gstreamer1-plugins-good",
    ),
    GstreamerPackageHint::new(
        "opusenc",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "pulsesrc",
        "gstreamer1.0-pulseaudio",
        "gstreamer1-plugins-good",
    ),
    GstreamerPackageHint::new("queue", "libgstreamer1.0-0", "gstreamer1"),
    GstreamerPackageHint::new(
        "videoconvert",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "x264enc",
        "gstreamer1.0-plugins-ugly",
        "gstreamer1-plugins-ugly",
    ),
];

#[derive(Clone, Copy, Debug)]
struct GstreamerPackageHint {
    deb: &'static str,
    element: &'static str,
    rpm: &'static str,
}

impl GstreamerPackageHint {
    const fn new(element: &'static str, deb: &'static str, rpm: &'static str) -> Self {
        Self { deb, element, rpm }
    }
}

pub(crate) fn collect_media_writer_health() -> RecorderMediaWriterHealth {
    let (available, init_error) = match gst::init() {
        Ok(()) => (true, None),
        Err(error) => (
            false,
            Some(format!("GStreamer initialization failed: {error}.")),
        ),
    };
    let capabilities = if available {
        collect_capabilities()
    } else {
        RecorderMediaWriterCapabilities::default()
    };
    let error = init_error.or_else(|| capability_error(&capabilities));

    RecorderMediaWriterHealth {
        available: error.is_none(),
        capabilities,
        error,
        version: available.then(gst_version_string),
        writer: Some("gstreamer".to_string()),
    }
}

pub(crate) fn validate_segment_capabilities(
    capabilities: &RecorderMediaWriterCapabilities,
    input: &MediaSegmentInput,
) -> Result<(), RecorderError> {
    if input.video_sources.len() > 1 {
        let supported = gst::ElementFactory::find("splitmuxsink").is_some_and(|factory| {
            factory
                .static_pad_templates()
                .iter()
                .any(|pad| pad.name_template() == "video_aux_%u")
        });
        if !supported {
            return Err(dependency_error("recorder_multivideo_unsupported","The installed GStreamer writer cannot record independent video tracks. Update its Good plugins before recording several sources."));
        }
    }
    if let Some(error) = segment_capability_error(capabilities, input) {
        return Err(dependency_error("recorder_media_writer_unavailable", error));
    }

    Ok(())
}

fn collect_capabilities() -> RecorderMediaWriterCapabilities {
    RecorderMediaWriterCapabilities {
        appsrc: element_available("appsrc"),
        audio_convert: element_available("audioconvert"),
        audio_resample: element_available("audioresample"),
        file_sink: element_available("filesink"),
        h264_parse: element_available("h264parse"),
        matroska_muxer: element_available("matroskamux"),
        split_muxer: element_available("splitmuxsink"),
        opus_encoder: element_available("opusenc"),
        pulse_source: element_available("pulsesrc"),
        queue: element_available("queue"),
        video_convert: element_available("videoconvert"),
        x264_encoder: element_available("x264enc"),
    }
}

fn element_available(name: &str) -> bool {
    gst::ElementFactory::find(name).is_some()
}

fn capability_error(capabilities: &RecorderMediaWriterCapabilities) -> Option<String> {
    missing_capability_error(capabilities, REQUIRED_VIDEO_ELEMENTS)
}

fn segment_capability_error(
    capabilities: &RecorderMediaWriterCapabilities,
    input: &MediaSegmentInput,
) -> Option<String> {
    let mut missing = Vec::new();
    for element in REQUIRED_VIDEO_ELEMENTS {
        if !capability_for_element(capabilities, element) {
            missing.push(*element);
        }
    }
    if input.desktop_audio_source_id.is_some() || input.microphone_audio_source_id.is_some() {
        for element in REQUIRED_AUDIO_ELEMENTS {
            if !capability_for_element(capabilities, element) {
                missing.push(*element);
            }
        }
    }
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "GStreamer is missing required media writer elements: {}. {}",
            missing.join(", "),
            missing_package_hint(&missing)
        ))
    }
}

fn missing_capability_error(
    capabilities: &RecorderMediaWriterCapabilities,
    elements: &[&str],
) -> Option<String> {
    let missing = elements
        .iter()
        .copied()
        .filter(|element| !capability_for_element(capabilities, element))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "GStreamer is missing required media writer elements: {}. {}",
            missing.join(", "),
            missing_package_hint(&missing)
        ))
    }
}

fn missing_package_hint(missing: &[&str]) -> String {
    let deb_packages = package_hints_for(missing, |hint| hint.deb);
    let rpm_packages = package_hints_for(missing, |hint| hint.rpm);

    format!(
        "Install the Linux GStreamer runtime/plugins declared in apps/desktop/src-tauri/tauri.conf.json: Debian/Ubuntu {}; RPM-family {}.",
        deb_packages.join(" "),
        rpm_packages.join(" ")
    )
}

fn package_hints_for(
    missing: &[&str],
    package_for: impl Fn(&GstreamerPackageHint) -> &'static str,
) -> Vec<&'static str> {
    let mut packages = Vec::new();
    for element in missing {
        let Some(hint) = GSTREAMER_PACKAGE_HINTS
            .iter()
            .find(|hint| hint.element == *element)
        else {
            continue;
        };
        let package = package_for(hint);
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    packages
}

fn capability_for_element(capabilities: &RecorderMediaWriterCapabilities, element: &str) -> bool {
    match element {
        "appsrc" => capabilities.appsrc,
        "audioconvert" => capabilities.audio_convert,
        "audioresample" => capabilities.audio_resample,
        "filesink" => capabilities.file_sink,
        "h264parse" => capabilities.h264_parse,
        "matroskamux" => capabilities.matroska_muxer,
        "splitmuxsink" => capabilities.split_muxer,
        "opusenc" => capabilities.opus_encoder,
        "pulsesrc" => capabilities.pulse_source,
        "queue" => capabilities.queue,
        "videoconvert" => capabilities.video_convert,
        "x264enc" => capabilities.x264_encoder,
        _ => false,
    }
}

fn gst_version_string() -> String {
    let version = gst::version();
    format!("{}.{}.{}.{}", version.0, version.1, version.2, version.3)
}
