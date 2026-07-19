//! Aquarium state serialization, ported from AquariumStateManager.cpp.
//!
//! The C++ firmware writes one JSON document to LittleFS; the Rust port
//! stores one small JSON document per fish (NVS entries are size-limited),
//! but the per-fish JSON shape is kept identical.

use crate::color::CHsv;
use crate::fish::FishDefinition;

/// JSON shape of one saved fish (same keys as the C++ ArduinoJson document).
#[derive(serde::Serialize, serde::Deserialize)]
struct FishJson {
    age: f32,
    health: f32,
    #[serde(rename = "bodyType")]
    body_type: String,
    #[serde(rename = "headType")]
    head_type: String,
    #[serde(rename = "tailType")]
    tail_type: String,
    #[serde(rename = "finType")]
    fin_type: String,
    #[serde(rename = "motionType")]
    motion_type: String,
    colors: Vec<ColorJson>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ColorJson {
    h: u8,
    s: u8,
    v: u8,
}

impl From<&FishDefinition> for FishJson {
    fn from(def: &FishDefinition) -> Self {
        Self {
            age: def.age,
            health: def.health,
            body_type: def.body_type.clone(),
            head_type: def.head_type.clone(),
            tail_type: def.tail_type.clone(),
            fin_type: def.fin_type.clone(),
            motion_type: def.motion_type.clone(),
            colors: def
                .colors
                .iter()
                .map(|c| ColorJson {
                    h: c.h,
                    s: c.s,
                    v: c.v,
                })
                .collect(),
        }
    }
}

impl From<FishJson> for FishDefinition {
    fn from(json: FishJson) -> Self {
        Self {
            age: json.age,
            health: json.health,
            body_type: json.body_type,
            head_type: json.head_type,
            tail_type: json.tail_type,
            fin_type: json.fin_type,
            motion_type: json.motion_type,
            colors: json
                .colors
                .into_iter()
                .map(|c| CHsv::new(c.h, c.s, c.v))
                .collect(),
        }
    }
}

pub fn fish_to_json(def: &FishDefinition) -> String {
    let json = FishJson::from(def);
    serde_json::to_string(&json).unwrap_or_default()
}

pub fn fish_from_json(data: &str) -> Option<FishDefinition> {
    serde_json::from_str::<FishJson>(data)
        .ok()
        .map(FishDefinition::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FishDefinition {
        FishDefinition {
            age: 0.66,
            health: 0.95,
            body_type: "Fish".into(),
            head_type: "FrogHead".into(),
            tail_type: "CurvyTail".into(),
            fin_type: "RoundFin".into(),
            motion_type: "Fish".into(),
            colors: vec![CHsv::new(10, 130, 255), CHsv::new(95, 130, 255)],
        }
    }

    #[test]
    fn json_roundtrip() {
        let def = sample();
        let text = fish_to_json(&def);
        let restored = fish_from_json(&text).expect("should parse");
        assert_eq!(restored, def);
    }

    #[test]
    fn json_keys_match_cpp_layout() {
        let text = fish_to_json(&sample());
        for key in [
            "age",
            "health",
            "bodyType",
            "headType",
            "tailType",
            "finType",
            "motionType",
            "colors",
        ] {
            assert!(
                text.contains(&format!("\"{key}\"")),
                "missing key {key}: {text}"
            );
        }
        assert!(text.contains("\"h\":10"));
    }

    #[test]
    fn parses_cpp_document() {
        // Document as written by the ArduinoJson-based C++ firmware.
        let cpp = r#"{"age":0.7,"health":1,"bodyType":"Star","headType":"TriangleHead","tailType":"noTail","finType":"TriangleFin","motionType":"Star","colors":[{"h":42,"s":115,"v":255},{"h":127,"s":115,"v":255},{"h":212,"s":115,"v":255}]}"#;
        let def = fish_from_json(cpp).expect("cpp json should parse");
        assert_eq!(def.body_type, "Star");
        assert_eq!(def.colors.len(), 3);
        assert_eq!(def.colors[1], CHsv::new(127, 115, 255));
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(fish_from_json("not json").is_none());
        assert!(fish_from_json("{}").is_none());
    }
}
