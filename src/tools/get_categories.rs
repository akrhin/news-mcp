//! get_categories tool implementation
//!
//! Lists available news categories.

use crate::cache::NewsCache;
use crate::config::FeedSourceConfig;
use crate::tools::Tool;
use async_trait::async_trait;
use rust_mcp_sdk::macros;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Get categories tool parameters
#[macros::mcp_tool(
    name = "get_categories",
    title = "Get Categories",
    description = "Lists available news categories with article counts.",
    destructive_hint = false,
    idempotent_hint = true,
    open_world_hint = false,
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, macros::JsonSchema)]
pub struct GetCategoriesTool {
    // No parameters needed
}

/// Get categories tool implementation
pub struct GetCategoriesToolImpl {
    cache: Arc<NewsCache>,
    feeds: HashMap<String, FeedSourceConfig>,
}

impl GetCategoriesToolImpl {
    /// Create a new get_categories tool
    pub fn new(cache: Arc<NewsCache>, feeds: HashMap<String, FeedSourceConfig>) -> Self {
        Self { cache, feeds }
    }

    /// Look up display info for a Custom category from feeds config
    fn resolve_custom_category(&self, cat: &crate::cache::NewsCategory) -> (String, String) {
        let key = cat.config_key();
        if let Some(feed) = self.feeds.get(key.as_ref()) {
            let name = feed.display_name.clone().unwrap_or_else(|| key.to_string());
            let desc = feed
                .description
                .clone()
                .unwrap_or_else(|| format!("User-defined feed: {key}"));
            (name, desc)
        } else {
            // fallback to the enum's own display_name/description
            (
                cat.display_name().to_string(),
                cat.description().to_string(),
            )
        }
    }
}

#[async_trait]
impl Tool for GetCategoriesToolImpl {
    fn definition(&self) -> rust_mcp_sdk::schema::Tool {
        GetCategoriesTool::tool()
    }

    async fn execute(
        &self,
        _arguments: serde_json::Value,
    ) -> std::result::Result<
        rust_mcp_sdk::schema::CallToolResult,
        rust_mcp_sdk::schema::CallToolError,
    > {
        let categories = self.cache.get_all_categories().map_err(|e| {
            rust_mcp_sdk::schema::CallToolError::from_message(format!("Cache error: {}", e))
        })?;

        let mut output = String::new();
        output.push_str("# Available News Categories\n\n");

        for (category, count) in categories {
            let (name, desc) = if matches!(category, crate::cache::NewsCategory::Custom(_)) {
                self.resolve_custom_category(&category)
            } else {
                (
                    category.display_name().to_string(),
                    category.description().to_string(),
                )
            };
            output.push_str(&format!("- **{name}** ({count} articles)\n"));
            output.push_str(&format!("  {desc}\n\n"));
        }

        Ok(rust_mcp_sdk::schema::CallToolResult::text_content(vec![
            output.into(),
        ]))
    }
}
