use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Cloudflare DDNS")]
#[command(about = "A simple DDNS tool for Cloudflare", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the application
    Run {
        /// Run once
        #[arg(long, default_value_t = true, conflicts_with = "loops")]
        once: bool,

        /// Run in loops
        #[arg(long)]
        loops: bool,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,
    },
    /// Install components
    Install {
        #[command(subcommand)]
        component: InstallComponents,
    },
}

#[derive(Subcommand)]
pub enum InstallComponents {
    /// Install service
    Service,
    /// Install schedule
    Schedule,
    /// Install cron
    #[cfg(unix)]
    Cron,
}
