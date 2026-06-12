use super::super::*;

pub(in crate::ai) struct CatalogModel {
    pub(in crate::ai) id: &'static str,
    pub(in crate::ai) label: &'static str,
    pub(in crate::ai) sha1: &'static str,
    pub(in crate::ai) size_bytes: i64,
}

pub(in crate::ai) const LOCAL_WHISPER_CATALOG: [CatalogModel; 6] = [
    CatalogModel {
        id: "tiny",
        label: "Whisper tiny",
        sha1: "bd577a113a864445d4c299885e0cb97d4ba92b5f",
        size_bytes: 75 * 1024 * 1024,
    },
    CatalogModel {
        id: "base",
        label: "Whisper base",
        sha1: "465707469ff3a37a2b9b8d8f89f2f99de7299dac",
        size_bytes: 142 * 1024 * 1024,
    },
    CatalogModel {
        id: "small",
        label: "Whisper small",
        sha1: "55356645c2b361a969dfd0ef2c5a50d530afd8d5",
        size_bytes: 466 * 1024 * 1024,
    },
    CatalogModel {
        id: "medium",
        label: "Whisper medium",
        sha1: "fd9727b6e1217c2f614f9b698455c4ffd82463b4",
        size_bytes: 1_500 * 1024 * 1024,
    },
    CatalogModel {
        id: "large-v3-turbo",
        label: "Whisper large v3 turbo",
        sha1: "4af2b29d7ec73d781377bfd1758ca957a807e941",
        size_bytes: 1_500 * 1024 * 1024,
    },
    CatalogModel {
        id: "large-v3",
        label: "Whisper large v3",
        sha1: "ad82bf6a9043ceed055076d0fd39f5f186ff8062",
        size_bytes: 2_900 * 1024 * 1024,
    },
];

#[derive(Clone)]
pub(in crate::ai) struct ModelAssetPatch<'a> {
    pub(in crate::ai) download_status: Cow<'a, str>,
    pub(in crate::ai) downloaded_bytes: i64,
    pub(in crate::ai) error_code: Option<Cow<'a, str>>,
    pub(in crate::ai) etag: Option<Cow<'a, str>>,
    pub(in crate::ai) local_path: Option<Cow<'a, str>>,
    pub(in crate::ai) self_test_status: Cow<'a, str>,
    pub(in crate::ai) total_bytes: Option<i64>,
    pub(in crate::ai) verification_status: Cow<'a, str>,
}

impl<'a> ModelAssetPatch<'a> {
    pub(in crate::ai) fn missing() -> Self {
        Self {
            download_status: "missing".into(),
            downloaded_bytes: 0,
            error_code: None,
            etag: None,
            local_path: None,
            self_test_status: "not-run".into(),
            total_bytes: None,
            verification_status: "unknown".into(),
        }
    }

    pub(in crate::ai) fn from_existing(
        connection: &rusqlite::Connection,
        catalog: &CatalogModel,
    ) -> rusqlite::Result<Self> {
        Ok(storage::get_ai_model_asset(connection, catalog.id)?
            .as_ref()
            .map_or_else(Self::missing, |row| Self {
                download_status: Cow::Owned(row.download_status.clone()),
                downloaded_bytes: row.downloaded_bytes,
                error_code: row.error_code.clone().map(Cow::Owned),
                etag: row.etag.clone().map(Cow::Owned),
                local_path: row.local_path.clone().map(Cow::Owned),
                self_test_status: Cow::Owned(row.self_test_status.clone()),
                total_bytes: row.total_bytes,
                verification_status: Cow::Owned(row.verification_status.clone()),
            }))
    }
}

