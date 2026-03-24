//! Load pricing.toml and compute cost from Usage.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::Usage;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelPricingEntry {
    pub name: String,
    pub provider: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    #[serde(default)]
    pub cache_creation_per_mtok: Option<f64>,
    #[serde(default)]
    pub cache_read_per_mtok: Option<f64>,
    #[serde(default)]
    pub context_window: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PricingToml {
    #[serde(default)]
    pub model: HashMap<String, ModelPricingEntry>,
}

/// Pricing table keyed by model id.
#[derive(Debug, Clone, Default)]
pub struct PricingTable {
    pub models: HashMap<String, ModelPricingEntry>,
}

const STALENESS_DAYS: u64 = 90;

fn pricing_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "clido")
        .map(|d: directories::ProjectDirs| d.config_dir().join("pricing.toml"))
}

/// Load pricing from config dir. If file is older than 90 days, log a warning.
/// Returns default (empty) table if file absent; uses default per-model cost when model not in table.
pub fn load_pricing() -> (PricingTable, Option<std::path::PathBuf>) {
    let path = match pricing_path() {
        Some(p) => p,
        None => return (PricingTable::default(), None),
    };
    if !path.exists() {
        return (PricingTable::default(), None);
    }
    let table = match std::fs::read_to_string(&path) {
        Ok(s) => match toml::from_str::<PricingToml>(&s) {
            Ok(t) => PricingTable { models: t.model },
            Err(_) => PricingTable::default(),
        },
        Err(_) => PricingTable::default(),
    };

    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let (Ok(now_dur), Ok(mod_dur)) = (
                SystemTime::now().duration_since(UNIX_EPOCH),
                modified.duration_since(UNIX_EPOCH),
            ) {
                let age_secs = now_dur.as_secs().saturating_sub(mod_dur.as_secs());
                if age_secs > STALENESS_DAYS * 86400 {
                    tracing::warn!(
                        "pricing.toml is older than {} days; consider running clido update-pricing (or doctor)",
                        STALENESS_DAYS
                    );
                }
            }
        }
    }

    (table, Some(path))
}

