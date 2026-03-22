//! `clido index` subcommand: build, stats, clear.

use std::path::PathBuf;

use clido_core::load_config;
use clido_index::{BuildOptions, RepoIndex};

use crate::cli::IndexCmd;

pub async fn run_index(cmd: &IndexCmd) -> anyhow::Result<()> {
    let workspace_root = std::env::current_dir()?;
    let db_path = workspace_root.join(".clido").join("index.db");
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    match cmd {
        IndexCmd::Build {
            dir,
            ext,
            include_ignored,
        } => {
            let target: PathBuf = dir.clone().unwrap_or_else(|| workspace_root.clone());
            let exts: Vec<String> = ext.split(',').map(|s| s.trim().to_string()).collect();

            // Load config to get exclude_patterns and config-level include_ignored.
            let cfg = load_config(&workspace_root).ok();
            let config_exclude_patterns = cfg
                .as_ref()
                .map(|c| c.index.exclude_patterns.clone())
                .unwrap_or_default();
            let config_include_ignored = cfg
                .as_ref()
                .map(|c| c.index.include_ignored)
                .unwrap_or(false);

            // CLI flag takes priority; falls back to config value.
            let effective_include_ignored = *include_ignored || config_include_ignored;

            let opts = BuildOptions {
                extensions: exts,
                exclude_patterns: config_exclude_patterns,
                include_ignored: effective_include_ignored,
            };

            let mut index = RepoIndex::open(&db_path)?;
            let stats = index.build_with_options(&target, &opts)?;

            if effective_include_ignored {
                println!(
                    "Indexed {:} files in {} (ignore rules bypassed).",
                    format_count(stats.indexed),
                    target.display()
                );
            } else {
                println!(
                    "Indexed {:} files. Skipped {:} files (ignore rules).",
                    format_count(stats.indexed),
                    format_count(stats.skipped)
                );
            }

            let (files, symbols) = index.stats()?;
            println!("  {} files, {} symbols in index.", files, symbols);
        }
        IndexCmd::Stats => {
            if !db_path.exists() {
                println!("No index found. Run `clido index build` first.");
                return Ok(());
            }
            let index = RepoIndex::open(&db_path)?;
            let (files, symbols) = index.stats()?;
            println!("Index: {} files, {} symbols", files, symbols);
        }
        IndexCmd::Clear => {
            if db_path.exists() {
                std::fs::remove_file(&db_path)?;
                println!("Index cleared.");
            } else {
                println!("No index found.");
            }
        }
    }
    Ok(())
}

/// Format a count with thousands separators.
fn format_count(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}
