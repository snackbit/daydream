use super::types::*;
use crate::settings::{AppSettings, AppSettingsError};
use std::collections::HashSet;

fn invalid(message: &str) -> AppSettingsError {
    AppSettingsError {
        code: "settings_model_invalid".into(),
        field: Some("ai.modelRegistry".into()),
        message: message.into(),
    }
}
pub(crate) fn validate(registry: &Registry) -> Result<(), AppSettingsError> {
    if registry.version != 1 {
        return Err(invalid(
            "This model configuration version is not supported.",
        ));
    }
    let mut ids = HashSet::new();
    for connection in &registry.connections {
        if connection.id.is_empty() || connection.id.len() > 128 || !ids.insert(&connection.id) {
            return Err(invalid("Connection IDs must be unique."));
        }
        if connection.name.trim().is_empty() || connection.name.len() > 256 {
            return Err(invalid("Give each connection a name."));
        }
        if !["openai", "openai-compatible"].contains(&connection.provider.as_str())
            || !["api-key", "none", "chatgpt"].contains(&connection.auth_mode.as_str())
        {
            return Err(invalid(
                "Choose a supported provider and authentication method.",
            ));
        }
        if connection.provider == "openai" {
            if connection.base_url != "https://api.openai.com/v1"
                || connection.auth_mode == "none"
                || connection.protocol != Protocol::Responses
            {
                return Err(invalid(
                    "OpenAI connections use the public Responses API and authenticated access.",
                ));
            }
        } else if connection.auth_mode == "chatgpt" {
            return Err(invalid(
                "ChatGPT sign-in is available for OpenAI connections.",
            ));
        }
        if !connection.base_url.is_empty() {
            let url = reqwest::Url::parse(&connection.base_url)
                .map_err(|_| invalid("Enter a valid API base URL."))?;
            if !["http", "https"].contains(&url.scheme())
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(invalid(
                    "The API URL must not contain credentials, query parameters or a fragment.",
                ));
            }
        }
        if connection
            .api_key_ref
            .as_ref()
            .is_some_and(|value| !value.starts_with("ai.secret.") || value.len() > 256)
        {
            return Err(invalid("The saved credential reference is invalid."));
        }
        if connection
            .subscription_id
            .as_ref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err(invalid("The ChatGPT registration is invalid."));
        }
        if connection.auth_mode != "chatgpt" && connection.subscription_id.is_some() {
            return Err(invalid(
                "Subscription registrations cannot be used as API keys.",
            ));
        }
        if connection.auth_mode == "chatgpt" && connection.api_key_ref.is_some() {
            return Err(invalid("ChatGPT connections cannot contain an API key."));
        }
    }
    let mut model_ids = HashSet::new();
    for model in &registry.models {
        if model.id.is_empty() || model.id.len() > 128 || !model_ids.insert(&model.id) {
            return Err(invalid("Model IDs must be unique."));
        }
        let connection = registry
            .connections
            .iter()
            .find(|c| c.id == model.connection_id)
            .ok_or_else(|| invalid("Choose an existing connection for every model."))?;
        if model.name.trim().is_empty()
            || model.name.len() > 256
            || model.model.len() > 256
            || model.model.chars().any(char::is_control)
        {
            return Err(invalid("Enter a model name and a valid model identifier."));
        }
        if model
            .capabilities
            .iter()
            .enumerate()
            .any(|(i, cap)| model.capabilities[..i].contains(cap))
        {
            return Err(invalid("Model capabilities must be unique."));
        }
        if model.capabilities.contains(&Capability::Vision)
            && !model.capabilities.contains(&Capability::Text)
        {
            return Err(invalid("Vision models must also support text generation."));
        }
        if connection.auth_mode == "chatgpt" && model.capabilities.contains(&Capability::Embedding)
        {
            return Err(invalid(
                "Use a local model or API connection for embeddings.",
            ));
        }
        if model
            .embedding_dimensions
            .is_some_and(|d| !(1..=8192).contains(&d))
            || !["plain", "e5", "bge"].contains(&model.embedding_convention.as_str())
        {
            return Err(invalid(
                "The embedding dimensions or input convention are invalid.",
            ));
        }
    }
    for (id, capability) in [
        (&registry.bindings.embedding, Capability::Embedding),
        (&registry.bindings.summary, Capability::Text),
        (&registry.bindings.vision, Capability::Vision),
    ] {
        if let Some(id) = id {
            if !registry
                .models
                .iter()
                .any(|m| &m.id == id && m.capabilities.contains(&capability))
            {
                return Err(invalid("A selected model is missing or no longer supports its feature. Reassign it before saving."));
            }
        }
    }
    Ok(())
}