/// Compute cost in USD for the given usage and model. Uses pricing table if model found; else fallback defaults.
pub fn compute_cost_usd(usage: &Usage, model_id: &str, table: &PricingTable) -> f64 {
    let (input_per_mtok, output_per_mtok, cache_creation, cache_read) = table
        .models
        .get(model_id)
        .map(|e| {
            (
                e.input_per_mtok,
                e.output_per_mtok,
                e.cache_creation_per_mtok.unwrap_or(e.input_per_mtok * 1.25),
                e.cache_read_per_mtok.unwrap_or(e.input_per_mtok * 0.10),
            )
        })
        .unwrap_or((3.0, 15.0, 3.75, 0.3)); // fallback USD per million tokens

    let input = (usage.input_tokens as f64 / 1_000_000.0) * input_per_mtok;
    let output = (usage.output_tokens as f64 / 1_000_000.0) * output_per_mtok;
    let cache_creation_cost = usage
        .cache_creation_input_tokens
        .map(|t| (t as f64 / 1_000_000.0) * cache_creation)
        .unwrap_or(0.0);
    let cache_read_cost = usage
        .cache_read_input_tokens
        .map(|t| (t as f64 / 1_000_000.0) * cache_read)
        .unwrap_or(0.0);

    input + output + cache_creation_cost + cache_read_cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Usage;

    fn make_usage(input: u64, output: u64) -> Usage {
        Usage {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        }
    }

    fn make_usage_with_cache(input: u64, output: u64, create: u64, read: u64) -> Usage {
        Usage {
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: Some(create),
            cache_read_input_tokens: Some(read),
        }
    }

    #[test]
    fn cost_fallback_rates_known_values() {
        // Fallback: $3/mtok input, $15/mtok output
        let usage = make_usage(1_000_000, 1_000_000);
        let table = PricingTable::default();
        let cost = compute_cost_usd(&usage, "unknown-model", &table);
        assert!((cost - 18.0).abs() < 0.001, "expected $18, got {}", cost);
    }

    #[test]
    fn cost_zero_tokens() {
        let usage = make_usage(0, 0);
        let table = PricingTable::default();
        let cost = compute_cost_usd(&usage, "any", &table);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn cost_from_table_entry() {
        let mut models = std::collections::HashMap::new();
        models.insert(
            "claude-3-haiku".to_string(),
            ModelPricingEntry {
                name: "claude-3-haiku".to_string(),
                provider: "anthropic".to_string(),
                input_per_mtok: 0.25,
                output_per_mtok: 1.25,
                cache_creation_per_mtok: None,
                cache_read_per_mtok: None,
                context_window: None,
            },
        );
        let table = PricingTable { models };
        let usage = make_usage(1_000_000, 1_000_000);
        let cost = compute_cost_usd(&usage, "claude-3-haiku", &table);
        assert!((cost - 1.50).abs() < 0.001, "expected $1.50, got {}", cost);
    }

    #[test]
    fn cost_with_cache_tokens_fallback() {
        // cache_creation = input * 1.25 = 3.75/mtok; cache_read = input * 0.10 = 0.30/mtok
        let usage = make_usage_with_cache(0, 0, 1_000_000, 1_000_000);
        let table = PricingTable::default();
        let cost = compute_cost_usd(&usage, "unknown", &table);
        let expected = 3.75 + 0.30;
        assert!(
            (cost - expected).abs() < 0.001,
            "expected {}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn cost_with_explicit_cache_rates_in_table() {
        let mut models = std::collections::HashMap::new();
        models.insert(
            "gpt-4".to_string(),
            ModelPricingEntry {
                name: "gpt-4".to_string(),
                provider: "openai".to_string(),
                input_per_mtok: 10.0,
                output_per_mtok: 30.0,
                cache_creation_per_mtok: Some(5.0),
                cache_read_per_mtok: Some(1.0),
                context_window: None,
            },
        );
        let table = PricingTable { models };
        let usage = make_usage_with_cache(0, 0, 2_000_000, 500_000);
        let cost = compute_cost_usd(&usage, "gpt-4", &table);
        let expected = 2.0 * 5.0 + 0.5 * 1.0; // 10 + 0.5
        assert!(
            (cost - expected).abs() < 0.001,
            "expected {}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn load_pricing_returns_default_when_no_file() {
        // Should not panic; may return empty table
        let (table, _path) = load_pricing();
        // table.models may or may not be empty depending on whether the user has a pricing file
        let _ = table; // just ensure no panic
    }

    // ── PricingToml deserialization ────────────────────────────────────────

    #[test]
    fn pricing_toml_parses_model_entries() {
        let toml_str = r#"
[model.claude-3-haiku]
name = "claude-3-haiku"
provider = "anthropic"
input_per_mtok = 0.25
output_per_mtok = 1.25
context_window = 200000
"#;
        let parsed: PricingToml = toml::from_str(toml_str).unwrap();
        assert!(parsed.model.contains_key("claude-3-haiku"));
        let entry = &parsed.model["claude-3-haiku"];
        assert_eq!(entry.input_per_mtok, 0.25);
        assert_eq!(entry.output_per_mtok, 1.25);
        assert_eq!(entry.context_window, Some(200000));
    }

    #[test]
    fn pricing_toml_with_cache_rates() {
        let toml_str = r#"
[model.claude-3-5-sonnet]
name = "claude-3-5-sonnet"
provider = "anthropic"
input_per_mtok = 3.0
output_per_mtok = 15.0
cache_creation_per_mtok = 3.75
cache_read_per_mtok = 0.30
"#;
        let parsed: PricingToml = toml::from_str(toml_str).unwrap();
        let entry = &parsed.model["claude-3-5-sonnet"];
        assert_eq!(entry.cache_creation_per_mtok, Some(3.75));
        assert_eq!(entry.cache_read_per_mtok, Some(0.30));
    }

    #[test]
    fn pricing_toml_empty_parses_ok() {
        let toml_str = "";
        let parsed: PricingToml = toml::from_str(toml_str).unwrap();
        assert!(parsed.model.is_empty());
    }

    #[test]
    fn pricing_table_default_has_no_models() {
        let table = PricingTable::default();
        assert!(table.models.is_empty());
    }

    #[test]
    fn cost_only_input_tokens() {
        let usage = Usage {
            input_tokens: 1_000_000,
            output_tokens: 0,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };
        let table = PricingTable::default(); // uses fallback: $3/mtok input
        let cost = compute_cost_usd(&usage, "unknown", &table);
        assert!((cost - 3.0).abs() < 0.001);
    }

    #[test]
    fn cost_only_output_tokens() {
        let usage = Usage {
            input_tokens: 0,
            output_tokens: 1_000_000,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        };
        let table = PricingTable::default(); // uses fallback: $15/mtok output
        let cost = compute_cost_usd(&usage, "unknown", &table);
        assert!((cost - 15.0).abs() < 0.001);
    }
}
