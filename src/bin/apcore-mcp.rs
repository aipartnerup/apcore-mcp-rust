//! Binary entry point for the apcore-mcp server.

#[tokio::main]
async fn main() {
    match apcore_mcp::cli::run().await {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(e.exit_code());
        }
    }
}