pub(in crate::ai) fn save_catalog_model(
    connection: &rusqlite::Connection,
    catalog: &CatalogModel,
    patch: ModelAssetPatch<'_>,
    timestamp_ms: i64,
) -> rusqlite::Result<AiModelAssetRow> {
    storage::save_ai_model_asset(
        connection,
        &SaveAiModelAssetInput {
            checksum: Some(catalog.sha1.to_string()),
            checksum_algorithm: Some("sha1".to_string()),
            display_label: catalog.label.to_string(),
            downloaded_bytes: patch.downloaded_bytes,
            download_status: patch.download_status.into_owned(),
            etag: patch.etag.map(Cow::into_owned),
            error_code: patch.error_code.map(Cow::into_owned),
            id: catalog.id.to_string(),
            local_path: patch.local_path.map(Cow::into_owned),
            provider: "local-whisper-cpp".to_string(),
            self_test_status: patch.self_test_status.into_owned(),
            size_bytes: Some(catalog.size_bytes),
            source_url: Some(model_source_url(catalog)),
            timestamp_ms,
            total_bytes: patch.total_bytes,
            verification_status: patch.verification_status.into_owned(),
        },
    )
}

pub(in crate::ai) fn model_list(storage: &AppStorage) -> Result<AiModelAssetList, AiError> {
    storage
        .with_connection(storage::list_ai_model_assets)
        .map_err(storage_error)
        .map(models_from_stored)
}

pub(in crate::ai) fn models_from_stored(stored: Vec<AiModelAssetRow>) -> AiModelAssetList {
    let models = LOCAL_WHISPER_CATALOG
        .iter()
        .map(|catalog| {
            stored.iter().find(|row| row.id == catalog.id).map_or_else(
                || catalog_asset(catalog),
                |row| model_asset_from_row(row.clone()),
            )
        })
        .collect();

    AiModelAssetList { models }
}

pub(in crate::ai) fn catalog_asset(catalog: &CatalogModel) -> AiModelAsset {
    AiModelAsset {
        checksum: Some(catalog.sha1.to_string()),
        checksum_algorithm: Some("sha1".to_string()),
        created_at_ms: 0,
        downloaded_bytes: 0,
        download_status: "missing".to_string(),
        etag: None,
        error_code: None,
        id: catalog.id.to_string(),
        label: catalog.label.to_string(),
        local_path: None,
        provider: "local-whisper-cpp".to_string(),
        self_test_status: "not-run".to_string(),
        size_bytes: Some(catalog.size_bytes),
        source_url: Some(model_source_url(catalog)),
        total_bytes: Some(catalog.size_bytes),
        updated_at_ms: 0,
        verification_status: "unknown".to_string(),
    }
}

pub(in crate::ai) fn model_asset_from_row(row: AiModelAssetRow) -> AiModelAsset {
    AiModelAsset {
        checksum: row.checksum,
        checksum_algorithm: row.checksum_algorithm,
        created_at_ms: row.created_at_ms,
        downloaded_bytes: row.downloaded_bytes,
        download_status: row.download_status,
        etag: row.etag,
        error_code: row.error_code,
        id: row.id,
        label: row.display_label,
        local_path: row.local_path,
        provider: row.provider,
        self_test_status: row.self_test_status,
        size_bytes: row.size_bytes,
        source_url: row.source_url,
        total_bytes: row.total_bytes,
        updated_at_ms: row.updated_at_ms,
        verification_status: row.verification_status,
    }
}

pub(in crate::ai) fn ensure_catalog_model(
    model_id: &str,
) -> Result<&'static CatalogModel, AiError> {
    catalog_model(model_id).ok_or_else(|| AiError {
        code: "ai_model_unknown".to_string(),
        message: format!("AI model is unknown: {model_id}"),
    })
}

pub(in crate::ai) fn catalog_model(model_id: &str) -> Option<&'static CatalogModel> {
    LOCAL_WHISPER_CATALOG
        .iter()
        .find(|model| model.id == model_id)
}

pub(in crate::ai) fn model_source_url(catalog: &CatalogModel) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        catalog.id
    )
}
pub(in crate::ai) fn emit_model_list(app: &AppHandle) {
    let storage = app.state::<AppStorage>();
    if let Ok(models) = model_list(&storage) {
        let _ = app.emit(AI_MODELS_CHANGED_EVENT, models);
    }
}
