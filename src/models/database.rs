use super::ModelEntry;

/// The model catalogue is embedded at compile time so the binary works offline.
static MODELS_JSON: &str = include_str!("../../assets/models.json");

/// Parses and returns the full list of known models.
/// Panics on malformed JSON — this is a developer error, not a runtime error.
pub fn load() -> Vec<ModelEntry> {
    serde_json::from_str(MODELS_JSON).expect("assets/models.json is malformed")
}
