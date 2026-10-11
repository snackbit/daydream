//! The media process owns all streaming locks; the UI process owns capture and storage.
mod child;
#[cfg(test)]
mod deleted_executable_tests;
#[cfg(test)]
mod fd_tests;
mod parent;
#[cfg(test)]
mod parent_death_tests;
mod process;
mod protocol;
mod recovery;
#[cfg(test)]
mod tests;
pub(crate) use recovery::recover_media;
#[cfg(test)]
pub(crate) use tests::PROCESS_TEST;

pub(crate) use child::run_worker_process;
pub(crate) use parent::{
    is_quarantined, is_quiescent, quarantine, start_segment_from_frames, MediaWriterSegment,
};
