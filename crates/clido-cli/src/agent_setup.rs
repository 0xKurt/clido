//! Shared agent setup: config loading, provider, registry, permissions.
//! Used by both the single-shot runner and the REPL to avoid duplication.

use clido_agent::AskUser;
use clido_core::{
    agent_config_from_loaded, load_config, load_pricing, AgentConfig, LoadedConfig, PermissionMode,
    PricingTable,
};
use clido_tools::{default_registry_with_options, McpTool, ToolRegistry};
use std::io::{self, IsTerminal};
use std::path::Path;
use std::sync::Arc;

use crate::cli::Cli;
use crate::errors::CliError;
use crate::git_context::GitContext;
use crate::provider::{make_provider, StdinAskUser};
use crate::spawn_tools::{SpawnReviewerTool, SpawnWorkerTool};

pub struct AgentSetup {
    pub provider: Arc<dyn clido_providers::ModelProvider>,
    pub registry: ToolRegistry,
    pub config: AgentConfig,
    pub ask_user: Option<Arc<dyn AskUser>>,
    pub pricing_table: PricingTable,
}

impl AgentSetup {
    pub fn build(cli: &Cli, workspace_root: &Path) -> Result<Self, anyhow::Error> {
        let loaded = load_config(workspace_root).map_err(|e| CliError::Usage(e.to_string()))?;
        let (pricing_table, _) = load_pricing();
        let profile_name = cli
            .profile
            .as_deref()
            .unwrap_or(loaded.default_profile.as_str());
        let profile = loaded
            .get_profile(profile_name)
            .map_err(|e| CliError::Usage(e.to_string()))?;
        LoadedConfig::validate_provider(&profile.provider)
            .map_err(|e| CliError::Usage(e.to_string()))?;

        let provider = make_provider(
            profile_name,
            profile,
            cli.provider.as_deref(),
            cli.model.as_deref(),
        )
        .map_err(CliError::Usage)?;

        let mut registry = build_registry(cli, &loaded, workspace_root)?;
        registry = load_mcp_tools(cli, registry);

        let permission_mode = parse_permission_mode(cli.permission_mode.as_deref());

        let system_prompt = assemble_system_prompt(cli)?;

        let mut config = agent_config_from_loaded(
            &loaded,
            profile_name,
            cli.max_turns,
            cli.max_budget_usd,
            cli.model.clone(),
            Some(system_prompt),
            Some(permission_mode),
            cli.quiet,
            cli.max_parallel_tools,
        )
        .map_err(|e| CliError::Usage(e.to_string()))?;

        // Override with [agents.main] if present (newer config format).
        let provider = if let Some(main_slot) = &loaded.agents.main {
            let new_provider = build_provider_from_slot(main_slot).map_err(CliError::Usage)?;
            config.model = main_slot.model.clone();
            new_provider
        } else {
            provider
        };

        // Build worker provider if configured (per-profile slot takes priority over global).
        let worker_provider: Option<Arc<dyn clido_providers::ModelProvider>> = loaded
            .effective_slot_for_profile("worker", profile_name)
            .and_then(|slot| {
                build_provider_from_slot(slot)
                    .map_err(|e| {
                        eprintln!("Warning: worker provider failed to build: {}", e);
                    })
                    .ok()
            });

        let reviewer_provider: Option<Arc<dyn clido_providers::ModelProvider>> = loaded
            .effective_slot_for_profile("reviewer", profile_name)
            .and_then(|slot| {
                build_provider_from_slot(slot)
                    .map_err(|e| {
                        eprintln!("Warning: reviewer provider failed to build: {}", e);
                    })
                    .ok()
            });

        // Register sub-agent tools if configured.
        if let Some(ref wp) = worker_provider {
            let worker_config = if let Some(ws) = &loaded.agents.worker {
                let mut wc = config.clone();
                wc.model = ws.model.clone();
                wc
            } else {
                config.clone()
            };
            registry.register(SpawnWorkerTool::new(
                wp.clone(),
                worker_config,
                workspace_root.to_path_buf(),
            ));
        }
        if let Some(ref rp) = reviewer_provider {
            let reviewer_config = if let Some(rs) = &loaded.agents.reviewer {
                let mut rc = config.clone();
                rc.model = rs.model.clone();
                rc
            } else {
                config.clone()
            };
            registry.register(SpawnReviewerTool::new(
                rp.clone(),
                reviewer_config,
                workspace_root.to_path_buf(),
            ));
        }

        // Inject sub-agent routing instructions if sub-agents are configured.
        let has_worker = loaded.agents.worker.is_some();
        let has_reviewer = loaded.agents.reviewer.is_some();
        if has_worker || has_reviewer {
            let routing = build_routing_instructions(has_worker, has_reviewer);
            if let Some(ref mut sp) = config.system_prompt {
                *sp = format!("{}\n\n{}", sp, routing);
            }
        }

        // Inject project rules into system prompt
        let rules_file_path = cli
            .rules_file
            .as_deref()
            .or_else(|| config.rules_file.as_ref().map(|s| Path::new(s.as_str())));
        let rules = clido_context::load_and_assemble_rules(
            workspace_root,
            cli.no_rules || config.no_rules,
            rules_file_path,
        );
        if !rules.is_empty() {
            if let Some(ref mut sp) = config.system_prompt {
                *sp = format!("{}\n\n{}", rules, sp);
            }
        }

        // Inject git context into the system prompt if the working directory is a git repo.
        if let Some(git_ctx) = GitContext::discover(workspace_root) {
            if let Some(ref mut sp) = config.system_prompt {
                *sp = format!("{}\n\n{}", sp, git_ctx.to_prompt_section());
            }
        }

        if config.max_context_tokens.is_none() {
            if let Some(entry) = pricing_table.models.get(&config.model) {
                if let Some(cw) = entry.context_window {
                    config.max_context_tokens = Some(cw);
                }
            }
        }

        let ask_user: Option<Arc<dyn AskUser>> =
            if permission_mode == PermissionMode::Default && io::stdin().is_terminal() {
                Some(Arc::new(StdinAskUser))
            } else {
                None
            };

        Ok(AgentSetup {
            provider,
            registry,
            config,
            ask_user,
            pricing_table,
        })
    }
}

