//! `clido update-pricing`: show pricing file info (pricing is bundled with binary).

use clido_core::load_pricing;

pub fn run_update_pricing() {
    let (_pricing_table, pricing_path) = load_pricing();
    if let Some(path) = pricing_path {
        let age_str = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| {
                std::time::SystemTime::now()
                    .duration_since(t)
                    .ok()
                    .map(|d| {
                        let days = d.as_secs() / 86400;
                        if days == 0 {
                            "today".to_string()
                        } else if days == 1 {
                            "1 day ago".to_string()
                        } else {
                            format!("{} days ago", days)
                        }
                    })
            })
            .unwrap_or_else(|| "unknown age".to_string());
        println!("Pricing is bundled with the binary and updated with each release.");
        println!("Current pricing file: {}", path.display());
        println!("File last modified: {}", age_str);
        println!("To update pricing, upgrade clido to the latest version.");
    } else {
        println!("Pricing is bundled with the binary and updated with each release.");
        println!("No external pricing.toml found; using built-in defaults.");
        println!("To update pricing, upgrade clido to the latest version.");
    }
}
