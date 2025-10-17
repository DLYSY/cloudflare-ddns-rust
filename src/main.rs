// std
use std::env::{current_dir, current_exe};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
// 日志
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming, WriteMode,
    colored_detailed_format, detailed_format,
};
#[cfg(windows)]
use log::error;
use log::{debug, info};
// 异步
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::time::sleep;
// 环境
use clap::Parser;
#[cfg(windows)]
use windows_services::{Command, Service};

mod install;
mod load_conf;
mod parse_args;
mod update_ip;

pub static DATA_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    let root_dir = if cfg!(debug_assertions) {
        current_dir().expect("无法读取当前工作目录")
    } else {
        current_exe()
            .expect("无法读取二进制文件路径")
            .parent()
            .expect("无法读取二进制文件所在目录")
            .to_path_buf()
    };
    root_dir.join("data")
});

fn init_log(debug_mod: bool) -> Result<LoggerHandle, String> {
    let log_level = if cfg!(debug_assertions) || debug_mod {
        "debug"
    } else {
        "info"
    };
    let logger = Logger::try_with_str(log_level)
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory(DATA_DIR.join("logs")) //定义日志文件位置
                .basename("ddns"),
        ) //定义日志文件名，不包含后缀
        .duplicate_to_stdout(Duplicate::Debug) //复制日志到控制台
        .rotate(
            Criterion::Age(Age::Day), // 按天轮转
            Naming::TimestampsCustomFormat {
                current_infix: None,
                format: "%Y-%m-%d",
            }, // 文件名包含日期并以天为单位轮换
            Cleanup::KeepCompressedFiles(15), // 保留15天日志并启用压缩
        )
        .format_for_stdout(colored_detailed_format) //控制台输出彩色带时间的日志格式
        .format_for_files(detailed_format) //文件中使用ANSI颜色会乱码，所以使用无颜色格式
        .write_mode(WriteMode::Async)
        .append() //指定日志文件为添加内容而不是覆盖重写
        .start()
        .map_err(|e| format!("无法创建logger句柄,回溯错误:\n{e}"))?;
    debug!("日志初始化成功");
    Ok(logger)
}

async fn run(
    run_type: &str,
    tx: Sender<&'static str>,
    rx: Receiver<&'static str>,
) -> Result<(), String> {
    let config_json = load_conf::init_conf()?;

    let ipv4_config: Arc<Vec<&load_conf::DnsRecord>> = Arc::new(
        config_json
            .iter()
            .filter(|&x| x.record_type == Arc::new("A".to_string()))
            .collect(),
    );
    let ipv6_config: Arc<Vec<&load_conf::DnsRecord>> = Arc::new(
        config_json
            .iter()
            .filter(|&x| x.record_type == Arc::new("AAAA".to_string()))
            .collect(),
    );

    let run_once = || async {
        tokio::join!(
            update_ip::update_ip(4, ipv4_config.clone()),
            update_ip::update_ip(6, ipv6_config.clone())
        );
        info!("本次更新完成");
    };

    match run_type {
        "once" => {
            run_once().await;
            return Ok(());
        }
        "loop" => {
            let mut rx2 = rx.clone();
            #[cfg(windows)]
            let mut rx3 = rx.clone();

            ctrlc::set_handler(move || {
                debug!("退出中...");
                tx.send("stop").unwrap();
            })
            .unwrap();

            loop {
                run_once().await;

                tokio::select! {
                    _ = rx2.wait_for(|&val| val == "stop") => return Ok(()),
                    _ = sleep(Duration::from_secs(60))=>(),
                }

                #[cfg(windows)]
                tokio::select! {
                    _ = rx2.wait_for(|&val| val != "pause")=>(),
                    _ = rx3.wait_for(|&val| val == "stop") => return Ok(()),
                }
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(windows)]
fn run_service_windows() -> Result<(), String> {
    let mut task: Option<std::thread::JoinHandle<()>> = None;

    let (tx, rx) = watch::channel("");

    Service::new()
        .can_stop()
        .can_pause()
        .run(|_, command| match command {
            Command::Start => {
                let tx2 = tx.clone();
                let rx2 = rx.clone();
                task = Some(std::thread::spawn(|| {
                    let rt_run = tokio::runtime::Runtime::new().unwrap();
                    match rt_run.block_on(run("loop", tx2, rx2)) {
                        Ok(()) => (),
                        Err(_) => {
                            error!("检测到服务环境，强制退出进程...");
                            std::process::exit(1)
                        }
                    }
                }));
            }
            Command::Stop => {
                debug!("服务退出中...");
                tx.send("stop").unwrap();
                task.take().unwrap().join().unwrap();
            }
            Command::Pause => {
                debug!("收到暂停信号");
                tx.send("pause").unwrap();
            }
            Command::Resume => {
                debug!("取消暂停，恢复运行...");
                tx.send("").unwrap();
            }
            _ => unreachable!(),
        })?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    match parse_args::CliArgs::parse().command {
        parse_args::Commands::Run {
            once: _,
            loops,
            debug,
        } => {
            let _logger = init_log(debug)?;
            if loops {
                #[cfg(windows)]
                match run_service_windows() {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        if e != "Use service control manager to start service" {
                            return Err(e);
                        }
                    }
                }
                let (tx, rx) = watch::channel("");
                run("loop", tx, rx).await?;
            } else {
                let (tx, rx) = watch::channel("");
                run("once", tx, rx).await?;
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
