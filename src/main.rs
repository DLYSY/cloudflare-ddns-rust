use clap::Parser;

mod run;
mod obj;
mod install;
mod parse_args;

#[tokio::main]
async fn main() -> Result<(), String> {
    match parse_args::CliArgs::parse().command {
        parse_args::Commands::Run {
            once: _,
            loops,
            debug,
        } => {
            let _logger = obj::init_log(debug)?;
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
    }
    Ok(())
}
