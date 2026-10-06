//! Explicit draft-provider probe. Only this generated image can be sent.
use super::super::*;
use super::{ocr_documents, visual, TempProcessingDirectory};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestVisionProviderInput {
    pub vision: settings::AiVisionProviderSettings,
    pub key_draft: Option<String>,
    pub ocr: bool,
    pub observations: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisionProviderTestResult {
    pub checks: Vec<VisionProviderCheck>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisionProviderCheck {
    pub kind: &'static str,
    pub status: &'static str,
    pub error_code: Option<String>,
    pub elapsed_ms: u64,
}

#[tauri::command(rename = "testVisionProvider")]
pub(crate) async fn test_vision_provider(
    app: AppHandle,
    input: TestVisionProviderInput,
) -> Result<VisionProviderTestResult, AiError> {
    let mut draft =
        settings::load_app_settings(&app.state::<AppStorage>()).map_err(settings_error)?;
    draft.ai.vision = input.vision;
    settings::validate_settings(draft.clone()).map_err(|_| {
        ai_error(
            "ai_vision_profile_invalid",
            "Review the Vision endpoint, model and authentication fields.",
        )
    })?;
    let config = if draft.ai.vision.auth_mode == "none"
        || input
            .key_draft
            .as_deref()
            .is_none_or(|key| key.trim().is_empty())
    {
        visual::vision_request_config_for_ocr(&app.state::<AppStorage>(), &draft)?
    } else {
        visual::VisionRequestConfig {
            api_key: input.key_draft.unwrap_or_default().trim().into(),
            base_url: draft.ai.vision.base_url.clone().unwrap_or_default(),
            model: draft.ai.vision.model.clone().unwrap_or_default(),
            provider: "openai-compatible".into(),
        }
    };
    if draft.ai.vision.provider != "openai-compatible"
        || config.model.trim().is_empty()
        || reqwest::Url::parse(&config.base_url).is_err()
    {
        return Err(ai_error(
            "ai_vision_remote_unconfigured",
            "Enter the Vision endpoint, model and authentication before testing.",
        ));
    }
    let directory = TempProcessingDirectory::new(
        std::env::temp_dir().join(format!("daydream-vision-test-{}", Uuid::now_v7())),
    );
    fs::create_dir_all(directory.path()).map_err(|_| {
        ai_error(
            "ai_vision_test_fixture_failed",
            "The test image could not be prepared.",
        )
    })?;
    let path = directory.path().join("test.png");
    write_test_image(&path)?;
    let mut checks = Vec::new();
    if input.ocr {
        checks.push(
            probe(
                "ocr",
                ocr_documents::probe_provider(&config, &path, Arc::new(AtomicBool::new(false))),
            )
            .await,
        );
    }
    if input.observations || !input.ocr {
        checks.push(
            probe(
                "observations",
                visual::probe_provider(&config, &path, Arc::new(AtomicBool::new(false))),
            )
            .await,
        );
    }
    Ok(VisionProviderTestResult { checks })
}

async fn probe(
    kind: &'static str,
    request: impl std::future::Future<Output = Result<(), AiError>>,
) -> VisionProviderCheck {
    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(20), request).await;
    let error_code = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.code),
        Err(_) => Some("ai_vision_test_timeout".into()),
    };
    VisionProviderCheck {
        kind,
        status: if error_code.is_some() {
            "failed"
        } else {
            "succeeded"
        },
        error_code,
        elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
    }
}

fn write_test_image(path: &Path) -> Result<(), AiError> {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut pixels = vec![255u8; 128 * 64 * 3];
        // Five-pixel glyphs spell TEST, at a readable 5x scale.
        let glyphs = [
            [31, 4, 4, 4, 4, 4, 4],
            [31, 16, 16, 30, 16, 16, 31],
            [15, 16, 16, 14, 1, 1, 30],
            [31, 4, 4, 4, 4, 4, 4],
        ];
        for (letter, rows) in glyphs.iter().enumerate() {
            for (y, row) in rows.iter().enumerate() {
                for x in 0..5 {
                    if row & (1 << (4 - x)) == 0 {
                        continue;
                    }
                    for dy in 0..5 {
                        for dx in 0..5 {
                            let offset =
                                ((12 + y * 5 + dy) * 128 + 4 + letter * 30 + x * 5 + dx) * 3;
                            pixels[offset..offset + 3].fill(0);
                        }
                    }
                }
            }
        }
        let mut encoder = png::Encoder::new(File::create(path)?, 128, 64);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&pixels)?;
        Ok(())
    })();
    result.map_err(|_| {
        ai_error(
            "ai_vision_test_fixture_failed",
            "The test image could not be prepared.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    #[test]
    fn anonymous_probe_sends_only_generated_pixels_and_reports_provider_failures() {
        tauri::async_runtime::block_on(async {
            for status in [200, 401] {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let address = listener.local_addr().unwrap();
                let server = std::thread::spawn(move || {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let mut buffer = [0u8; 4096];
                    let (header_end, length) = loop {
                        let count = stream.read(&mut buffer).unwrap();
                        assert!(count > 0);
                        bytes.extend_from_slice(&buffer[..count]);
                        if let Some(index) =
                            bytes.windows(4).position(|window| window == b"\r\n\r\n")
                        {
                            let headers = String::from_utf8_lossy(&bytes[..index]);
                            let length = headers
                                .lines()
                                .find_map(|line| {
                                    line.split_once(':')
                                        .filter(|(name, _)| {
                                            name.eq_ignore_ascii_case("content-length")
                                        })
                                        .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap();
                            break (index + 4, length);
                        }
                    };
                    while bytes.len() < header_end + length {
                        let count = stream.read(&mut buffer).unwrap();
                        assert!(count > 0);
                        bytes.extend_from_slice(&buffer[..count]);
                    }
                    assert!(!String::from_utf8_lossy(&bytes[..header_end])
                        .to_ascii_lowercase()
                        .contains("authorization:"));
                    let request: serde_json::Value =
                        serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                    let uri = request["messages"][1]["content"][1]["image_url"]["url"]
                        .as_str()
                        .unwrap();
                    let image = BASE64_STANDARD
                        .decode(uri.strip_prefix("data:image/png;base64,").unwrap())
                        .unwrap();
                    assert_eq!(&image[..8], b"\x89PNG\r\n\x1a\n");
                    assert_eq!(u32::from_be_bytes(image[16..20].try_into().unwrap()), 128);
                    assert_eq!(u32::from_be_bytes(image[20..24].try_into().unwrap()), 64);
                    let body=serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"blocks\":[{\"text\":\"TEST\"}]}"}}]}).to_string();
                    write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                });
                let directory = TempProcessingDirectory::new(
                    std::env::temp_dir().join(format!("daydream-vision-test-{}", Uuid::now_v7())),
                );
                fs::create_dir_all(directory.path()).unwrap();
                let path = directory.path().join("test.png");
                write_test_image(&path).unwrap();
                let config = visual::VisionRequestConfig {
                    api_key: String::new(),
                    base_url: format!("http://{address}"),
                    model: "synthetic".into(),
                    provider: "openai-compatible".into(),
                };
                let result = probe(
                    "ocr",
                    ocr_documents::probe_provider(&config, &path, Arc::new(AtomicBool::new(false))),
                )
                .await;
                assert_eq!(
                    result.status,
                    if status == 200 { "succeeded" } else { "failed" }
                );
                if status == 401 {
                    assert_eq!(
                        result.error_code.as_deref(),
                        Some("ai_ocr_vision_auth_failed")
                    );
                }
                server.join().unwrap();
            }
        });
    }
}
