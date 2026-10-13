use super::super::ocr_documents::extraction::FullFrame;
use super::change_protocol::*;

fn frame() -> FullFrame {
    FullFrame {
        path: "/tmp/synthetic.png".into(),
        timestamp_ms: 30_000,
        end_ms: 60_000,
        width: 100,
        height: 100,
        pixel_hash: "synthetic".into(),
        source_ref: "synthetic".into(),
    }
}
fn response(value: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":value.to_string()}}]})
}
#[test]
fn studio_combined_output_validates_each_section_independently() {
    let valid = response(
        serde_json::json!({"ocr":{"blocks":[{"text":"Synthetic literal text"}]},"observation":{"status":"changed","text":"Visible content changed."}}),
    );
    let (ocr, change) = validate_sections(&valid, &frame()).unwrap();
    assert_eq!(ocr.unwrap().text, "Synthetic literal text");
    assert_eq!(change.unwrap().status, ChangeStatus::Changed);
    let bad_ocr = response(
        serde_json::json!({"ocr":{"blocks":[{"text":"Synthetic","confidence":1}]},"observation":{"status":"unchanged","text":null}}),
    );
    let (ocr, change) = validate_sections(&bad_ocr, &frame()).unwrap();
    assert!(ocr.is_none());
    assert_eq!(change.unwrap().status, ChangeStatus::Unchanged);
    let bad_change = response(
        serde_json::json!({"ocr":{"blocks":[]},"observation":{"status":"unchanged","text":"Invented change"}}),
    );
    let (ocr, change) = validate_sections(&bad_change, &frame()).unwrap();
    assert!(ocr.unwrap().text.is_empty());
    assert!(change.is_none());
}
#[test]
fn studio_screen_protocol_rejects_truncation_inference_metadata_and_unknown_outcomes() {
    for value in [
        serde_json::json!({"status":"changed","text":""}),
        serde_json::json!({"status":"initial","text":null}),
        serde_json::json!({"status":"unchanged","text":null,"confidence":1}),
        serde_json::json!({"status":"guess","text":"Synthetic"}),
    ] {
        assert!(validate_change(value).is_err());
    }
    assert!(validate_sections(&serde_json::json!({}), &frame()).is_err());
    let mut truncated =
        response(serde_json::json!({"observation":{"status":"initial","text":"Synthetic"}}));
    truncated["choices"][0]["finish_reason"] = "length".into();
    assert!(validate_sections(&truncated, &frame()).is_err());
    assert!(
        validate_sections(&response(serde_json::json!({"unexpected":true})), &frame()).is_err()
    );
}
