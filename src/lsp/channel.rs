use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot};
use color_eyre::eyre::Result;

use crate::autocomplete::{SuggestionItem, SqlContext};
use super::{LspClient, LspCompletionProvider};

/// Request to get completions from LSP
#[derive(Debug)]
pub struct CompletionRequest {
    pub context: SqlContext,
    pub input: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub response_tx: oneshot::Sender<Vec<SuggestionItem>>,
}

/// LSP handler that runs in a background task
pub struct LspHandler {
    request_rx: mpsc::UnboundedReceiver<CompletionRequest>,
    lsp_provider: LspCompletionProvider,
}

impl LspHandler {
    pub fn new(
        request_rx: mpsc::UnboundedReceiver<CompletionRequest>,
        lsp_client: Arc<Mutex<LspClient>>,
    ) -> Result<Self> {
        Ok(Self {
            request_rx,
            lsp_provider: LspCompletionProvider::new(lsp_client)?,
        })
    }
    
    /// Run the handler, processing completion requests
    pub async fn run(mut self) {
        while let Some(request) = self.request_rx.recv().await {
            // Update document with current text
            if let Err(e) = self.lsp_provider.update_document(&request.input).await {
                eprintln!("Failed to update LSP document: {}", e);
            }
            
            // Get completions
            let completions = match self.lsp_provider.get_completions(
                request.cursor_line,
                request.cursor_col,
            ).await {
                Ok(items) => items,
                Err(e) => {
                    eprintln!("Failed to get LSP completions: {}", e);
                    vec![]
                }
            };
            
            // Send response back
            let _ = request.response_tx.send(completions);
        }
    }
}

/// Channel-based LSP client for synchronous contexts
#[derive(Clone)]
pub struct LspChannelClient {
    request_tx: mpsc::UnboundedSender<CompletionRequest>,
}

impl LspChannelClient {
    /// Create a new channel client and spawn the handler
    pub fn new(lsp_client: Arc<Mutex<LspClient>>) -> Result<Self> {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        
        // Create and spawn handler
        let handler = LspHandler::new(request_rx, lsp_client)?;
        tokio::spawn(async move {
            handler.run().await;
        });
        
        Ok(Self { request_tx })
    }
    
    /// Request completions synchronously (with timeout)
    pub fn get_completions_sync(
        &self,
        context: SqlContext,
        input: String,
        cursor_line: usize,
        cursor_col: usize,
    ) -> Vec<SuggestionItem> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let request = CompletionRequest {
            context,
            input,
            cursor_line,
            cursor_col,
            response_tx,
        };
        
        // Send request
        if self.request_tx.send(request).is_err() {
            eprintln!("LSP handler channel closed");
            return vec![];
        }
        
        // Wait for response with timeout
        match response_rx.blocking_recv() {
            Ok(completions) => completions,
            Err(_) => {
                eprintln!("LSP completion timeout or handler error");
                vec![]
            }
        }
    }
}