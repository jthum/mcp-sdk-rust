use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[async_trait]
pub trait Transport {
    async fn send<T: Serialize + Send + Sync>(&self, message: T) -> Result<()>;
    async fn receive<T: DeserializeOwned + Send + Sync>(&self) -> Result<T>;
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Transport over Stdio of a subprocess
pub struct StdioTransport {
    #[allow(dead_code)]
    child: Mutex<Child>,
    reader: Mutex<BufReader<tokio::process::ChildStdout>>,
    writer: Mutex<tokio::process::ChildStdin>,
}

impl StdioTransport {
    pub fn new(command: &str, args: &[&str]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let mut child = cmd.spawn().context("Failed to spawn MCP server process")?;

        let stdin = child.stdin.take().context("Failed to open stdin")?;
        let stdout = child.stdout.take().context("Failed to open stdout")?;

        Ok(Self {
            child: Mutex::new(child),
            reader: Mutex::new(BufReader::new(stdout)),
            writer: Mutex::new(stdin),
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send<T: Serialize + Send + Sync>(&self, message: T) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        let mut writer = self.writer.lock().await;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    async fn receive<T: DeserializeOwned + Send + Sync>(&self) -> Result<T> {
        let mut reader = self.reader.lock().await;
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(anyhow::anyhow!("MCP Server closed connection (EOF)"));
        }
        let message: T = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse MCP message: {}", line))?;
        Ok(message)
    }

    async fn close(&self) -> Result<()> {
        {
            let mut writer = self.writer.lock().await;
            let _ = writer.shutdown().await;
        }

        let mut child = self.child.lock().await;
        if child.try_wait()?.is_some() {
            return Ok(());
        }

        let _ = child.kill().await;
        let _ = child.wait().await;
        Ok(())
    }
}
