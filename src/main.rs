mod obj;
mod run;
mod initialize;
mod setup;

use initialize::{load_conf, parse_args};
use setup::{install, uninstall};

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), String> {
    match &*obj::ARGS { 
        parse_args::Commands::Run { loops, datadir: _ } => {
            load_conf::Config::init()?;
            let _logger = obj::init_log(
                load_conf::CONFIG_JSON
                    .get()
                    .ok_or("CONFIG_JSON 未初始化")?
                    .log_level,
            )?;

            #[cfg(windows)]
            if *loops {
                match run::run_service_windows() {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        if e != "Use service control manager to start service" {
                            return Err(e.to_string());
                        }
                    }
                }
            }
            run::run(*loops)?;
        }
        parse_args::Commands::Install { component } => match component {
            parse_args::InstallComponents::Service => install::service()?,
            parse_args::InstallComponents::Schedule => install::schedule()?,
            #[cfg(unix)]
            parse_args::InstallComponents::Cron => install::cron()?,
        },
        parse_args::Commands::Uninstall { component } => match component {
            parse_args::UninstallComponents::Service => uninstall::service()?,
            parse_args::UninstallComponents::Schedule => uninstall::schedule()?,
            #[cfg(unix)]
            parse_args::UninstallComponents::Cron => uninstall::cron()?,
        },
    }
    Ok(())
}
