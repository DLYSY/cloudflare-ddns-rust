use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "myapp")]
#[command(about = "A CLI application with run and install commands", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the application
    Run {
        /// Run once
        #[arg(long,default_value_t = true, conflicts_with = "loops")]
        once: bool,
        
        /// Run in loops
        #[arg(long)]
        loops: bool,
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