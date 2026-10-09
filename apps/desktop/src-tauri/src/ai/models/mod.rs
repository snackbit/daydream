mod catalog;
mod download;
mod finalize;
mod manager;
mod paths;
mod recovery;
mod transfer_state;
mod transport;

#[cfg(test)]
pub(in crate::ai) use catalog::{catalog_asset, LOCAL_WHISPER_CATALOG};
pub(in crate::ai) use catalog::{
    catalog_model, emit_model_list, ensure_catalog_model, is_whisper_model, model_list,
    models_from_stored, require_model_source_url, save_catalog_model, CatalogModel,
    ModelAssetPatch,
};
pub(in crate::ai) use download::{
    prepare_partial_download, spawn_model_download, PartialDownloadResume,
};
pub use manager::AiModelManager;
pub(in crate::ai) use manager::DownloadRequestState;
pub(in crate::ai) use paths::{
    checksum_file, delete_file_if_exists, downloaded_bytes, is_verified_model_file, model_path,
    partial_model_path, rollback_finalized_model_file,
};
pub use recovery::recover_model_downloads;
#[cfg(test)]
pub(in crate::ai) use recovery::{recover_model_files, RecoveredModelFiles};
pub use transfer_state::cancel_transfers_for_maintenance;
pub(in crate::ai) use transfer_state::model_transfer_action;
pub(in crate::ai) use transfer_state::save_cancel_download_result;
pub(in crate::ai) use transfer_state::save_stopped_download;
