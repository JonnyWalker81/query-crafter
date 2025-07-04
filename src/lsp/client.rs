use std::sync::Arc;
use std::time::Duration;

use async_lsp::{
    MainLoop, ServerSocket,
};
use lsp_types::{
    CompletionItem, CompletionParams, CompletionResponse, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, Position, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, Url, WorkDoneProgressParams,
    notification::{DidOpenTextDocument, Initialized},
    request::{Completion, Initialize},
};
use color_eyre::eyre::{anyhow, Result};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::config::LspConfig;

#[derive(Debug)]
pub struct LspClient {
    config: LspConfig,
    client: Option<Arc<ServerSocket>>,
    server_process: Option<Child>,
    initialized: bool,
    mainloop_handle: Option<tokio::task::JoinHandle<()>>,
}

impl LspClient {
    pub fn new(config: LspConfig) -> Self {
        Self {
            config,
            client: None,
            server_process: None,
            initialized: false,
            mainloop_handle: None,
        }
    }
    
    /// Start the LSP server and initialize the client
    pub async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Get the command to launch the server
        let cmd_parts = self.config.get_server_command();
        if cmd_parts.is_empty() {
            return Err(anyhow!("No LSP server command specified"));
        }
        
        // Start the server process
        let mut cmd = Command::new(&cmd_parts[0]);
        if cmd_parts.len() > 1 {
            cmd.args(&cmd_parts[1..]);
        }
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
            
        let mut server_process = cmd.spawn()
            .map_err(|e| anyhow!("Failed to start LSP server '{}': {}", cmd_parts[0], e))?;
        
        // Get stdin and stdout from the process
        let stdin = server_process.stdin.take()
            .ok_or_else(|| anyhow!("Failed to get stdin from LSP server"))?;
        let stdout = server_process.stdout.take()
            .ok_or_else(|| anyhow!("Failed to get stdout from LSP server"))?;
        
        // Create the LSP client mainloop and server socket
        let (mainloop, server_socket) = MainLoop::new_client(|client| {
            // Return the client directly - it's already a proper service
            client
        });
        
        // Store the server socket for communication
        self.client = Some(Arc::new(server_socket));
        self.server_process = Some(server_process);
        
        // Spawn the mainloop task
        let handle = tokio::spawn(async move {
            // Convert tokio streams to futures streams
            let stdout_compat = stdout.compat();
            let stdin_compat = stdin.compat_write();
            
            if let Err(e) = mainloop.run_buffered(stdout_compat, stdin_compat).await {
                eprintln!("LSP mainloop error: {}", e);
            }
        });
        
        self.mainloop_handle = Some(handle);
        
        // Initialize the LSP connection
        self.initialize().await?;
        
        Ok(())
    }
    
    /// Initialize the LSP connection
    async fn initialize(&mut self) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("LSP client not started"))?;
        
        // Create root URI
        let root_uri = if let Some(uri) = &self.config.root_uri {
            Url::parse(uri)?
        } else {
            // Use current directory as default
            let cwd = std::env::current_dir()?;
            Url::from_file_path(cwd)
                .map_err(|_| anyhow!("Failed to create file URL from current directory"))?
        };
        
        // Send initialize request
        let init_params = InitializeParams {
            initialization_options: Some(serde_json::json!(self.config.init_options)),
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: root_uri,
                name: "query-crafter".to_string(),
            }]),
            ..Default::default()
        };
        
        let _init_result: InitializeResult = timeout(
            Duration::from_secs(10),
            client.request::<Initialize>(init_params)
        ).await
            .map_err(|_| anyhow!("LSP initialization timed out"))??;
        
        // Send initialized notification
        client.notify::<Initialized>(InitializedParams {})?;
        
        self.initialized = true;
        Ok(())
    }
    
    /// Stop the LSP server
    pub async fn stop(&mut self) -> Result<()> {
        // TODO: Send shutdown request to server
        
        // Cancel the mainloop task
        if let Some(handle) = self.mainloop_handle.take() {
            handle.abort();
        }
        
        if let Some(mut process) = self.server_process.take() {
            process.kill().await?;
        }
        
        self.client = None;
        self.initialized = false;
        Ok(())
    }
    
    /// Open a document in the LSP server (for better context)
    pub async fn open_document(&self, uri: Url, text: String) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("LSP client not started"))?;
        
        if !self.initialized {
            return Err(anyhow!("LSP not initialized"));
        }
        
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "sql".to_string(),
                version: 1,
                text,
            },
        };
        
        client.notify::<DidOpenTextDocument>(params)?;
        Ok(())
    }
    
    /// Request completions at the given position
    pub async fn get_completions(
        &self,
        uri: Url,
        position: Position,
    ) -> Result<Vec<CompletionItem>> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("LSP client not started"))?;
        
        if !self.initialized {
            return Err(anyhow!("LSP not initialized"));
        }
        
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        
        let response: Option<CompletionResponse> = timeout(
            Duration::from_secs(5),
            client.request::<Completion>(params)
        ).await
            .map_err(|_| anyhow!("Completion request timed out"))??;
        
        let items = match response {
            Some(CompletionResponse::Array(items)) => items,
            Some(CompletionResponse::List(list)) => list.items,
            None => vec![],
        };
        
        Ok(items)
    }
    
    /// Check if the LSP client is running
    pub fn is_running(&self) -> bool {
        self.client.is_some() && self.initialized
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Try to kill the server process on drop
        if let Some(mut process) = self.server_process.take() {
            let _ = process.start_kill();
        }
    }
}