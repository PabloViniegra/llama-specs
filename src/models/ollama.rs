use std::collections::HashMap;

use scraper::{Html, Selector};
use serde::Deserialize;

use super::ModelEntry;

const OLLAMA_BASE: &str = "http://localhost:11434";
const LIBRARY_URL: &str = "https://ollama.com/library";

// ---------------------------------------------------------------------------
// Local Ollama API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<InstalledModelInfo>,
}

#[derive(Deserialize)]
struct InstalledModelInfo {
    name: String,
}

/// Checks if the Ollama daemon is reachable.
pub async fn is_running(client: &reqwest::Client) -> bool {
    client
        .get(OLLAMA_BASE)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok()
}

/// Fetches the list of locally installed model names from the Ollama API.
/// Returns an empty list if Ollama is unreachable.
pub async fn fetch_installed(client: &reqwest::Client) -> Vec<String> {
    let url = format!("{OLLAMA_BASE}/api/tags");
    let Ok(resp) = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    else {
        return Vec::new();
    };

    let Ok(tags) = resp.json::<TagsResponse>().await else {
        return Vec::new();
    };

    tags.models.into_iter().map(|m| m.name).collect()
}

// ---------------------------------------------------------------------------
// ollama.com/library catalog scraper
// ---------------------------------------------------------------------------

/// Downloads the Ollama public model library from ollama.com/library,
/// scraping model names, descriptions, and size variants.
///
/// Generates one `ModelEntry` per (model × size tag) pair, e.g.
/// "deepseek-r1:7b", "deepseek-r1:8b", …
///
/// Parameter resolution priority:
/// 1. Lookup in `static_db` (embedded JSON, most reliable)
/// 2. Numeric extraction from size tag ("8b" → 8.0, "270m" → 0.27)
/// 3. Named-tag lookup ("mini" → 3.8, …)
/// 4. `0.0` — verdict will not be computed for this model
pub async fn fetch_library(
    client: &reqwest::Client,
    static_db: &[ModelEntry],
) -> Result<Vec<ModelEntry>, String> {
    let static_map: HashMap<&str, f64> = static_db
        .iter()
        .map(|m| (m.name.as_str(), m.params_b))
        .collect();

    let html = client
        .get(LIBRARY_URL)
        .timeout(std::time::Duration::from_secs(30))
        .header("User-Agent", "Mozilla/5.0 (compatible; llama-specs/0.1)")
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Read error: {e}"))?;

    parse_library_html(&html, &static_map)
}

