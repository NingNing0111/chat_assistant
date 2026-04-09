//! MCP (Model Context Protocol) client implementation
//!
//! Supports streamable HTTP MCP servers with manual JSON-RPC protocol handling.

use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolDyn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("MCP connection error: {0}")]
    Connection(String),
    #[error("MCP tool error: {0}")]
    Tool(String),
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
}

/// MCP server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub description: Option<String>,
    pub base_url: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
}

/// Root MCP configuration
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// MCP JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

/// MCP JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// MCP tool loaded from server
#[derive(Debug, Clone)]
pub struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
    base_url: String,
    client: reqwest::Client,
}

impl McpTool {
    pub fn new(name: String, description: String, input_schema: Value, base_url: String) -> Self {
        Self {
            name,
            description,
            input_schema,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

impl Tool for McpTool {
    const NAME: &'static str = "mcp_tool";
    type Error = McpError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.input_schema.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "tools/call".to_string(),
            params: json!({
                "name": self.name,
                "arguments": args,
            }),
        };

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| McpError::Connection(e.to_string()))?;

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| McpError::Connection(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            return Err(McpError::JsonRpc(error.message));
        }

        rpc_response
            .result
            .ok_or_else(|| McpError::JsonRpc("No result".to_string()))
    }
}

/// MCP client for connecting to servers and loading tools
pub struct McpClient {
    config: McpConfig,
    client: reqwest::Client,
}

impl McpClient {
    pub fn new(config: McpConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Connect to a single MCP server and list its tools
    pub async fn list_server_tools(&self, server: &McpServerConfig) -> anyhow::Result<Vec<McpTool>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let response = self
            .client
            .post(&server.base_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Invalid response: {}", e))?;

        if let Some(error) = rpc_response.error {
            anyhow::bail!("Server error: {}", error.message);
        }

        let result = rpc_response.result.ok_or_else(|| anyhow::anyhow!("No result"))?;

        let tools_array = result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        let tools: Vec<McpTool> = tools_array
            .iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?.to_string();
                let description = t
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let input_schema = t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or(json!({}));

                Some(McpTool::new(
                    name,
                    description,
                    input_schema,
                    server.base_url.clone(),
                ))
            })
            .collect();

        Ok(tools)
    }

    /// Connect to all active servers and load their tools
    pub async fn load_all_tools(&self) -> anyhow::Result<Vec<Box<dyn ToolDyn>>> {
        let mut all_tools: Vec<Box<dyn ToolDyn>> = Vec::new();

        for (id, server) in &self.config.mcp_servers {
            if !server.is_active {
                println!("Skipping inactive MCP server: {} ({})", server.name, id);
                continue;
            }

            println!("Connecting to MCP server: {} at {}", server.name, server.base_url);

            match self.list_server_tools(server).await {
                Ok(tools) => {
                    println!("  Loaded {} tools from {}", tools.len(), server.name);
                    for tool in tools {
                        all_tools.push(Box::new(tool));
                    }
                }
                Err(e) => {
                    println!("  Warning: Failed to load tools from {}: {}", server.name, e);
                }
            }
        }

        Ok(all_tools)
    }
}

/// MCP tool loader from config file
pub struct McpToolLoader {
    config_path: std::path::PathBuf,
}

impl McpToolLoader {
    pub fn new(config_path: &std::path::Path) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    /// Load MCP configuration from file
    pub fn load_config(&self) -> anyhow::Result<McpConfig> {
        let content = std::fs::read_to_string(&self.config_path)?;
        Self::parse_config(&content)
    }

    /// Parse MCP configuration from JSON string
    pub fn parse_config(json: &str) -> anyhow::Result<McpConfig> {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Failed to parse MCP config: {}", e))
    }

    /// Get active server configs
    pub fn get_active_servers(&self) -> anyhow::Result<Vec<(String, McpServerConfig)>> {
        let config = self.load_config()?;
        Ok(config
            .mcp_servers
            .into_iter()
            .filter(|(_, server)| server.is_active)
            .collect())
    }

    /// Load tools from all active MCP servers
    pub async fn load_tools(&self) -> anyhow::Result<Vec<Box<dyn ToolDyn>>> {
        let config = self.load_config()?;
        let client = McpClient::new(config);
        client.load_all_tools().await
    }
}

/// Parse MCP JSON config from string
pub fn parse_mcp_config(json_str: &str) -> anyhow::Result<McpConfig> {
    serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse MCP config: {}", e))
}

impl McpConfig {
    /// Get all active server configurations
    pub fn active_servers(&self) -> Vec<(&String, &McpServerConfig)> {
        self.mcp_servers
            .iter()
            .filter(|(_, config)| config.is_active)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{
            "mcpServers": {
                "test-server": {
                    "name": "Test Server",
                    "base_url": "http://localhost:8080/mcp",
                    "isActive": true
                }
            }
        }"#;

        let config = parse_mcp_config(json).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert!(config.mcp_servers.contains_key("test-server"));
    }
}
