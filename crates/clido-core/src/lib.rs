//! Shared types, errors, and config for Clido.

pub mod config;
pub mod config_loader;
pub mod error;
pub mod model_prefs;
pub mod pricing;
pub mod types;

pub use config::{
    AgentConfig, AgentSlotConfig, AgentsConfig, HooksConfig, PermissionMode, ProviderConfig,
    ProviderType,
};
pub use config_loader::{
    agent_config_from_loaded, config_file_exists, delete_profile_from_config, global_config_path,
    load_config, switch_active_profile, upsert_profile_in_config, LoadedConfig, ProfileEntry,
    RolesSection,
};
pub use error::{ClidoError, Result};
pub use model_prefs::ModelPrefs;
pub use pricing::{compute_cost_usd, load_pricing, ModelPricingEntry, PricingTable};
pub use types::{ContentBlock, Message, ModelResponse, Role, StopReason, ToolSchema, Usage};