/// Parses ollama.com/library HTML and returns model entries.
///
/// Uses stable `x-test-*` attributes as anchors — these are part of the
/// site's test harness and are unlikely to change without notice.
fn parse_library_html(html: &str, static_map: &HashMap<&str, f64>) -> Result<Vec<ModelEntry>, String> {
    let document = Html::parse_document(html);

    let model_sel = Selector::parse("li[x-test-model]").unwrap();
    let title_sel = Selector::parse("div[x-test-model-title]").unwrap();
    let desc_sel  = Selector::parse("p.max-w-lg").unwrap();
    let size_sel  = Selector::parse("span[x-test-size]").unwrap();

    let mut entries: Vec<ModelEntry> = Vec::new();

    for model_el in document.select(&model_sel) {
        // Model name from the `title` attribute of div[x-test-model-title].
        let name = match model_el
            .select(&title_sel)
            .next()
            .and_then(|el| el.value().attr("title"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(n) => n.to_owned(),
            None => continue,
        };

        // One-line description from the first matching paragraph.
        let description = model_el
            .select(&desc_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_owned())
            .unwrap_or_default();

        // Collect all size tags (e.g. "1.5b", "7b", "70b").
        let sizes: Vec<String> = model_el
            .select(&size_sel)
            .map(|el| el.text().collect::<String>().trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let family = capitalize_words(&name);

        if sizes.is_empty() {
            // No size variants advertised — add the bare name (params unknown).
            let params_b = resolve_params(&name, static_map);
            entries.push(ModelEntry { name, params_b, family, description });
        } else {
            for size in sizes {
                let model_id = format!("{name}:{size}");
                let params_b = resolve_params(&model_id, static_map);
                entries.push(ModelEntry {
                    name: model_id,
                    params_b,
                    family: family.clone(),
                    description: description.clone(),
                });
            }
        }
    }

    if entries.is_empty() {
        return Err(
            "No models found — ollama.com/library structure may have changed".to_owned(),
        );
    }

    Ok(entries)
}

/// Capitalises each hyphen-separated word in a model name.
/// E.g. "deepseek-r1" → "Deepseek R1", "llama3" → "Llama3".
fn capitalize_words(s: &str) -> String {
    s.split('-')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Parameter resolution helpers
// ---------------------------------------------------------------------------

/// Resolves the number of parameters (in billions) for a model identifier.
///
/// Tries, in order:
/// 1. Exact match in the static database
/// 2. Numeric extraction from the tag (e.g. "8b" → 8.0, "270m" → 0.27)
/// 3. Named-tag lookup (e.g. "mini" → 3.8)
/// 4. Falls back to 0.0 (means "unknown" — no verdict computed)
fn resolve_params(model_id: &str, static_map: &HashMap<&str, f64>) -> f64 {
    // Priority 1: static database override.
    if let Some(&p) = static_map.get(model_id) {
        return p;
    }

    let tag = model_id.split(':').nth(1).unwrap_or("");

    // Priority 2: numeric extraction, e.g. "8b", "70b", "0.5b", "270m".
    if let Some(p) = parse_param_from_tag(tag) {
        return p;
    }

    // Priority 3: well-known named tags.
    named_tag_lookup(tag)
}

/// Extracts a parameter count from a tag string using a simple numeric scan.
/// Accepts patterns like "8b", "70b", "0.5b", "14b-instruct", "270m".
fn parse_param_from_tag(tag: &str) -> Option<f64> {
    // Walk the tag looking for a digit sequence immediately followed by
    // 'b'/'B' (billions) or 'm'/'M' (millions, converted to billions).
    let bytes = tag.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // Collect the full number (may include a decimal point).
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len()
                && let Ok(num) = tag[start..i].parse::<f64>()
                && num > 0.0
            {
                match bytes[i] {
                    b'b' | b'B' => return Some(num),
                    // Millions of parameters → convert to billions.
                    b'm' | b'M' => return Some(num / 1000.0),
                    _ => {}
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Returns a known parameter count for special named tags.
/// Returns 0.0 if the tag is not recognised (treated as "unknown").
fn named_tag_lookup(tag: &str) -> f64 {
    // Match the beginning of the tag so "mini-instruct" also hits "mini".
    let tag_lower = tag.to_lowercase();
    let tag_prefix = tag_lower.split('-').next().unwrap_or(&tag_lower);

    match tag_prefix {
        "mini"   => 3.8,
        "medium" => 14.0,
        "large"  => 34.0,
        "small"  => 7.0,
        "tiny"   => 1.1,
        "nemo"   => 12.0,
        "nano"   => 0.5,
        // 0.0 = unknown; the compat engine treats this as "no verdict".
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer_param() {
        assert_eq!(parse_param_from_tag("8b"), Some(8.0));
        assert_eq!(parse_param_from_tag("70b"), Some(70.0));
        assert_eq!(parse_param_from_tag("8B"), Some(8.0));
    }

    #[test]
    fn parse_decimal_param() {
        assert_eq!(parse_param_from_tag("0.5b"), Some(0.5));
        assert_eq!(parse_param_from_tag("1.5b"), Some(1.5));
        assert_eq!(parse_param_from_tag("3.8b"), Some(3.8));
    }

    #[test]
    fn parse_param_with_suffix() {
        assert_eq!(parse_param_from_tag("8b-instruct"), Some(8.0));
        assert_eq!(parse_param_from_tag("14b-chat"), Some(14.0));
    }

    #[test]
    fn parse_million_param() {
        assert_eq!(parse_param_from_tag("270m"), Some(0.27));
        assert_eq!(parse_param_from_tag("500m"), Some(0.5));
    }

    #[test]
    fn unknown_tag_returns_none() {
        assert_eq!(parse_param_from_tag("latest"), None);
        assert_eq!(parse_param_from_tag("mini"), None);
    }

    #[test]
    fn named_tag_mini() {
        assert_eq!(named_tag_lookup("mini"), 3.8);
        assert_eq!(named_tag_lookup("mini-instruct"), 3.8);
    }

    #[test]
    fn named_tag_unknown() {
        assert_eq!(named_tag_lookup("latest"), 0.0);
        assert_eq!(named_tag_lookup("xyz"), 0.0);
    }

    #[test]
    fn capitalize_words_basic() {
        assert_eq!(capitalize_words("deepseek-r1"), "Deepseek R1");
        assert_eq!(capitalize_words("llama3"), "Llama3");
        assert_eq!(capitalize_words("phi-3-mini"), "Phi 3 Mini");
    }

    #[test]
    fn parse_library_html_basic() {
        let html = r#"<!DOCTYPE html><html><body><ul>
            <li x-test-model>
              <a href="/library/llama3">
                <div x-test-model-title title="llama3">
                  <h2>llama3</h2>
                  <p class="max-w-lg">Meta's Llama 3 model.</p>
                </div>
                <div>
                  <span x-test-size>8b</span>
                  <span x-test-size>70b</span>
                </div>
              </a>
            </li>
        </ul></body></html>"#;

        let map = HashMap::new();
        let result = parse_library_html(html, &map).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "llama3:8b");
        assert_eq!(result[0].params_b, 8.0);
        assert_eq!(result[1].name, "llama3:70b");
        assert_eq!(result[1].params_b, 70.0);
        assert_eq!(result[0].description, "Meta's Llama 3 model.");
    }

    #[test]
    fn parse_library_html_no_sizes_uses_bare_name() {
        let html = r#"<!DOCTYPE html><html><body><ul>
            <li x-test-model>
              <a href="/library/mymodel">
                <div x-test-model-title title="mymodel">
                  <h2>mymodel</h2>
                </div>
              </a>
            </li>
        </ul></body></html>"#;

        let map = HashMap::new();
        let result = parse_library_html(html, &map).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "mymodel");
        assert_eq!(result[0].params_b, 0.0);
    }
}
