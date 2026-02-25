use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use crate::transport::Transport;
use crate::types::*;

pub struct McpClient<T: Transport> {
    transport: Arc<T>,
    next_id: AtomicI64,
    pending_requests: Arc<Mutex<HashMap<i64, oneshot::Sender<JsonRpcResponse>>>>,
}

impl<T: Transport + Send + Sync + 'static> McpClient<T> {
    pub fn new(transport: T) -> Self {
        let client = Self {
            transport: Arc::new(transport),
            next_id: AtomicI64::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        // Spawn a background task to read responses
        let transport_clone = client.transport.clone();
        let pending_clone = client.pending_requests.clone();

        tokio::spawn(async move {
            loop {
                match transport_clone.receive::<JsonRpcResponse>().await {
                    Ok(response) => {
                        if let Some(id_val) = response.id.clone() {
                            if let Some(id) = id_val.as_i64() {
                                let mut pending = pending_clone.lock().await;
                                if let Some(sender) = pending.remove(&id) {
                                    let _ = sender.send(response);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("MCP Client Transport Error: {:?}", e);
                        break;
                    }
                }
            }
        });

        client
    }

    async fn request<P: Serialize + Send + Sync>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let params_value = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(id.into())),
            method: method.to_string(),
            params: params_value,
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        self.transport.send(request).await?;

        let response = rx
            .await
            .context("Failed to receive response from MCP server")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "MCP Error {}: {}",
                error.code,
                error.message
            ));
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    pub async fn initialize(&self) -> Result<()> {
        // Minimal init for now
        let _ = self
            .request::<Value>(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2024-11-05", // Example version
                    "capabilities": {},
                    "clientInfo": { "name": "bedrock-mcp", "version": "0.1.0" }
                })),
            )
            .await?;

        // Send initialized notification
        self.transport
            .send(JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: None,
                method: "notifications/initialized".to_string(),
                params: None,
            })
            .await?;

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<ListToolsResult> {
        let result = self.request::<()>("tools/list", None).await?;
        serde_json::from_value(result).context("Failed to parse list_tools result")
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        let params = CallToolRequest {
            name: name.to_string(),
            arguments,
        };
        let result = self.request("tools/call", Some(params)).await?;
        serde_json::from_value(result).context("Failed to parse call_tool result")
    }

    pub async fn shutdown(&self) -> Result<()> {
        let shutdown_result = self.request::<()>("shutdown", None).await;
        let _ = self
            .transport
            .send(JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: None,
                method: "exit".to_string(),
                params: None,
            })
            .await;
        let close_result = self.transport.close().await;

        if let Err(err) = shutdown_result {
            if let Err(close_err) = close_result {
                return Err(anyhow::anyhow!(
                    "MCP shutdown request failed: {}; transport close failed: {}",
                    err,
                    close_err
                ));
            }
            return Err(err);
        }

        close_result
    }
}
