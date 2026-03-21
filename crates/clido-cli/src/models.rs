//! `clido list-models`: list known models per provider.

const ANTHROPIC_MODELS: &[&str] = &[
    "claude-sonnet-4-5",
    "claude-opus-4-5",
    "claude-haiku-4-5-20251001",
    "claude-3-5-sonnet-20241022",
    "claude-3-5-haiku-20241022",
];

const OPENROUTER_MODELS: &[&str] = &[
    "anthropic/claude-3-5-sonnet",
    "anthropic/claude-haiku-3-5",
    "openai/gpt-4o",
    "openai/gpt-4o-mini",
    "google/gemini-2.0-flash",
];

const LOCAL_MODELS: &[&str] = &["llama3.2", "llama3.1", "mistral", "codellama"];

pub fn run_list_models(provider: Option<&str>, json: bool) {
    let providers: &[(&str, &[&str])] = &[
        ("anthropic", ANTHROPIC_MODELS),
        ("openrouter", OPENROUTER_MODELS),
        ("local", LOCAL_MODELS),
    ];

    let filtered: Vec<(&str, &[&str])> = if let Some(p) = provider {
        providers
            .iter()
            .filter(|(name, _)| *name == p)
            .copied()
            .collect()
    } else {
        providers.to_vec()
    };

    if json {
        let obj: serde_json::Value = serde_json::Value::Object(
            filtered
                .iter()
                .map(|(name, models)| {
                    (
                        name.to_string(),
                        serde_json::Value::Array(
                            models
                                .iter()
                                .map(|m| serde_json::Value::String(m.to_string()))
                                .collect(),
                        ),
                    )
                })
                .collect(),
        );
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    } else {
        for (name, models) in &filtered {
            println!("{}:", name);
            for m in *models {
                println!("  {}", m);
            }
        }
    }
}
