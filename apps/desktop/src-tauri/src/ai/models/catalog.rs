use super::super::*;

pub(in crate::ai) struct CatalogModel {
    pub(in crate::ai) cache_dir: &'static str,
    pub(in crate::ai) filename: &'static str,
    pub(in crate::ai) id: &'static str,
    pub(in crate::ai) label: &'static str,
    pub(in crate::ai) provider: &'static str,
    pub(in crate::ai) sha1: &'static str,
    pub(in crate::ai) size_bytes: i64,
    pub(in crate::ai) source_url: Option<&'static str>,
}

pub(in crate::ai) const LOCAL_WHISPER_CATALOG: [CatalogModel; 6] = [
    CatalogModel {
        cache_dir: "whisper.cpp",
        filename: "ggml-tiny.bin",
        id: "tiny",
        label: "Whisper tiny",
        provider: "local-whisper-cpp",
        sha1: "bd577a113a864445d4c299885e0cb97d4ba92b5f",
        size_bytes: 75 * 1024 * 1024,
        source_url: None,
    },
    CatalogModel {
        cache_dir: "whisper.cpp",
        filename: "ggml-base.bin",
        id: "base",
        label: "Whisper base",
        provider: "local-whisper-cpp",
        sha1: "465707469ff3a37a2b9b8d8f89f2f99de7299dac",
        size_bytes: 142 * 1024 * 1024,
        source_url: None,
    },
    CatalogModel {
        cache_dir: "whisper.cpp",
        filename: "ggml-small.bin",
        id: "small",
        label: "Whisper small",
        provider: "local-whisper-cpp",
        sha1: "55356645c2b361a969dfd0ef2c5a50d530afd8d5",
        size_bytes: 466 * 1024 * 1024,
        source_url: None,
    },
    CatalogModel {
        cache_dir: "whisper.cpp",
        filename: "ggml-medium.bin",
        id: "medium",
        label: "Whisper medium",
        provider: "local-whisper-cpp",
        sha1: "fd9727b6e1217c2f614f9b698455c4ffd82463b4",
        size_bytes: 1_500 * 1024 * 1024,
        source_url: None,
    },
    CatalogModel {
        cache_dir: "whisper.cpp",
        filename: "ggml-large-v3-turbo.bin",
        id: "large-v3-turbo",
        label: "Whisper large v3 turbo",
        provider: "local-whisper-cpp",
        sha1: "4af2b29d7ec73d781377bfd1758ca957a807e941",
        size_bytes: 1_500 * 1024 * 1024,
        source_url: None,
    },
    CatalogModel {
        cache_dir: "whisper.cpp",
        filename: "ggml-large-v3.bin",
        id: "large-v3",
        label: "Whisper large v3",
        provider: "local-whisper-cpp",
        sha1: "ad82bf6a9043ceed055076d0fd39f5f186ff8062",
        size_bytes: 2_900 * 1024 * 1024,
        source_url: None,
    },
];

