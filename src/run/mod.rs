use log::{debug, error, info};
use std::sync::{Arc, LazyLock};
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::time::{Duration, sleep};

#[cfg(windows)]
use windows_services::{Command, Service};
mod load_conf;
mod update_ip;

static LOOP_SIGNAL: LazyLock<(Sender<&str>, Receiver<&str>)> = LazyLock::new(|| watch::channel(""));

fn system_signal_handler() {
    debug!("退出中...");
    LOOP_SIGNAL.0.send("stop").unwrap();
}
pub async fn run(run_type: &str) -> Result<(), String> {
    
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
            let mut rx = LOOP_SIGNAL.1.clone();
            #[cfg(windows)]
            let mut rx_pause = LOOP_SIGNAL.1.clone();

            ctrlc::set_handler(system_signal_handler).unwrap();

            loop {
                run_once().await;

                tokio::select! {
                    _ = rx.wait_for(|&signal| signal == "stop") => return Ok(()),
                    _ = sleep(Duration::from_secs(60))=>(),
                }

                #[cfg(windows)]
                tokio::select! {
                    _ = rx.wait_for(|&signal| signal == "stop") => return Ok(()),
                    _ = rx_pause.wait_for(|&signal| signal != "pause")=>(),
                }
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(windows)]
pub fn run_service_windows() -> Result<(), String> {
    let mut task: Option<std::thread::JoinHandle<()>> = None;

    Service::new()
        .can_stop()
        .can_pause()
        .run(|_, command| match command {
            Command::Start => {
                task = Some(std::thread::spawn(|| {
                    let rt_run = tokio::runtime::Runtime::new().unwrap();
                    match rt_run.block_on(run("loop")) {
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
                LOOP_SIGNAL.0.send("stop").unwrap();
                task.take().unwrap().join().unwrap();
            }
            Command::Pause => {
                debug!("收到暂停信号");
                LOOP_SIGNAL.0.send("pause").unwrap();
            }
            Command::Resume => {
                debug!("取消暂停，恢复运行...");
                LOOP_SIGNAL.0.send("").unwrap();
            }
            _ => unreachable!(),
        })?;
    Ok(())
}
