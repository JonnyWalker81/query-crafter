use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use color_eyre::eyre::{Result, anyhow};

use crate::lsp_patches::{COMPLETE_JS_PATCH, INITIALIZE_LOGGING_JS_PATCH, SETTING_STORE_JS_PATCH};

/// Find sql-language-server installation path
fn find_sql_lsp_path() -> Option<PathBuf> {
    // Try to find sql-language-server using which/where
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    
    if let Ok(output) = Command::new(which_cmd).arg("sql-language-server").output() {
        if output.status.success() {
            if let Ok(path_str) = String::from_utf8(output.stdout) {
                let sql_lsp_bin = PathBuf::from(path_str.trim());
                
                // Follow symlinks and find the node_modules directory
                if let Ok(real_path) = fs::canonicalize(&sql_lsp_bin) {
                    // Go up directories to find node_modules/sql-language-server
                    let mut current = real_path.parent();
                    while let Some(dir) = current {
                        let sql_lsp_module = dir.join("node_modules").join("sql-language-server");
                        if sql_lsp_module.exists() {
                            return Some(sql_lsp_module);
                        }
                        
                        // Also check if we're already in node_modules
                        if dir.file_name() == Some(std::ffi::OsStr::new("sql-language-server")) {
                            return Some(dir.to_path_buf());
                        }
                        
                        current = dir.parent();
                    }
                }
            }
        }
    }
    
    // Check npm global directory
    if let Ok(output) = Command::new("npm").args(&["config", "get", "prefix"]).output() {
        if output.status.success() {
            if let Ok(prefix) = String::from_utf8(output.stdout) {
                let prefix = prefix.trim();
                let global_path = if cfg!(windows) {
                    PathBuf::from(prefix).join("node_modules").join("sql-language-server")
                } else {
                    PathBuf::from(prefix).join("lib").join("node_modules").join("sql-language-server")
                };
                
                if global_path.exists() {
                    return Some(global_path);
                }
            }
        }
    }
    
    // Check local node_modules
    let local_path = PathBuf::from("node_modules").join("sql-language-server");
    if local_path.exists() {
        return Some(local_path);
    }
    
    None
}

/// Apply a patch to a file
fn apply_patch(file_path: &Path, patch_content: &str) -> Result<()> {
    if !file_path.exists() {
        return Err(anyhow!("File not found: {:?}", file_path));
    }
    
    // Create backup
    let backup_path = file_path.with_extension("js.bak");
    if !backup_path.exists() {
        fs::copy(file_path, &backup_path)?;
    }
    
    // Read the original file
    let original_content = fs::read_to_string(file_path)?;
    
    // Apply patch manually (simple string replacements based on our known patches)
    let patched_content = if file_path.file_name() == Some(std::ffi::OsStr::new("complete.js")) {
        original_content
            .replace("console.time('complete');", "// console.time('complete');  // Disabled to prevent stdout pollution")
            .replace("console.timeEnd('complete');", "// console.timeEnd('complete');  // Disabled to prevent stdout pollution")
    } else if file_path.file_name() == Some(std::ffi::OsStr::new("initializeLogging.js")) {
        original_content
            .replace("level: debug ? 'debug' : 'debug'", "level: debug ? 'debug' : 'error'")
    } else if file_path.file_name() == Some(std::ffi::OsStr::new("SettingStore.js")) {
        // For SettingStore, we need to be more careful
        let mut content = original_content.clone();
        
        // Add safety check to changeConnection
        if !content.contains("PATCH: Add safety check") {
            content = content.replace(
                "async changeConnection(connectionName) {\n        const config = this.personalConfig.connections.find",
                "async changeConnection(connectionName) {\n        // PATCH: Add safety check\n        if (!this.personalConfig || !this.personalConfig.connections || !Array.isArray(this.personalConfig.connections)) {\n            logger.error(`Cannot change connection - invalid personalConfig: ${JSON.stringify(this.personalConfig)}`);\n            throw new Error('Invalid personal config structure');\n        }\n        const config = this.personalConfig.connections.find"
            );
            
            // Add check to extractPersonalConfigMatchedProjectPath
            content = content.replace(
                "extractPersonalConfigMatchedProjectPath(projectPath) {\n        const con = this.personalConfig.connections.find",
                "extractPersonalConfigMatchedProjectPath(projectPath) {\n        // PATCH: Add safety check\n        if (!this.personalConfig || !this.personalConfig.connections || !Array.isArray(this.personalConfig.connections)) {\n            logger.error(`Invalid personalConfig structure: ${JSON.stringify(this.personalConfig)}`);\n            return null;\n        }\n        const con = this.personalConfig.connections.find"
            );
        }
        
        content
    } else {
        original_content
    };
    
    // Write the patched content
    fs::write(file_path, patched_content)?;
    
    Ok(())
}

/// Apply all patches to sql-language-server
pub fn patch_sql_language_server() -> Result<()> {
    println!("Checking for sql-language-server installation...");
    
    let sql_lsp_path = find_sql_lsp_path()
        .ok_or_else(|| anyhow!("sql-language-server not found. Please install it with:\n  npm install -g sql-language-server"))?;
    
    println!("Found sql-language-server at: {:?}", sql_lsp_path);
    
    // Apply patches
    let complete_js = sql_lsp_path.join("dist/src/complete/complete.js");
    if complete_js.exists() {
        println!("Patching complete.js to remove console.time() debug output...");
        apply_patch(&complete_js, COMPLETE_JS_PATCH)?;
    }
    
    let init_logging_js = sql_lsp_path.join("dist/src/initializeLogging.js");
    if init_logging_js.exists() {
        println!("Patching initializeLogging.js to fix log level...");
        apply_patch(&init_logging_js, INITIALIZE_LOGGING_JS_PATCH)?;
    }
    
    let setting_store_js = sql_lsp_path.join("dist/src/SettingStore.js");
    if setting_store_js.exists() {
        println!("Patching SettingStore.js to fix config handling...");
        apply_patch(&setting_store_js, SETTING_STORE_JS_PATCH)?;
    }
    
    println!("Patches applied successfully!");
    println!("\nNext steps:");
    println!("1. Create a .sqllsrc.json file in your project with database configuration");
    println!("2. Enable LSP in ~/.config/query-crafter/config.toml");
    println!("3. Run query-crafter and press Ctrl+Space for autocomplete");
    
    Ok(())
}