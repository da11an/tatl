use tatl::cli::run;

fn main() {
    if let Err(e) = run() {
        // Check if this is an internal error (database corruption, etc.)
        let error_str: String = e.to_string();
        if error_str.contains("database") || error_str.contains("constraint") || 
           error_str.contains("corruption") || error_str.contains("SQLite") ||
           error_str.contains("Failed to") {
            eprintln!("Internal error: {}", e);
            // Show error chain if available
            let mut source = e.source();
            if source.is_some() {
                eprintln!("\nCaused by:");
                let mut indent = 1;
                while let Some(err) = source {
                    eprintln!("{:indent$}  {}", "", err);
                    source = err.source();
                    indent += 1;
                }
            }
            std::process::exit(2);
        } else {
            // User error
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