pub(in crate::ai) const LOCAL_OCR_CATALOG: [CatalogModel; 3] = [
    CatalogModel {
        cache_dir: "tesseract",
        filename: "bin/tesseract",
        id: "tesseract-runtime",
        label: "Tesseract OCR runtime",
        provider: "local-tesseract-ocr",
        sha1: "",
        size_bytes: 0,
        source_url: None,
    },
    CatalogModel {
        cache_dir: "tesseract",
        filename: "tessdata/eng.traineddata",
        id: "tessdata-eng",
        label: "Tesseract best English OCR data",
        provider: "local-tesseract-ocr",
        sha1: "a63325f7bed43a0070ea61c3e4eccf646e429839",
        size_bytes: 15_400_601,
        source_url: Some("https://github.com/tesseract-ocr/tessdata_best/raw/main/eng.traineddata"),
    },
    CatalogModel {
        cache_dir: "tesseract",
        filename: "tessdata/deu.traineddata",
        id: "tessdata-deu",
        label: "Tesseract best German OCR data",
        provider: "local-tesseract-ocr",
        sha1: "6a9ea6731dfa2c745833853d87dd23ac729543fd",
        size_bytes: 8_628_461,
        source_url: Some("https://github.com/tesseract-ocr/tessdata_best/raw/main/deu.traineddata"),
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
            checksum: catalog_checksum(catalog),
            checksum_algorithm: catalog_checksum(catalog).map(|_checksum| "sha1".to_string()),
            display_label: catalog.label.to_string(),
            downloaded_bytes: patch.downloaded_bytes,
            download_status: patch.download_status.into_owned(),
            etag: patch.etag.map(Cow::into_owned),
            error_code: patch.error_code.map(Cow::into_owned),
            id: catalog.id.to_string(),
            local_path: patch.local_path.map(Cow::into_owned),
            provider: catalog.provider.to_string(),
            self_test_status: patch.self_test_status.into_owned(),
            size_bytes: (catalog.size_bytes > 0).then_some(catalog.size_bytes),
            source_url: model_source_url(catalog),
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
    let models = catalog_models()
        .into_iter()
        .map(|catalog| {
            stored.iter().find(|row| row.id == catalog.id).map_or_else(
                || catalog_asset(catalog),
                |row| model_asset_from_catalog_row(catalog, row.clone()),
            )
        })
        .collect();

    AiModelAssetList { models }
}

pub(in crate::ai) fn catalog_asset(catalog: &CatalogModel) -> AiModelAsset {
    AiModelAsset {
        checksum: catalog_checksum(catalog),
        checksum_algorithm: catalog_checksum(catalog).map(|_checksum| "sha1".to_string()),
        created_at_ms: 0,
        downloaded_bytes: 0,
        download_status: "missing".to_string(),
        etag: None,
        error_code: None,
        id: catalog.id.to_string(),
        label: catalog.label.to_string(),
        local_path: None,
        provider: catalog.provider.to_string(),
        self_test_status: "not-run".to_string(),
        size_bytes: (catalog.size_bytes > 0).then_some(catalog.size_bytes),
        source_url: model_source_url(catalog),
        total_bytes: (catalog.size_bytes > 0).then_some(catalog.size_bytes),
        updated_at_ms: 0,
        verification_status: "unknown".to_string(),
    }
}

fn model_asset_from_catalog_row(catalog: &CatalogModel, row: AiModelAssetRow) -> AiModelAsset {
    if row.checksum != catalog_checksum(catalog) {
        return catalog_asset(catalog);
    }

    AiModelAsset {
        checksum: catalog_checksum(catalog),
        checksum_algorithm: catalog_checksum(catalog).map(|_checksum| "sha1".to_string()),
        created_at_ms: row.created_at_ms,
        downloaded_bytes: row.downloaded_bytes,
        download_status: row.download_status,
        etag: row.etag,
        error_code: row.error_code,
        id: catalog.id.to_string(),
        label: catalog.label.to_string(),
        local_path: row.local_path,
        provider: catalog.provider.to_string(),
        self_test_status: row.self_test_status,
        size_bytes: (catalog.size_bytes > 0).then_some(catalog.size_bytes),
        source_url: model_source_url(catalog),
        total_bytes: row
            .total_bytes
            .or((catalog.size_bytes > 0).then_some(catalog.size_bytes)),
        updated_at_ms: row.updated_at_ms,
        verification_status: row.verification_status,
    }
}

fn catalog_checksum(catalog: &CatalogModel) -> Option<String> {
    (!catalog.sha1.is_empty()).then(|| catalog.sha1.to_string())
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
    catalog_models()
        .into_iter()
        .find(|model| model.id == model_id)
}

pub(in crate::ai) fn catalog_models() -> Vec<&'static CatalogModel> {
    LOCAL_WHISPER_CATALOG
        .iter()
        .chain(LOCAL_OCR_CATALOG.iter())
        .collect()
}

pub(in crate::ai) fn model_source_url(catalog: &CatalogModel) -> Option<String> {
    if catalog.provider == "local-whisper-cpp" {
        return Some(format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            catalog.id
        ));
    }

    catalog.source_url.map(ToOwned::to_owned)
}

pub(in crate::ai) fn require_model_source_url(catalog: &CatalogModel) -> Result<String, AiError> {
    model_source_url(catalog).ok_or_else(|| {
        let (code, message) = if catalog.id == "tesseract-runtime" {
            tesseract_runtime_source_error()
        } else {
            (
                "ai_model_download_source_unavailable",
                "This AI asset does not have a downloadable source for this platform.",
            )
        };

        AiError {
            code: code.to_string(),
            message: message.to_string(),
        }
    })
}

pub(in crate::ai) fn is_whisper_model(catalog: &CatalogModel) -> bool {
    catalog.provider == "local-whisper-cpp"
}

fn tesseract_runtime_source_error() -> (&'static str, &'static str) {
    if cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        (
            "ai_ocr_tesseract_runtime_unavailable",
            "Tesseract OCR runtime download is unavailable until a pinned Daydream runtime artifact exists for this platform.",
        )
    } else {
        (
            "ai_ocr_tesseract_platform_unsupported",
            "Tesseract OCR runtime download is unsupported on this platform.",
        )
    }
}
pub(in crate::ai) fn emit_model_list(app: &AppHandle) {
    let storage = app.state::<AppStorage>();
    if let Ok(models) = model_list(&storage) {
        let _ = app.emit(AI_MODELS_CHANGED_EVENT, models);
        crate::background_tasks::emit_changed(app, None);
    }
}
