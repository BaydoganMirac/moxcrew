//! Inspectable task recipes; a catalog entry is not a tool capability claim.
use serde_json::Value;
use std::sync::OnceLock;

pub fn catalog() -> Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("../assets/abilities.json"))
                .expect("compiled ability catalog")
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
            assert!(guide.starts_with("---\nname: moxcrew-"));
            assert!(guide.contains("## Verify and revise"));
            assert!(guide.contains("outputs/summary.md"));
            assert!(guide.len() < 8_000);
        }
        assert!(find("P1").is_some());
        assert!(find("p1").is_none());
        assert!(find("unknown").is_none());
        assert!(skill_guide("P10").is_none());
        assert!(skill_guide("é").is_none());
    }
}