pub(crate) fn import_legacy(settings: &mut AppSettings) {
    if settings.ai.model_registry.is_some() {
        return;
    }
    let mut registry = Registry::default();
    let embedding = &settings.ai.recall.embedding;
    let generation = &settings.ai.recall.generation;
    let vision = &settings.ai.vision;
    let roles = [
        (
            "embedding",
            "Embeddings",
            embedding.base_url.as_ref(),
            embedding.model.as_ref(),
            &embedding.auth_mode,
            &embedding.api_key_ref,
            vec![Capability::Embedding],
        ),
        (
            "summary",
            "Day summaries",
            generation.base_url.as_ref(),
            generation.model.as_ref(),
            &generation.auth_mode,
            &generation.api_key_ref,
            vec![Capability::Text],
        ),
        (
            "vision",
            "OCR and Vision",
            vision.base_url.as_ref(),
            vision.model.as_ref(),
            &vision.auth_mode,
            &vision.api_key_ref,
            vec![Capability::Text, Capability::Vision],
        ),
    ];
    for (role, name, base, model, auth, key, capabilities) in roles {
        if base.is_none() && model.is_none() && key.is_none() {
            continue;
        }
        let base = base.cloned().unwrap_or_default();
        let trimmed = base.trim_end_matches('/');
        let base = if role != "embedding"
            && !trimmed.is_empty()
            && !trimmed.ends_with("/v1")
            && !trimmed.ends_with("/chat/completions")
        {
            format!("{trimmed}/v1")
        } else {
            trimmed.trim_end_matches("/chat/completions").to_string()
        };
        let direct = base == "https://api.openai.com/v1" && auth == "api-key";
        let connection_id = registry
            .connections
            .iter()
            .find(|c| c.base_url == base && &c.auth_mode == auth && &c.api_key_ref == key)
            .map(|c| c.id.clone())
            .unwrap_or_else(|| {
                let id = format!("legacy-{role}-connection");
                registry.connections.push(Connection {
                    id: id.clone(),
                    name: name.into(),
                    provider: if direct {
                        "openai"
                    } else {
                        "openai-compatible"
                    }
                    .into(),
                    auth_mode: auth.clone(),
                    base_url: base,
                    api_key_ref: key.clone(),
                    subscription_id: None,
                    protocol: if direct {
                        Protocol::Responses
                    } else {
                        Protocol::ChatCompletions
                    },
                    token_limit: TokenLimit::Legacy,
                });
                id
            });
        let id = format!("legacy-{role}-model");
        registry.models.push(Model {
            id: id.clone(),
            name: name.into(),
            model: model.cloned().unwrap_or_default(),
            connection_id,
            capabilities,
            embedding_dimensions: (role == "embedding").then_some(embedding.dimensions),
            embedding_convention: if role == "embedding" {
                embedding.convention.clone()
            } else {
                "plain".into()
            },
        });
        match role {
            "embedding" => {
                if embedding.provider != "local" {
                    registry.bindings.embedding = Some(id);
                }
            }
            "summary" => registry.bindings.summary = Some(id),
            _ => registry.bindings.vision = Some(id),
        }
    }
    settings.ai.model_registry = Some(registry);
}

pub(crate) fn project(settings: &mut AppSettings) -> Result<(), AppSettingsError> {
    let Some(registry) = &settings.ai.model_registry else {
        return Ok(());
    };
    validate(registry)?;
    let selected = |id: &Option<String>| {
        id.as_ref()
            .and_then(|id| registry.models.iter().find(|m| &m.id == id))
            .and_then(|m| {
                Some(m).zip(
                    registry
                        .connections
                        .iter()
                        .find(|c| c.id == m.connection_id),
                )
            })
    };
    if let Some((model, connection)) = selected(&registry.bindings.embedding) {
        let profile = &mut settings.ai.recall.embedding;
        profile.provider = "openai-compatible".into();
        profile.base_url = Some(connection.base_url.clone());
        profile.model = Some(model.model.clone());
        profile.auth_mode = connection.auth_mode.clone();
        profile.api_key_ref = connection.api_key_ref.clone();
        if profile.enabled && model.embedding_dimensions.is_none() {
            return Err(invalid("Test the embedding model to detect its dimensions before enabling remote embeddings."));
        }
        profile.dimensions = model.embedding_dimensions.unwrap_or(profile.dimensions);
        profile.convention = model.embedding_convention.clone();
    } else if settings.ai.recall.embedding.provider == "openai-compatible" {
        settings.ai.recall.embedding.base_url = None;
        settings.ai.recall.embedding.model = None;
        settings.ai.recall.embedding.api_key_ref = None;
    }
    let generation = &mut settings.ai.recall.generation;
    if let Some((model, connection)) = selected(&registry.bindings.summary) {
        generation.base_url = Some(connection.base_url.clone());
        generation.model = Some(model.model.clone());
        generation.auth_mode = if connection.auth_mode == "chatgpt" {
            "api-key".into()
        } else {
            connection.auth_mode.clone()
        };
        generation.api_key_ref = connection.api_key_ref.clone().or_else(|| {
            connection
                .subscription_id
                .as_ref()
                .map(|id| format!("ai.secret.chatgpt.{id}"))
        });
        generation.model_route = Some(connection.into());
    } else {
        generation.base_url = None;
        generation.model = None;
        generation.api_key_ref = None;
        generation.model_route = None;
    }
    let vision = &mut settings.ai.vision;
    if let Some((model, connection)) = selected(&registry.bindings.vision) {
        vision.base_url = Some(connection.base_url.clone());
        vision.model = Some(model.model.clone());
        vision.auth_mode = if connection.auth_mode == "chatgpt" {
            "api-key".into()
        } else {
            connection.auth_mode.clone()
        };
        vision.api_key_ref = connection.api_key_ref.clone().or_else(|| {
            connection
                .subscription_id
                .as_ref()
                .map(|id| format!("ai.secret.chatgpt.{id}"))
        });
        vision.model_route = Some(connection.into());
    } else {
        vision.base_url = None;
        vision.model = None;
        vision.api_key_ref = None;
        vision.model_route = None;
    }
    Ok(())
}
