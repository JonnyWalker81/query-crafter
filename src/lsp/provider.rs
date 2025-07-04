use async_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position, Url};
use color_eyre::eyre::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::autocomplete::{SuggestionItem, SuggestionKind};
use super::client::LspClient;

/// Provides LSP-based completions that can be used alongside or instead of
/// the existing autocomplete system
#[derive(Debug)]
pub struct LspCompletionProvider {
    client: Arc<Mutex<LspClient>>,
    document_uri: Url,
    document_version: i32,
}

impl LspCompletionProvider {
    pub fn new(client: Arc<Mutex<LspClient>>) -> Result<Self> {
        // Create a virtual document URI for the SQL editor
        let document_uri = Url::parse("file:///tmp/query-crafter-editor.sql")?;
        
        Ok(Self {
            client,
            document_uri,
            document_version: 0,
        })
    }
    
    /// Update the document content in the LSP server
    pub async fn update_document(&mut self, text: &str) -> Result<()> {
        self.document_version += 1;
        
        let client = self.client.lock().await;
        if client.is_running() {
            // For now, we'll use open_document for each update
            // In a real implementation, we'd use didChange notifications
            client.open_document(self.document_uri.clone(), text.to_string()).await?;
        }
        
        Ok(())
    }
    
    /// Get completions at the given cursor position
    pub async fn get_completions(
        &self,
        cursor_line: usize,
        cursor_col: usize,
    ) -> Result<Vec<SuggestionItem>> {
        let client = self.client.lock().await;
        
        if !client.is_running() {
            return Ok(vec![]);
        }
        
        // Convert to LSP position (0-based)
        let position = Position {
            line: cursor_line as u32,
            character: cursor_col as u32,
        };
        
        // Get completions from LSP
        let lsp_items = client.get_completions(self.document_uri.clone(), position).await?;
        
        // Convert LSP completion items to our internal format
        let suggestions: Vec<SuggestionItem> = lsp_items
            .into_iter()
            .filter_map(|item| self.convert_completion_item(item))
            .collect();
        
        Ok(suggestions)
    }
    
    /// Convert LSP CompletionItem to internal SuggestionItem
    fn convert_completion_item(&self, item: CompletionItem) -> Option<SuggestionItem> {
        let text = item.label.clone();
        
        // Determine the kind based on LSP completion item kind
        let kind = match item.kind {
            Some(CompletionItemKind::KEYWORD) => SuggestionKind::Keyword,
            Some(CompletionItemKind::FIELD) => SuggestionKind::Column,
            Some(CompletionItemKind::CLASS | CompletionItemKind::INTERFACE) => SuggestionKind::Table,
            _ => {
                // Try to infer from the label or detail
                if let Some(detail) = &item.detail {
                    if detail.to_lowercase().contains("table") {
                        SuggestionKind::Table
                    } else if detail.to_lowercase().contains("column") {
                        SuggestionKind::Column
                    } else {
                        SuggestionKind::Keyword
                    }
                } else {
                    SuggestionKind::Keyword
                }
            }
        };
        
        // Extract table context from detail if it's a column
        let table_context = if kind == SuggestionKind::Column {
            item.detail.and_then(|detail| {
                // Look for patterns like "table.column" or "from table"
                let parts: Vec<&str> = detail.split('.').collect();
                if parts.len() > 1 {
                    Some(parts[0].to_string())
                } else {
                    None
                }
            })
        } else {
            None
        };
        
        // Use sort_text or a default score
        let score = item.sort_text
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(100);
        
        Some(SuggestionItem {
            text,
            kind,
            score,
            table_context,
        })
    }
    
    /// Check if LSP is available and running
    pub async fn is_available(&self) -> bool {
        self.client.lock().await.is_running()
    }
}