fn build_provider_from_slot(
    slot: &clido_core::AgentSlotConfig,
) -> Result<Arc<dyn clido_providers::ModelProvider>, String> {
    let api_key = slot
        .api_key
        .clone()
        .or_else(|| {
            slot.api_key_env
                .as_ref()
                .and_then(|e| std::env::var(e).ok())
        })
        .unwrap_or_default();
    clido_providers::build_provider(
        &slot.provider,
        api_key,
        slot.model.clone(),
        slot.base_url.as_deref(),
    )
    .map_err(|e| e.to_string())
}

fn build_routing_instructions(has_worker: bool, has_reviewer: bool) -> String {
    let mut lines = vec!["## Sub-Agent Routing".to_string(), String::new()];
    if has_worker {
        lines.push("You have access to `SpawnWorker` tool. Use it for mechanical subtasks:".into());
        lines.push("- Selecting relevant files from a list".into());
        lines.push("- Summarizing file content or chunks".into());
        lines.push("- Extracting structured fields from text".into());
        lines.push("- Formatting or normalizing output".into());
        lines.push("Pass only the minimal context needed — never the full conversation.".into());
        lines.push(String::new());
    }
    if has_reviewer {
        lines.push("You have access to `SpawnReviewer` tool. Use it when:".into());
        lines.push("- The task is complete and a final quality check is warranted.".into());
        lines.push("Only invoke if you are uncertain about the output quality.".into());
        lines.push(String::new());
    }
    lines.push("Sub-agent rules:".into());
    lines.push("- Pass only the context slice the sub-agent needs.".into());
    lines.push("- If a sub-agent fails, retry the task yourself without it.".into());
    lines.join("\n")
}

/// Compute the clido config file path (mirrors setup.rs logic).
pub fn global_config_path() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("CLIDO_CONFIG") {
        return Some(std::path::PathBuf::from(p));
    }
    directories::ProjectDirs::from("", "", "clido").map(|d| d.config_dir().join("config.toml"))
}

/// If --mcp-config is provided, spawn MCP servers and register their tools.
/// Errors are printed to stderr but never fatal — the agent runs with whatever
/// tools were successfully registered.
fn load_mcp_tools(cli: &Cli, mut registry: ToolRegistry) -> ToolRegistry {
    let Some(ref mcp_path) = cli.mcp_config else {
        return registry;
    };
    use clido_tools::load_mcp_config;
    use clido_tools::McpClient;
    let mcp_cfg = match load_mcp_config(mcp_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("MCP config load failed: {}", e);
            return registry;
        }
    };
    for server_config in mcp_cfg.servers {
        let server_name = server_config.name.clone();
        match McpClient::spawn(server_config) {
            Err(e) => eprintln!("MCP spawn failed for '{}': {}", server_name, e),
            Ok(client) => {
                if let Err(e) = client.initialize() {
                    eprintln!("MCP initialize failed for '{}': {}", server_name, e);
                    continue;
                }
                match client.list_tools() {
                    Err(e) => eprintln!("MCP list_tools failed for '{}': {}", server_name, e),
                    Ok(tools) => {
                        let client_arc = Arc::new(client);
                        for tool_def in tools {
                            let tool_name = tool_def.name.clone();
                            let mcp_tool = McpTool::new(tool_def, client_arc.clone());
                            registry.register(mcp_tool);
                            if !cli.quiet {
                                eprintln!("MCP tool registered: {}/{}", server_name, tool_name);
                            }
                        }
                    }
                }
            }
        }
    }
    registry
}

