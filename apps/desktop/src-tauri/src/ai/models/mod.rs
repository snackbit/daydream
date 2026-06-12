mod catalog;
mod download;
mod manager;
mod paths;
mod recovery;

#[cfg(test)]
pub(in crate::ai) use catalog::{catalog_asset, LOCAL_WHISPER_CATALOG};
pub(in crate::ai) use catalog::{
    catalog_model, emit_model_list, ensure_catalog_model, model_list, model_source_url,
    models_from_stored, save_catalog_model, CatalogModel, ModelAssetPatch,
};
pub(in crate::ai) use download::{
    prepare_partial_download, save_cancel_download_result, spawn_model_download,
    PartialDownloadResume,
};
pub use manager::AiModelManager;
pub(in crate::ai) use manager::DownloadRequestState;
pub(in crate::ai) use paths::{
    delete_file_if_exists, downloaded_bytes, is_verified_model_file, model_path,
    partial_model_path, rollback_finalized_model_file, sha1_file,
};
pub use recovery::recover_model_downloads;
#[cfg(test)]
pub(in crate::ai) use recovery::{recover_model_files, RecoveredModelFiles};
