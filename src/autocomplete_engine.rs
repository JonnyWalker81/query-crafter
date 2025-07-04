use std::sync::Arc;
use tokio::sync::Mutex;

use crate::autocomplete::{AutocompleteProvider, SuggestionItem, SqlContext};
use crate::components::db::{DbTable, DbColumn};
use crate::lsp::{LspClient, LspCompletionProvider};
use color_eyre::eyre::Result;

/// Enum to support different autocomplete backends
#[derive(Debug)]
pub enum AutocompleteBackend {
    /// Built-in fuzzy matching autocomplete
    Builtin(AutocompleteProvider),
    /// LSP-based autocomplete
    Lsp(LspCompletionProvider),
    /// Both providers running in parallel (LSP preferred when available)
    Hybrid {
        builtin: AutocompleteProvider,
        lsp: LspCompletionProvider,
    },
}

/// Unified autocomplete engine that can use different backends
pub struct AutocompleteEngine {
    backend: AutocompleteBackend,
    /// Prefer LSP results when in hybrid mode
    prefer_lsp: bool,
}

impl AutocompleteEngine {
    /// Create a new engine with builtin provider only
    pub fn new_builtin() -> Self {
        Self {
            backend: AutocompleteBackend::Builtin(AutocompleteProvider::new()),
            prefer_lsp: false,
        }
    }
    
    /// Get a mutable reference to the backend
    pub fn backend_mut(&mut self) -> &mut AutocompleteBackend {
        &mut self.backend
    }
    
    /// Create a new engine with LSP provider only
    pub fn new_lsp(lsp_client: Arc<Mutex<LspClient>>) -> Result<Self> {
        Ok(Self {
            backend: AutocompleteBackend::Lsp(LspCompletionProvider::new(lsp_client)?),
            prefer_lsp: true,
        })
    }
    
    /// Create a hybrid engine with both providers
    pub fn new_hybrid(lsp_client: Arc<Mutex<LspClient>>) -> Result<Self> {
        Ok(Self {
            backend: AutocompleteBackend::Hybrid {
                builtin: AutocompleteProvider::new(),
                lsp: LspCompletionProvider::new(lsp_client)?,
            },
            prefer_lsp: true,
        })
    }
    
    /// Update the SQL content for LSP provider
    pub async fn update_document(&mut self, text: &str) -> Result<()> {
        match &mut self.backend {
            AutocompleteBackend::Builtin(_) => Ok(()),
            AutocompleteBackend::Lsp(provider) => provider.update_document(text).await,
            AutocompleteBackend::Hybrid { lsp, .. } => lsp.update_document(text).await,
        }
    }
    
    /// Update tables for builtin provider
    pub fn update_tables(&mut self, tables: Vec<DbTable>) {
        match &mut self.backend {
            AutocompleteBackend::Builtin(provider) => provider.update_tables(tables),
            AutocompleteBackend::Lsp(_) => {},
            AutocompleteBackend::Hybrid { builtin, .. } => builtin.update_tables(tables),
        }
    }
    
    /// Update table columns for builtin provider
    pub fn update_table_columns(&mut self, table_name: String, columns: Vec<DbColumn>) {
        match &mut self.backend {
            AutocompleteBackend::Builtin(provider) => provider.update_table_columns(table_name, columns),
            AutocompleteBackend::Lsp(_) => {},
            AutocompleteBackend::Hybrid { builtin, .. } => builtin.update_table_columns(table_name, columns),
        }
    }
    
    /// Get suggestions from the appropriate backend
    pub async fn get_suggestions(
        &mut self,
        context: SqlContext,
        input: &str,
        cursor_line: usize,
        cursor_col: usize,
    ) -> Result<Vec<SuggestionItem>> {
        match &mut self.backend {
            AutocompleteBackend::Builtin(provider) => {
                Ok(provider.get_suggestions(context, input))
            },
            AutocompleteBackend::Lsp(provider) => {
                provider.get_completions(cursor_line, cursor_col).await
            },
            AutocompleteBackend::Hybrid { builtin, lsp } => {
                // Try LSP first if preferred and available
                if self.prefer_lsp && lsp.is_available().await {
                    let lsp_results = lsp.get_completions(cursor_line, cursor_col).await?;
                    if !lsp_results.is_empty() {
                        return Ok(lsp_results);
                    }
                }
                
                // Fall back to builtin
                Ok(builtin.get_suggestions(context, input))
            },
        }
    }
    
    /// Check if LSP is available
    pub async fn is_lsp_available(&self) -> bool {
        match &self.backend {
            AutocompleteBackend::Builtin(_) => false,
            AutocompleteBackend::Lsp(provider) => provider.is_available().await,
            AutocompleteBackend::Hybrid { lsp, .. } => lsp.is_available().await,
        }
    }
    
    /// Get the current backend type as a string
    pub fn backend_name(&self) -> &'static str {
        match &self.backend {
            AutocompleteBackend::Builtin(_) => "builtin",
            AutocompleteBackend::Lsp(_) => "lsp",
            AutocompleteBackend::Hybrid { .. } => "hybrid",
        }
    }
}