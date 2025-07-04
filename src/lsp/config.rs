use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    /// Whether to enable LSP support
    #[serde(default)]
    pub enabled: bool,
    
    /// Which SQL language server to use
    #[serde(default = "default_server_name")]
    pub server_name: String,
    
    /// Command to launch the LSP server
    #[serde(default = "default_server_command")]
    pub server_command: String,
    
    /// Arguments for the LSP server
    #[serde(default)]
    pub server_args: Vec<String>,
    
    /// Root URI for the LSP workspace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,
    
    /// Additional initialization options for the LSP server
    #[serde(default)]
    pub init_options: HashMap<String, serde_json::Value>,
    
    /// Completion trigger characters (in addition to Ctrl+Space)
    #[serde(default = "default_trigger_characters")]
    pub trigger_characters: Vec<String>,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_name: default_server_name(),
            server_command: default_server_command(),
            server_args: vec![],
            root_uri: None,
            init_options: HashMap::new(),
            trigger_characters: default_trigger_characters(),
        }
    }
}

fn default_server_name() -> String {
    "sql-language-server".to_string()
}

fn default_server_command() -> String {
    // First check if sql-lsp-wrapper is in the same directory as query-crafter
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let wrapper_path = parent.join("sql-lsp-wrapper");
            if wrapper_path.exists() {
                return wrapper_path.to_string_lossy().to_string();
            }
        }
    }
    
    // Fall back to PATH
    "sql-lsp-wrapper".to_string()
}

fn default_trigger_characters() -> Vec<String> {
    vec![".".to_string(), " ".to_string()]
}

impl LspConfig {
    /// Get the full command to launch the LSP server
    pub fn get_server_command(&self) -> Vec<String> {
        let mut cmd = vec![self.server_command.clone()];
        cmd.extend(self.server_args.clone());
        
        // Don't add args if using the wrapper - it handles them internally
        if !self.server_command.contains("wrapper") {
            // Add specific args for known servers
            match self.server_name.as_str() {
                "sql-language-server" => {
                    cmd.push("up".to_string());
                    cmd.push("--method".to_string());
                    cmd.push("stdio".to_string());
                    // Explicitly disable debug mode
                    cmd.push("--debug".to_string());
                    cmd.push("false".to_string());
                }
                _ => {}
            }
        }
        
        cmd
    }
    
    /// Check if a character should trigger completion
    pub fn should_trigger_completion(&self, ch: char) -> bool {
        self.trigger_characters.iter().any(|tc| tc.chars().next() == Some(ch))
    }
}