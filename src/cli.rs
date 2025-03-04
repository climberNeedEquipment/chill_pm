use clap::Parser;

/// Chill PM Web Server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Environment to use (test or prod)
    #[arg(long, default_value = "test")]
    pub environment: String,

    /// Port to run the server on
    #[arg(long, default_value = "8080")]
    pub port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
}
