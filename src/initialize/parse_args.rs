use clap::{Parser, Subcommand, ValueEnum};
use flexi_logger::LogSpecification;

#[derive(Parser)]
#[command(name = "Cloudflare DDNS")]
#[command(about = "A simple DDNS tool for Cloudflare", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}
impl LogLevel {
    pub fn to_loglevel(&self) -> LogSpecification {
        match self {
            LogLevel::Trace => LogSpecification::trace(),
            LogLevel::Debug => LogSpecification::debug(),
            LogLevel::Info => LogSpecification::info(),
            LogLevel::Warn => LogSpecification::warn(),
            LogLevel::Error => LogSpecification::error(),
            LogLevel::Off => LogSpecification::off(),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the application
    Run {
        /// Run in loops(Default is run once)
        #[arg(long)]
        loops: bool,

        /// data path, default is <current execute>/data
        #[arg(long)]
        datadir: Option<std::path::PathBuf>,
    },
    /// Install components
    Install {
        #[command(subcommand)]
        component: InstallComponents,
    },
    /// Uninstall components
    Uninstall {
        #[command(subcommand)]
        component: UninstallComponents,
    },
}

#[derive(Subcommand)]
pub enum InstallComponents {
    /// Install as a system service (Windows service or systemd service)
    Service,
    /// Install schedule task (Windows task scheduler or systemd timer)
    Schedule,
    /// Install as a cron job (Unix-like systems only)
    #[cfg(unix)]
    Cron,
}

#[derive(Subcommand)]
pub enum UninstallComponents {
    /// Uninstall the system service
    Service,
    /// Uninstall the schedule task
    Schedule,
    /// Uninstall the cron job (Unix-like systems only)
    #[cfg(unix)]
    Cron,
}
