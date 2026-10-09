//! Only a terminal writer result can carry fragment completion and its probe.
use super::{FragmentEvent, MediaSegmentProbe};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct FinalizedSegmentMedia {
    pub(crate) events: Vec<FragmentEvent>,
    pub(crate) probe: Option<(PathBuf, MediaSegmentProbe)>,
}
