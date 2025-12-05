mod install;
mod obj;
mod parse_args;
mod run;
mod uninstall;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<(), String> {
    match &*obj::ARGS {
        parse_args::Commands::Run {
            once: _,
            loops,
            log,
            datadir: _,
        } => {
            let _logger = obj::init_log(*log)?;
            if *loops {
                #[cfg(windows)]
                match run::run_service_windows() {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        if e != "Use service control manager to start service" {
                            return Err(e);
                        }
                    }
                }
                run::run("loop")?;
            } else {
                run::run("once")?;
            }
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
