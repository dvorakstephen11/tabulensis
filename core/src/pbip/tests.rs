use super::*;

#[test]
fn balanced_sorts_json_keys_deterministically() {
    let input = r#"{"b":1,"a":2}"#;
    let (out, applied) = normalize_doc_text(PbipDocType::Pbir, input, PbipNormalizationProfile::Balanced)
        .expect("normalize");
    assert!(applied.summary.contains("sorted keys"));
    // Pretty JSON output should place "a" before "b".
    let first_a = out.find("\"a\"").expect("a key");
    let first_b = out.find("\"b\"").expect("b key");
    assert!(first_a < first_b, "expected sorted keys: {out}");
}

#[test]
fn balanced_guid_normalization_is_conservative() {
    let guid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let input = format!(r#"{{"id":"{guid}","other":"{guid}"}}"#);
    let (out, _applied) =
        normalize_doc_text(PbipDocType::Pbir, &input, PbipNormalizationProfile::Balanced)
            .expect("normalize");
    assert!(out.contains("GUID_0001"), "expected id GUID placeholder: {out}");
    // "other" is not an id-like key, so it should retain the original GUID string in Balanced mode.
    assert!(out.contains(guid), "expected other GUID preserved: {out}");
}

#[test]
fn aggressive_guid_normalization_rewrites_all_guid_like_strings() {
    let guid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let input = format!(r#"{{"id":"{guid}","other":"{guid}"}}"#);
    let (out, _applied) =
        normalize_doc_text(PbipDocType::Pbir, &input, PbipNormalizationProfile::Aggressive)
            .expect("normalize");
    assert!(!out.contains(guid), "expected GUID to be normalized away: {out}");
    assert!(out.contains("GUID_0001"), "expected GUID placeholder: {out}");
}

#[test]
fn diff_classifies_added_removed_modified_unchanged() {
    let old = PbipProjectSnapshot {
        docs: vec![PbipDocRecord {
            path: "report/definition.pbir".to_string(),
            doc_type: PbipDocType::Pbir,
            snapshot: PbipDocSnapshot {
                doc_type: PbipDocType::Pbir,
                normalized_text: "{\n  \"a\": 1\n}\n".to_string(),
                hash: 1,
                error: None,
                normalization_applied: Some("test".to_string()),
            },
        }],
        profile: PbipNormalizationProfile::Balanced,
        profile_summary: "x".to_string(),
    };

    let new = PbipProjectSnapshot {
        docs: vec![
            PbipDocRecord {
                path: "report/definition.pbir".to_string(),
                doc_type: PbipDocType::Pbir,
                snapshot: PbipDocSnapshot {
                    doc_type: PbipDocType::Pbir,
                    normalized_text: "{\n  \"a\": 2\n}\n".to_string(),
                    hash: 2,
                    error: None,
                    normalization_applied: Some("test".to_string()),
                },
            },
            PbipDocRecord {
                path: "model/model.tmdl".to_string(),
                doc_type: PbipDocType::Tmdl,
                snapshot: PbipDocSnapshot {
                    doc_type: PbipDocType::Tmdl,
                    normalized_text: "model\n".to_string(),
                    hash: 3,
                    error: None,
                    normalization_applied: Some("test".to_string()),
                },
            },
        ],
        profile: PbipNormalizationProfile::Balanced,
        profile_summary: "x".to_string(),
    };

    let report = diff_snapshots(&old, &new);
    assert_eq!(report.docs.len(), 2);
    let mut kinds = report
        .docs
        .iter()
        .map(|d| (d.path.as_str(), d.change_kind))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        kinds.remove("report/definition.pbir"),
        Some(PbipChangeKind::Modified)
    );
    assert_eq!(
        kinds.remove("model/model.tmdl"),
        Some(PbipChangeKind::Added)
    );
}

