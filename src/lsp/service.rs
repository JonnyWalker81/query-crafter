use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use color_eyre::eyre::{anyhow, Result};
use serde_json::json;

use crate::{
    action::Action,
    autocomplete::SuggestionKind,
};
use super::{LspClient, LspCompletionProvider, LspConfig};

/// Service that manages LSP client lifecycle and handles autocomplete requests
pub struct LspService {
    lsp_client: Arc<Mutex<LspClient>>,
    lsp_provider: Arc<Mutex<LspCompletionProvider>>,
    action_tx: mpsc::UnboundedSender<Action>,
    current_document: Arc<Mutex<String>>,
}

impl LspService {
    /// Create a new LSP service
    pub async fn new(
        config: LspConfig,
        action_tx: mpsc::UnboundedSender<Action>,
    ) -> Result<Self> {
        // Create and start LSP client
        let lsp_client = Arc::new(Mutex::new(LspClient::new(config)));
        
        // Try to start the LSP server
        {
            let mut client = lsp_client.lock().await;
            client.start().await.map_err(|e| {
                anyhow!("Failed to start LSP server: {}. Make sure sql-language-server is installed.", e)
            })?;
        }
        
        // Create completion provider
        let lsp_provider = Arc::new(Mutex::new(
            LspCompletionProvider::new(lsp_client.clone())?
        ));
        
        Ok(Self {
            lsp_client,
            lsp_provider,
            action_tx,
            current_document: Arc::new(Mutex::new(String::new())),
        })
    }
    
    /// Update the current document content
    pub async fn update_document(&self, text: String) -> Result<()> {
        eprintln!("LSP Service: Updating document, length: {}", text.len());
        *self.current_document.lock().await = text.clone();
        
        let mut provider = self.lsp_provider.lock().await;
        provider.update_document(&text).await?;
        Ok(())
    }
    
    /// Handle an autocomplete request
    pub async fn handle_autocomplete_request(
        &self,
        text: String,
        cursor_line: usize,
        cursor_col: usize,
        _context: String, // Could parse SqlContext if needed
    ) -> Result<()> {
        eprintln!("LSP Service: Handling autocomplete request at {}:{}", cursor_line, cursor_col);
        eprintln!("LSP Service: Text content: '{}'", text);
        eprintln!("LSP Service: Context: '{}'", _context);
        
        // Update document if it changed
        let current_doc = self.current_document.lock().await;
        if *current_doc != text {
            drop(current_doc); // Release lock
            self.update_document(text.clone()).await?;
        }
        
        // Check if LSP is still running
        if !self.is_running().await {
            eprintln!("LSP Service: LSP client is not running!");
            return Err(anyhow!("LSP client is not running"));
        }
        
        // Get completions from LSP
        eprintln!("LSP Service: Requesting completions from LSP provider...");
        let provider = self.lsp_provider.lock().await;
        let suggestions = match provider.get_completions(cursor_line, cursor_col).await {
            Ok(items) => {
                eprintln!("LSP Service: Received {} suggestions from LSP", items.len());
                items
            },
            Err(e) => {
                eprintln!("LSP completion error: {}", e);
                vec![]
            }
        };
        
        // Convert suggestions to simple format for Action
        let results: Vec<(String, String)> = suggestions
            .into_iter()
            .map(|item| {
                let kind = match item.kind {
                    SuggestionKind::Table => "table",
                    SuggestionKind::Column => "column",
                    SuggestionKind::Keyword => "keyword",
                };
                (item.text, kind.to_string())
            })
            .collect();
        
        // Send results back via action
        self.action_tx.send(Action::AutocompleteResults(results))?;
        
        Ok(())
    }
    
    /// Check if the LSP client is running
    pub async fn is_running(&self) -> bool {
        self.lsp_client.lock().await.is_running()
    }
    
    /// Configure the LSP with database connection info
    pub async fn configure_database_connection(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        database: &str,
        adapter: &str,  // "postgres" or "sqlite"
    ) -> Result<()> {
        eprintln!("LSP Service: Configuring database connection for {}:{}/{}", host, port, database);
        
        // Create a temporary .sqllsrc.json configuration
        let config = json!({
            "connections": [{
                "name": "query-crafter-dynamic",
                "adapter": adapter,
                "host": host,
                "port": port,
                "user": username,
                "password": password,
                "database": database,
                "projectPaths": [std::env::current_dir()?.to_string_lossy()]
            }]
        });
        
        // Write to a temporary file
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("query-crafter-sqllsrc.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        eprintln!("LSP Service: Wrote temporary config to {:?}", config_path);
        
        // TODO: Send a notification to the LSP server to reload configuration
        // For now, this would require restarting the LSP server with the new config
        
        Ok(())
    }
}