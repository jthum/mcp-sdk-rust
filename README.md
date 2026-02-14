# MCP SDK (Rust)

A lightweight, high-performance Rust implementation of the **Model Context Protocol (MCP)**.

This SDK provides the primitives necessary to build MCP clients that communicate with MCP servers over standard I/O (stdio) using JSON-RPC.

## 🚀 Features

- **Standard I/O Transport**: Built-in support for spawning and communicating with MCP servers via stdin/stdout.
- **Async/Await**: Native `tokio` support for concurrent request handling.
- **Strongly Typed**: Clean separation of JSON-RPC types and MCP-specific primitives (Tools, Resources, Prompts).
- **Content Normalization**: Built-in helpers to flatten complex multi-modal content into text strings.
- **Lightweight**: Zero-bloat design, optimized for being embedded in larger runtimes like [Bedrock](https://github.com/bedrock-ai/bedrock).

## 🛠 Usage

### Basic Client Initialization

```rust
use mcp_sdk::{StdioTransport, McpClient};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define the server command
    let transport = StdioTransport::new("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/path/to/allow"])?;

    // 2. Start Client
    let client = McpClient::new(transport);
    
    // 3. Perform Handshake
    client.initialize().await?;

    // 4. List Tools
    let list = client.list_tools().await?;
    for tool in list.tools {
        println!("Available tool: {}", tool.name);
    }

    // 5. Call a Tool and normalize output
    let result = client.call_tool("read_file", serde_json::json!({"path": "file.txt"})).await?;
    println!("Content: {}", result.as_text());

    Ok(())
}
```

## 🏗 Project Structure

- **`src/types.rs`**: JSON-RPC models and MCP message schemas.
- **`src/transport.rs`**: Implementation of `StdioTransport` using `tokio` pipes.
- **`src/client.rs`**: The high-level `McpClient` managing request/response mapping and server lifecycle.

## 📄 License

MIT
