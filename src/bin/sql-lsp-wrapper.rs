use std::process::{Command, Stdio};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::thread;
use std::sync::mpsc;

fn main() {
    // Log startup
    eprintln!("sql-lsp-wrapper: Starting...");
    eprintln!("sql-lsp-wrapper: Current directory: {:?}", std::env::current_dir());
    
    // Check if we should use a custom config
    let _use_custom_config = if let Ok(config_path) = std::env::var("SQL_LSP_CONFIG_PATH") {
        eprintln!("sql-lsp-wrapper: Using custom config from: {}", config_path);
        true
    } else {
        false
    };
    
    // Get sql-language-server from PATH or use npx
    let sql_lsp = std::env::var("SQL_LANGUAGE_SERVER_PATH")
        .unwrap_or_else(|_| "sql-language-server".to_string());
    
    // Try different ways to start sql-language-server
    let mut child = None;
    let mut last_error = None;
    
    // Build command arguments
    let mut args = vec!["up", "--method", "stdio", "--debug", "false"];
    
    // Check if .sqllsrc.json exists in current directory
    let sqllsrc_path = std::path::Path::new(".sqllsrc.json");
    if sqllsrc_path.exists() {
        eprintln!("sql-lsp-wrapper: Found .sqllsrc.json in current directory, using it for LSP configuration");
        // Don't add --no-personal-config, let it use the config
    } else {
        eprintln!("sql-lsp-wrapper: No .sqllsrc.json found, running without database configuration");
        args.push("--no-personal-config");
    }
    
    // Try 1: Direct command
    eprintln!("sql-lsp-wrapper: Trying direct command: {}", sql_lsp);
    match Command::new(&sql_lsp)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("NODE_ENV", "production")
        .env("DEBUG", "")
        .spawn() {
        Ok(c) => {
            eprintln!("sql-lsp-wrapper: Started via direct command");
            child = Some(c)
        },
        Err(e) => {
            eprintln!("sql-lsp-wrapper: Direct command failed: {}", e);
            last_error = Some(e)
        },
    }
    
    // Try 2: Local node_modules in current directory
    if child.is_none() {
        if let Ok(cwd) = std::env::current_dir() {
            let local_path = cwd.join("node_modules").join(".bin").join("sql-language-server");
            eprintln!("sql-lsp-wrapper: Checking local path: {:?}", local_path);
            if local_path.exists() {
                eprintln!("sql-lsp-wrapper: Found at local path, starting...");
                match Command::new(&local_path)
                    .args(&args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .env("NODE_ENV", "production")
                    .env("DEBUG", "")
                    .spawn() {
                    Ok(c) => {
                        eprintln!("sql-lsp-wrapper: Started via local node_modules");
                        child = Some(c)
                    },
                    Err(e) => {
                        eprintln!("sql-lsp-wrapper: Local node_modules failed: {}", e);
                        last_error = Some(e)
                    },
                }
            } else {
                eprintln!("sql-lsp-wrapper: Local path does not exist");
            }
        }
    }
    
    // Try 2b: Check in parent directories for node_modules
    if child.is_none() {
        eprintln!("sql-lsp-wrapper: Checking parent directories for node_modules...");
        let mut current = std::env::current_dir().unwrap_or_default();
        for _ in 0..5 {  // Check up to 5 parent directories
            let npm_bin = current.join("node_modules").join(".bin").join("sql-language-server");
            eprintln!("sql-lsp-wrapper: Checking parent path: {:?}", npm_bin);
            if npm_bin.exists() {
                eprintln!("sql-lsp-wrapper: Found in parent directory, starting...");
                match Command::new(&npm_bin)
                    .args(&args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .env("NODE_ENV", "production")
                    .env("DEBUG", "")
                    .spawn() {
                    Ok(c) => {
                        eprintln!("sql-lsp-wrapper: Started via parent node_modules");
                        child = Some(c);
                        break;
                    },
                    Err(e) => {
                        eprintln!("sql-lsp-wrapper: Parent node_modules failed: {}", e);
                        last_error = Some(e)
                    },
                }
            }
            if !current.pop() {
                break;
            }
        }
    }
    
    // Try 2c: Check specifically in komodo directory
    if child.is_none() {
        let komodo_path = std::path::Path::new("/home/cipher/Repositories/komodo/node_modules/.bin/sql-language-server");
        eprintln!("sql-lsp-wrapper: Checking komodo path: {:?}", komodo_path);
        if komodo_path.exists() {
            eprintln!("sql-lsp-wrapper: Found in komodo directory, starting...");
            match Command::new(&komodo_path)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("NODE_ENV", "production")
                .env("DEBUG", "")
                .spawn() {
                Ok(c) => {
                    eprintln!("sql-lsp-wrapper: Started via komodo node_modules");
                    child = Some(c)
                },
                Err(e) => {
                    eprintln!("sql-lsp-wrapper: Komodo node_modules failed: {}", e);
                    last_error = Some(e)
                },
            }
        }
    }
    
    // Try 3: npx
    if child.is_none() {
        eprintln!("sql-lsp-wrapper: Trying npx...");
        let mut npx_args = vec!["sql-language-server"];
        npx_args.extend(args.iter().map(|s| *s));
        match Command::new("npx")
            .args(&npx_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("NODE_ENV", "production")
            .env("DEBUG", "")
            .spawn() {
            Ok(c) => {
                eprintln!("sql-lsp-wrapper: Started via npx");
                child = Some(c)
            },
            Err(e) => {
                eprintln!("sql-lsp-wrapper: npx failed: {}", e);
                last_error = Some(e)
            },
        }
    }
    
    let mut child = child.unwrap_or_else(|| {
        eprintln!("Failed to start sql-language-server: {:?}", last_error);
        eprintln!("Tried:");
        eprintln!("  1. Direct command: {}", sql_lsp);
        eprintln!("  2. Local node_modules/.bin/sql-language-server");
        eprintln!("  3. Parent directories node_modules");
        eprintln!("  4. /home/cipher/Repositories/komodo/node_modules");
        eprintln!("  5. npx sql-language-server");
        eprintln!("\nPlease install it with: npm install -g sql-language-server");
        eprintln!("Or install locally: npm install sql-language-server");
        std::process::exit(1);
    });
    
    // Give the process a moment to start and check if it's still running
    thread::sleep(std::time::Duration::from_millis(100));
    match child.try_wait() {
        Ok(Some(status)) => {
            eprintln!("sql-lsp-wrapper: ERROR - sql-language-server exited immediately with status: {:?}", status);
            std::process::exit(1);
        }
        Ok(None) => {
            eprintln!("sql-lsp-wrapper: sql-language-server process is running");
        }
        Err(e) => {
            eprintln!("sql-lsp-wrapper: WARNING - Could not check process status: {}", e);
        }
    }
    
    let mut child_stdin = child.stdin.take().expect("Failed to get stdin");
    let child_stdout = child.stdout.take().expect("Failed to get stdout");
    let child_stderr = child.stderr.take();
    
    // Spawn stderr logger if available
    if let Some(stderr) = child_stderr {
        thread::spawn(move || {
            let mut stderr_reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                match stderr_reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        // Log all stderr output from sql-language-server
                        eprint!("sql-lsp-wrapper: [SQL-LSP stderr] {}", line);
                        line.clear();
                    }
                    Err(_) => break,
                }
            }
        });
    }
    
    // Channel for filtered output
    let (tx, rx) = mpsc::channel();
    
    // Thread to filter stdout
    let filter_thread = thread::spawn(move || {
        let mut reader = BufReader::new(child_stdout);
        let mut buffer = Vec::new();
        let mut header_buffer = Vec::new();
        let mut in_header = true;
        let mut content_length = 0usize;
        
        let mut byte_buffer = [0u8; 1];
        loop {
            match reader.read_exact(&mut byte_buffer) {
                Ok(()) => {
                    let byte = byte_buffer[0];
                    
                    if in_header {
                        header_buffer.push(byte);
                        
                        // Check if we've completed the header
                        if header_buffer.ends_with(b"\r\n\r\n") {
                            let header_str = String::from_utf8_lossy(&header_buffer);
                            
                            // Log the header for debugging
                            eprintln!("sql-lsp-wrapper: LSP Header: {}", header_str.trim());
                            
                            // Parse Content-Length
                            for line in header_str.lines() {
                                if line.starts_with("Content-Length:") {
                                    if let Some(len_str) = line.split(':').nth(1) {
                                        content_length = len_str.trim().parse().unwrap_or(0);
                                        eprintln!("sql-lsp-wrapper: Content-Length: {}", content_length);
                                    }
                                }
                            }
                            
                            // Send the header
                            if tx.send(header_buffer.clone()).is_err() {
                                break;
                            }
                            header_buffer.clear();
                            in_header = false;
                            buffer.clear();
                        }
                    } else {
                        buffer.push(byte);
                        
                        // Check if we've read the complete content
                        if buffer.len() >= content_length {
                            // Log first part of content for debugging
                            let content_preview = String::from_utf8_lossy(&buffer[..buffer.len().min(200)]);
                            eprintln!("sql-lsp-wrapper: LSP Content (first 200 chars): {}", content_preview);
                            
                            // Send the content
                            if tx.send(buffer.clone()).is_err() {
                                break;
                            }
                            buffer.clear();
                            in_header = true;
                            content_length = 0;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Thread to forward stdin
    let stdin_thread = thread::spawn(move || {
        use std::io::Read;
        let mut stdin = io::stdin();
        let mut buffer = vec![0u8; 4096];
        
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    // Log what we're sending to the LSP server
                    let data_str = String::from_utf8_lossy(&buffer[..n.min(200)]);
                    eprintln!("sql-lsp-wrapper: Sending to LSP (first 200 chars): {}", data_str);
                    
                    if child_stdin.write_all(&buffer[..n]).is_err() {
                        break;
                    }
                    if child_stdin.flush().is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Forward filtered output to stdout
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    
    while let Ok(data) = rx.recv() {
        if stdout_lock.write_all(&data).is_err() {
            break;
        }
        if stdout_lock.flush().is_err() {
            break;
        }
    }
    
    // Wait for threads and child process
    let _ = filter_thread.join();
    let _ = stdin_thread.join();
    let _ = child.wait();
}