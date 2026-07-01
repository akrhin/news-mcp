//! Tools module
//!
//! Provides MCP tools for the news server.

mod get_article_content;
mod get_categories;
mod get_news;

pub use get_article_content::*;
pub use get_categories::*;
pub use get_news::*;

use crate::cache::{ArticleCache, NewsCache};
use crate::config::{ArticleFetchConfig, FeedSourceConfig};
use async_trait::async_trait;
use rust_mcp_sdk::schema::{CallToolError, CallToolResult, Tool as McpTool};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool trait for MCP tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool definition
    fn definition(&self) -> McpTool;

    /// Execute the tool
    async fn execute(
        &self,
        arguments: serde_json::Value,
    ) -> std::result::Result<CallToolResult, CallToolError>;
}

/// Tool registry for managing MCP tools
pub struct ToolRegistry {
    /// Registered tools
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(mut self, tool: Box<dyn Tool>) -> Self {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
        self
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    /// Get all registered tools
    pub fn get_tools(&self) -> Vec<McpTool> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Execute tool by name
    pub async fn execute_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(arguments).await,
            None => Err(CallToolError::unknown_tool(name.to_string())),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create default tool registry with all tools
pub fn create_default_registry(
    news_cache: Arc<NewsCache>,
    article_cache: Arc<ArticleCache>,
    article_fetch_config: ArticleFetchConfig,
    feeds: HashMap<String, FeedSourceConfig>,
) -> ToolRegistry {
    ToolRegistry::new()
        .register(Box::new(GetNewsToolImpl::new(
            news_cache.clone(),
            feeds.clone(),
        )))
        .register(Box::new(GetCategoriesToolImpl::new(
            news_cache.clone(),
            feeds,
        )))
        .register(Box::new(GetArticleContentToolImpl::new(
            news_cache,
            article_cache,
            article_fetch_config,
        )))
}
