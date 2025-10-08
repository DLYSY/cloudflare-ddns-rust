use clap::Parser;
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming, WriteMode,
    colored_detailed_format, detailed_format,
};
use futures::future::join_all;
use json::{self};
use log::{debug, error, info};
use std::env::{current_dir, current_exe};
use std::fs;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::time::sleep;
use tokio::{self, select, sync::Notify};

mod install;
mod parse_args;
mod update_ip;

static DATA_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
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

fn init_log() -> Result<LoggerHandle, String> {
    let log_level = if cfg!(debug_assertions) {
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

async fn run(run_type: &str, exit_signal: Option<Arc<Notify>>) -> Result<(), String> {
    let config_json_text = match fs::read_to_string(DATA_DIR.join("config.json")) {
        Ok(success) => {
            debug!("成功读取配置文件");
            success
        }
        Err(_) => {
            error!("读取失败,请检查config.json是否存在并使用UTF-8编码");
            return Err("读取失败,请检查config.json是否存在并使用UTF-8编码".to_string());
        }
    };
    let config_json = match json::parse(config_json_text.as_str()) {
        Ok(success) => {
            debug!("成功解析json");
            success
        }
        Err(_) => {
            error!("json文件格式不正确");
            return Err("json文件格式不正确".to_string());
        }
    };
    let mut have_ipv4 = false;
    let mut have_ipv6 = false;

    for i in config_json.members() {
        if i["type"] == "A" {
            debug!("找到A记录需要解析");
            have_ipv4 = true;
        } else if i["type"] == "AAAA" {
            debug!("找到AAAA记录需要解析");
            have_ipv6 = true;
        }
        if have_ipv4 && have_ipv6 {
            break;
        }
    }

    let run_once = || async {
        let mut update_tasks = Vec::new();
        if have_ipv4 {
            update_tasks.push(update_ip::update_ip(4, &config_json));
        }
        if have_ipv6 {
            update_tasks.push(update_ip::update_ip(6, &config_json));
        }
        join_all(update_tasks).await;
        info!("本次更新完成");
    };

    match run_type {
        "once" => {
            run_once().await;
            return Ok(());
        }
        "loop" => {
            let exit_signal = exit_signal.unwrap_or_else(|| Arc::new(Notify::new()));
            let exit_signal_recv = exit_signal.clone();
            ctrlc::set_handler(move || {
                debug!("开始退出");
                exit_signal.notify_one();
            })
            .unwrap();
            loop {
                run_once().await;
                select! {
                    _ = sleep(Duration::from_secs(90))=>(),
                    _ = exit_signal_recv.notified() => return Ok(())
                }
            }
        }
        _ => return Ok(()),
    }
}

#[cfg(windows)]
fn run_service_windows() -> Result<(), String> {
    let mut task: Option<std::thread::JoinHandle<Result<(), String>>> = None;
    let exit_signal = Arc::new(Notify::new());

    windows_services::Service::new()
        .can_stop()
        .run(|_, command| match command {
            windows_services::Command::Start => {
                let signal = exit_signal.clone();
                task = Some(std::thread::spawn(|| {
                    let _logger = init_log().unwrap();
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(run("loop", Some(signal)))
                }));
            }
            windows_services::Command::Stop => {
                exit_signal.notify_one();
                task.take().unwrap().join().unwrap().unwrap();
            }
            _ => {}
        })?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    #[cfg(windows)]
    match run_service_windows() {
        Ok(_) => return Ok(()),
        Err(e) => {
            if e != "Use service control manager to start service" {
                return Err(e);
            }
        }
    }

    match parse_args::CliArgs::parse().command {
        parse_args::Commands::Run { once: _, loops } => {
            let _logger = init_log()?;
            if loops {
                run("loop", None).await?;
            } else {
                run("once", None).await?;
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
