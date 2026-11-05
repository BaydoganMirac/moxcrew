//! Inspectable task recipes; a catalog entry is not a tool capability claim.
use serde_json::{json, Value};
use std::sync::OnceLock;

fn readiness_profile(id: &str) -> Option<Value> {
    let (required_tools, optional_tools, access): (&[&str], &[&str], &[&str]) = match id {
        "R1" | "R2" | "R3" | "R4" | "R5" => (&[], &[], &["sourceAccess"]),
        "D1" => (&["python"], &["libreoffice", "pdftoppm"], &[]),
        "D2" => (
            &["python", "pdftotext", "pdftoppm", "pdfinfo", "pdfunite"],
            &["tesseract"],
            &[],
        ),
        "D3" => (&["python"], &["pdftotext", "tesseract"], &[]),
        "D4" => (&["python"], &[], &[]),
        "D5" => (&["python"], &["libreoffice", "pdftoppm"], &[]),
        "P1" | "P2" | "P3" | "P4" => (&["python"], &["libreoffice", "pdftoppm"], &[]),
        "P5" => (
            &["python", "ffmpeg", "ffprobe", "libreoffice", "pdftoppm"],
            &[],
            &[],
        ),
        "V1" | "V5" => (&["python", "ffmpeg", "ffprobe"], &[], &[]),
        "V4" => (
            &["python", "ffmpeg", "ffprobe", "libreoffice", "pdftoppm"],
            &[],
            &[],
        ),
        "V2" | "V3" => (
            &["python", "ffmpeg", "ffprobe"],
            &["whisper", "whisperModel"],
            &[],
        ),
        "I1" | "I2" | "I4" | "I5" => (&[], &[], &["imageEngine"]),
        "I3" => (&["python", "ffmpeg"], &[], &[]),
        "A1" => (
            &["python", "ffmpeg", "ffprobe", "whisper", "whisperModel"],
            &[],
            &[],
        ),
        "A2" => (&[], &[], &["voiceModelAccess"]),
        "A3" | "A4" => (&["python", "ffmpeg", "ffprobe"], &[], &[]),
        "A5" => (&["python", "ffmpeg", "ffprobe"], &[], &["voiceModelAccess"]),
        "T1" | "T2" | "T3" | "T4" => (&["python"], &[], &[]),
        "T5" => (&["python"], &["pdftotext", "tesseract"], &[]),
        "W1" => (&[], &[], &[]),
        "W2" => (&[], &["python"], &[]),
        "W3" => (&["chrome"], &[], &[]),
        "W4" => (&[], &[], &["appConnection"]),
        "W5" => (&[], &[], &["hostAwake"]),
        _ => return None,
    };
    Some(json!({
        "requiredTools": required_tools,
        "optionalTools": optional_tools,
        "access": access,
    }))
}

pub fn catalog() -> Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let mut catalog: Value = serde_json::from_str(include_str!("../assets/abilities.json"))
                .expect("compiled ability catalog");
            for ability in catalog
                .as_array_mut()
                .expect("compiled ability catalog array")
            {
                let id = ability["id"].as_str().expect("compiled ability id");
                ability["readiness"] = readiness_profile(id).expect("compiled readiness profile");
            }
            catalog
        })
        .clone()
}

pub fn find(id: &str) -> Option<Value> {
    catalog()
        .as_array()?
        .iter()
        .find(|a| a["id"] == id)
        .cloned()
}

pub fn skill_guide(id: &str) -> Option<&'static str> {
    if !matches!(
        id.as_bytes(),
        [
            b'R' | b'D' | b'P' | b'V' | b'I' | b'A' | b'T' | b'W',
            b'1'..=b'5'
        ]
    ) {
        return None;
    }
    Some(match &id[..1] {
        "R" => include_str!("../assets/skills/research/SKILL.md"),
        "D" => include_str!("../assets/skills/documents/SKILL.md"),
        "P" => include_str!("../assets/skills/presentations/SKILL.md"),
        "V" => include_str!("../assets/skills/video/SKILL.md"),
        "I" => include_str!("../assets/skills/images/SKILL.md"),
        "A" => include_str!("../assets/skills/audio/SKILL.md"),
        "T" => include_str!("../assets/skills/data/SKILL.md"),
        "W" => include_str!("../assets/skills/web/SKILL.md"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn release_recipes_have_stable_unique_ids_and_localized_starters() {
        let entries = catalog();
        let entries = entries.as_array().unwrap();
        let ids: HashSet<_> = entries.iter().map(|a| a["id"].as_str().unwrap()).collect();
        let expected: HashSet<_> = "RDPVIATW"
            .chars()
            .flat_map(|p| (1..=5).map(move |n| format!("{p}{n}")))
            .collect();
        assert_eq!(entries.len(), 40);
        assert_eq!(ids.len(), expected.len());
        assert!(expected.iter().all(|id| ids.contains(id.as_str())));
        for entry in entries {
            for field in ["name", "description", "starter"] {
                for language in ["en", "tr"] {
                    assert!(!entry[field][language].as_str().unwrap().trim().is_empty());
                }
            }
            assert!(entry["instruction"].as_str().unwrap().len() < 1000);
            let guide = skill_guide(entry["id"].as_str().unwrap()).unwrap();
            let mut lines = guide.lines();
            assert_eq!(lines.next(), Some("---"));
            assert!(lines
                .next()
                .is_some_and(|line| line.starts_with("name: moxcrew-")));
            assert!(guide.contains("## Verify and revise"));
            assert!(guide.contains("outputs/summary.md"));
            assert!(guide.len() < 8_000);
            let readiness = &entry["readiness"];
            let known_tools: HashSet<_> = [
                "python",
                "ffmpeg",
                "ffprobe",
                "pdftotext",
                "pdftoppm",
                "pdfinfo",
                "pdfunite",
                "tesseract",
                "whisper",
                "whisperModel",
                "libreoffice",
                "chrome",
            ]
            .into_iter()
            .collect();
            for field in ["requiredTools", "optionalTools"] {
                let tools = readiness[field].as_array().unwrap();
                assert!(tools
                    .iter()
                    .all(|tool| known_tools
                        .contains(tool.as_str().expect("readiness tool identifier"))));
            }
            let known_access: HashSet<_> = [
                "sourceAccess",
                "imageEngine",
                "voiceModelAccess",
                "appConnection",
                "hostAwake",
            ]
            .into_iter()
            .collect();
            assert!(readiness["access"]
                .as_array()
                .unwrap()
                .iter()
                .all(|access| {
                    known_access.contains(access.as_str().expect("readiness access identifier"))
                }));
        }
        assert!(find("P1").is_some());
        assert!(find("p1").is_none());
        assert!(find("unknown").is_none());
        assert!(skill_guide("P10").is_none());
        assert!(skill_guide("é").is_none());
        assert!(readiness_profile("P1").is_some());
        assert!(readiness_profile("P10").is_none());
    }
}
