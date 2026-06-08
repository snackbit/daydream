use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSourceRole {
    DesktopAudio,
    Microphone,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSourceKind {
    Input,
    Monitor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSource {
    pub(crate) display_name: String,
    pub(crate) id: String,
    pub(crate) is_default: bool,
    pub(crate) kind: AudioSourceKind,
    pub(crate) roles: Vec<AudioSourceRole>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceList {
    pub(crate) sources: Vec<AudioSource>,
}
