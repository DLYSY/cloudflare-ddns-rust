// #[allow(unused_imports)]
// use auto_allocator;
use clap::Parser;

mod install;
mod obj;
mod parse_args;
mod run;
mod uninstall;

#[cfg(target_env = "musl")]
use mimalloc::MiMalloc;
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), String> {
    match parse_args::CliArgs::parse().command {
        parse_args::Commands::Run {
            once: _,
            loops,
            log,
        } => {
            let _logger = obj::init_log(log)?;
            if loops {
                #[cfg(windows)]
                match run::run_service_windows() {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        if e != "Use service control manager to start service" {
                            return Err(e);
                        }
                    }
                }
                run::run("loop").await?;
            } else {
                run::run("once").await?;
            }
        }
        parse_args::Commands::Install { component } => match component {
            parse_args::InstallComponents::Service => install::service()?,
            parse_args::InstallComponents::Schedule => install::schedule().await?,
            #[cfg(unix)]
            parse_args::InstallComponents::Cron => install::cron()?,
        },
        parse_args::Commands::Uninstall { component } => match component {
            parse_args::UninstallComponents::Service => uninstall::service()?,
            parse_args::UninstallComponents::Schedule => uninstall::schedule().await?,
            #[cfg(unix)]
            parse_args::UninstallComponents::Cron => uninstall::cron()?,
        },
    }
    Ok(())
}