fn build_registry(
    cli: &Cli,
    loaded: &clido_core::LoadedConfig,
    workspace_root: &Path,
) -> Result<ToolRegistry, anyhow::Error> {
    let allowed = cli
        .allowed_tools
        .clone()
        .or_else(|| cli.tools.clone())
        .or_else(|| {
            if loaded.tools.allowed.is_empty() {
                None
            } else {
                Some(loaded.tools.allowed.join(","))
            }
        })
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let disallowed = cli
        .disallowed_tools
        .clone()
        .or_else(|| {
            if loaded.tools.disallowed.is_empty() {
                None
            } else {
                Some(loaded.tools.disallowed.join(","))
            }
        })
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    // Block the config file from all tool access so its contents never leave the local system.
    let blocked = global_config_path().into_iter().collect::<Vec<_>>();
    let sandbox = cli.sandbox;
    let registry = default_registry_with_options(workspace_root.to_path_buf(), blocked, sandbox)
        .with_filters(allowed, disallowed);
    if registry.schemas().is_empty() {
        return Err(CliError::Usage(
            "No tools left after --allowed-tools/--disallowed-tools/--tools. Check your filters."
                .into(),
        )
        .into());
    }
    Ok(registry)
}

pub fn parse_permission_mode(s: Option<&str>) -> PermissionMode {
    match s {
        Some("plan") | Some("plan-only") => PermissionMode::PlanOnly,
        Some("accept-all") => PermissionMode::AcceptAll,
        Some("diff-review") => PermissionMode::DiffReview,
        _ => PermissionMode::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_permission_mode ─────────────────────────────────────────────

    #[test]
    fn parse_permission_mode_plan() {
        assert_eq!(
            parse_permission_mode(Some("plan")),
            PermissionMode::PlanOnly
        );
        assert_eq!(
            parse_permission_mode(Some("plan-only")),
            PermissionMode::PlanOnly
        );
    }

    #[test]
    fn parse_permission_mode_accept_all() {
        assert_eq!(
            parse_permission_mode(Some("accept-all")),
            PermissionMode::AcceptAll
        );
    }

    #[test]
    fn parse_permission_mode_diff_review() {
        assert_eq!(
            parse_permission_mode(Some("diff-review")),
            PermissionMode::DiffReview
        );
    }

    #[test]
    fn parse_permission_mode_default_on_none() {
        assert_eq!(parse_permission_mode(None), PermissionMode::Default);
    }

    #[test]
    fn parse_permission_mode_default_on_unknown() {
        assert_eq!(
            parse_permission_mode(Some("garbage")),
            PermissionMode::Default
        );
    }

    // ── build_routing_instructions ────────────────────────────────────────

    #[test]
    fn routing_instructions_worker_only_mentions_spawn_worker() {
        let s = build_routing_instructions(true, false);
        assert!(s.contains("SpawnWorker"), "should mention SpawnWorker");
        assert!(
            !s.contains("SpawnReviewer"),
            "should not mention SpawnReviewer"
        );
        assert!(s.contains("Sub-Agent Routing"));
        assert!(s.contains("Sub-agent rules:"));
    }

    #[test]
    fn routing_instructions_reviewer_only_mentions_spawn_reviewer() {
        let s = build_routing_instructions(false, true);
        assert!(!s.contains("SpawnWorker"), "should not mention SpawnWorker");
        assert!(s.contains("SpawnReviewer"), "should mention SpawnReviewer");
    }

    #[test]
    fn routing_instructions_both_mentions_both() {
        let s = build_routing_instructions(true, true);
        assert!(s.contains("SpawnWorker"));
        assert!(s.contains("SpawnReviewer"));
        assert!(s.contains("Sub-agent rules:"));
    }

    #[test]
    fn routing_instructions_neither_still_has_header_and_rules() {
        let s = build_routing_instructions(false, false);
        assert!(s.contains("Sub-Agent Routing"));
        assert!(s.contains("Sub-agent rules:"));
        assert!(!s.contains("SpawnWorker"));
        assert!(!s.contains("SpawnReviewer"));
    }

    #[test]
    fn routing_instructions_has_retry_guidance() {
        let s = build_routing_instructions(true, true);
        assert!(s.contains("retry"), "should include retry guidance");
    }
}

fn assemble_system_prompt(cli: &Cli) -> Result<String, anyhow::Error> {
    let base = if let Some(ref path) = cli.system_prompt_file {
        std::fs::read_to_string(path)
            .map_err(|e| CliError::Usage(format!("Failed to read system prompt file: {}", e)))?
    } else if let Some(ref s) = cli.system_prompt {
        s.clone()
    } else {
        "You are clido, an AI coding agent. \
         You help with software development tasks: reading, writing, editing, and running code. \
         Always refer to yourself as clido — never as Claude, GPT, Gemini, or any other model name. \
         Be concise and direct."
            .to_string()
    };
    Ok(if let Some(ref append) = cli.append_system_prompt {
        format!("{}\n{}", base, append)
    } else {
        base
    })
}
