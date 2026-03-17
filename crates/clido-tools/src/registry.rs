//! Tool registry: register and lookup tools by name.

use clido_core::ToolSchema;
use std::collections::{HashMap, HashSet};

use crate::Tool;

/// Registry of named tools.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        let name = tool.name().to_string();
        self.tools.insert(name, Box::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .values()
            .map(|t| ToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.schema(),
            })
            .collect()
    }

    /// Apply allow/disallow lists. Disallowed takes precedence. Returns a new registry
    /// with only the allowed tools (or all if allowed is None, minus disallowed).
    pub fn with_filters(
        self,
        allowed: Option<Vec<String>>,
        disallowed: Option<Vec<String>>,
    ) -> Self {
        let disallowed_set: HashSet<String> = disallowed
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let allowed_set: Option<HashSet<String>> = allowed.map(|v| {
            v.into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });
        let tools = self
            .tools
            .into_iter()
            .filter(|(name, _)| {
                if disallowed_set.contains(name) {
                    return false;
                }
                if let Some(ref a) = allowed_set {
                    if !a.contains(name) {
                        return false;
                    }
                }
                true
            })
            .collect();
        ToolRegistry { tools }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
