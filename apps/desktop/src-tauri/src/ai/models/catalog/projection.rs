use super::super::super::*;
use super::{catalog_checksum, catalog_models, checksum_algorithm, model_source_url, CatalogModel};

pub(in crate::ai) fn model_list(storage: &AppStorage) -> Result<AiModelAssetList, AiError> {
    storage
        .with_connection(|connection| {
            let mut models = models_from_stored(storage::list_ai_model_assets(connection)?);
            let mut statement =
                connection.prepare("SELECT model_id,attempt FROM ai_model_download_runtime")?;
            let attempts = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for model in &mut models.models {
                if let Some((_, attempt)) = attempts.iter().find(|(id, _)| id == &model.id) {
                    model.transfer = Some(AiModelTransfer {
                        attempt: *attempt,
                        phase: match model.download_status.as_str() {
                            "available" => "ready",
                            "downloading" => "connecting",
                            "missing" => "cancelled",
                            status => status,
                        }
                        .into(),
                        bytes_per_second: None,
                        eta_ms: None,
                    });
                }
            }
            Ok(models)
        })
        .map_err(storage_error)
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
        transfer: None,
        checksum: catalog_checksum(catalog),
        checksum_algorithm: catalog_checksum(catalog)
            .map(|_checksum| checksum_algorithm(catalog).to_string()),
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
        transfer: None,
        checksum: catalog_checksum(catalog),
        checksum_algorithm: catalog_checksum(catalog)
            .map(|_checksum| checksum_algorithm(catalog).to_string()),
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
