#[allow(unused_imports)]
use log::{debug, error, info};
use std::sync::LazyLock;
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

#[cfg(windows)]
use windows_services::{Command, Service};

use crate::initialize::load_conf::{self, RecordType};
use crate::run::update_ip::update_ip;
mod update_ip;

#[derive(PartialEq)]
enum SignalType {
    Run,
    Stop,
    #[cfg(windows)]
    Pause,
}

static LOOP_SIGNAL: LazyLock<(Sender<SignalType>, Receiver<SignalType>)> =
    LazyLock::new(|| watch::channel(SignalType::Run));

fn system_signal_handler() {
    debug!("退出中...");
    LOOP_SIGNAL.0.send(SignalType::Stop).unwrap();
}

pub fn run(loops_run: bool) -> Result<(), String> {
    let conf: &'static load_conf::Config = load_conf::CONFIG
        .get()
        .ok_or("运行run函数时，CONFIG_JSON 未初始化")?;

    type AsyncTask<T> = Box<dyn Fn() -> JoinHandle<T> + Send + Sync>;

    let run_once: AsyncTask<()> = if !conf.ipv4_records.is_empty() && conf.ipv6_records.is_empty() {
        info!("没有找到需要更新的ipv6记录");
        Box::new(move || tokio::spawn(update_ip(RecordType::A, &conf.ipv4_records, &conf.ipv4_url)))
    } else if conf.ipv4_records.is_empty() && !conf.ipv6_records.is_empty() {
        info!("没有找到需要更新的ipv4记录");
        Box::new(move || {
            tokio::spawn(update_ip(
                RecordType::AAAA,
                &conf.ipv6_records,
                &conf.ipv6_url,
            ))
        })
    } else if !conf.ipv4_records.is_empty() && !conf.ipv6_records.is_empty() {
        Box::new(move || {
            tokio::spawn(async move {
                tokio::join!(
                    update_ip(RecordType::A, &conf.ipv4_records, &conf.ipv4_url),
                    update_ip(RecordType::AAAA, &conf.ipv6_records, &conf.ipv6_url)
                );
            })
        })
    } else {
        return Err("未配置任何dns_records".to_string());
    };

    if conf.mutli_thread {
        tokio::runtime::Builder::new_multi_thread()
    } else {
        tokio::runtime::Builder::new_current_thread()
    }
    .enable_all()
    .build()
    .map_err(|e| {
        let e = format!("无法创建tokio runtime，回溯错误：{e}");
        error!("{e}");
        e
    })?
    .block_on(async {
        if loops_run {
            let mut rx = LOOP_SIGNAL.1.clone();
            #[cfg(windows)]
            let mut rx_pause = LOOP_SIGNAL.1.clone();

            ctrlc::set_handler(system_signal_handler).map_err(|e| {
                let e = format!("无法创建系统信号处理器 | {e}");
                error!("{e}");
                e
            })?;

            loop {
                run_once().await.map_err(|e| {
                    let e = format!("tokio运行时错误: {e}");
                    error!("{e}");
                    e
                })?;
                info!("本次更新完成");

                tokio::select! {
                    _ = rx.wait_for(|signal| signal == &SignalType::Stop) => return Ok(()),
                    _ = sleep(Duration::from_secs(conf.delay))=>(),
                }

                #[cfg(windows)]
                tokio::select! {
                    _ = rx.wait_for(|signal| signal == &SignalType::Stop) => return Ok(()),
                    _ = rx_pause.wait_for(|signal| signal != &SignalType::Pause)=>(),
                }
            }
        } else {
            run_once().await.map_err(|e| {
                let e = format!("tokio运行时错误: {e}");
                error!("{e}");
                e
            })?;
            info!("更新完成");
            Ok(())
        }
    })
}

#[cfg(windows)]
fn send_service_signal(signal: SignalType) {
    LOOP_SIGNAL
        .0
        .send(signal)
        .map_err(|e| {
            let e = format!("LOOP_SIGNAL 已关闭 | {e}");
            error!("{e}");
            e
        })
        .expect("LOOP_SIGNAL 已关闭");
}

#[cfg(windows)]
pub fn run_service_windows() -> Result<(), &'static str> {
    let mut task: Option<std::thread::JoinHandle<()>> = None;

    Service::new()
        .can_stop()
        .can_pause()
        .run(|_, command| match command {
            Command::Start => {
                task = Some(std::thread::spawn(|| match run(true) {
                    Ok(()) => (),
                    Err(_) => {
                        error!("检测到服务环境，强制退出进程...");
                        std::process::exit(1)
                    }
                }));
            }
            Command::Stop => {
                debug!("服务退出中...");
                send_service_signal(SignalType::Stop);
                task.take().unwrap().join().unwrap();
            }
            Command::Pause => {
                debug!("收到暂停信号");
                send_service_signal(SignalType::Pause);
            }
            Command::Resume => {
                debug!("取消暂停，恢复运行...");
                send_service_signal(SignalType::Run);
            }
            Command::Extended(_) => unreachable!("程序内部错误：不接受扩展命令"),
        })?;
    Ok(())
}
