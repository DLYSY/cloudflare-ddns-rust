#[allow(unused_imports)]
use log::{debug, error, info};
use std::sync::LazyLock;
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::time::{Duration, sleep};

#[cfg(windows)]
use windows_services::{Command, Service};
mod load_conf;
mod update_ip;

pub enum RunType {
    Once,
    Loops,
}

#[derive(PartialEq)]
enum SignalType {
    Run,
    Stop,
    Pause
}

static LOOP_SIGNAL: LazyLock<(Sender<SignalType>, Receiver<SignalType>)> = LazyLock::new(|| watch::channel(SignalType::Run));

fn system_signal_handler() {
    debug!("退出中...");
    LOOP_SIGNAL.0.send(SignalType::Stop).unwrap();
}

#[tokio::main(flavor = "current_thread")]
pub async fn run(run_type: RunType) -> Result<(), &'static str> {
    let config_json = load_conf::init_conf()?;

    let ipv4_config: Vec<&load_conf::DnsRecord> = config_json
        .iter()
        .filter(|&x| x.record_type == "A")
        .collect();
    let ipv6_config: Vec<&load_conf::DnsRecord> = config_json
        .iter()
        .filter(|&x| x.record_type == "AAAA")
        .collect();

    let run_once = || async {
        tokio::join!(
            update_ip::update_ip(4, &ipv4_config),
            update_ip::update_ip(6, &ipv6_config)
        );
        info!("本次更新完成");
    };

    match run_type {
        RunType::Once => {
            run_once().await;
            return Ok(());
        }
        RunType::Loops => {
            let mut rx = LOOP_SIGNAL.1.clone();
            #[cfg(windows)]
            let mut rx_pause = LOOP_SIGNAL.1.clone();

            ctrlc::set_handler(system_signal_handler).unwrap();


            loop {
                run_once().await;

                tokio::select! {
                    _ = rx.wait_for(|signal| signal == &SignalType::Stop) => return Ok(()),
                    _ = sleep(Duration::from_secs(60))=>(),
                }

                #[cfg(windows)]
                tokio::select! {
                    _ = rx.wait_for(|signal| signal == &SignalType::Stop) => return Ok(()),
                    _ = rx_pause.wait_for(|signal| signal != &SignalType::Pause)=>(),
                }
            }
        }
    }
}

#[cfg(windows)]
pub fn run_service_windows() -> Result<(), &'static str> {
    let mut task: Option<std::thread::JoinHandle<()>> = None;

    Service::new()
        .can_stop()
        .can_pause()
        .run(|_, command| match command {
            Command::Start => {
                task = Some(std::thread::spawn(|| match run(RunType::Loops) {
                    Ok(()) => (),
                    Err(_) => {
                        error!("检测到服务环境，强制退出进程...");
                        std::process::exit(1)
                    }
                }));
            }
            Command::Stop => {
                debug!("服务退出中...");
                LOOP_SIGNAL.0.send(SignalType::Stop).unwrap();
                task.take().unwrap().join().unwrap();
            }
            Command::Pause => {
                debug!("收到暂停信号");
                LOOP_SIGNAL.0.send(SignalType::Pause).unwrap();
            }
            Command::Resume => {
                debug!("取消暂停，恢复运行...");
                LOOP_SIGNAL.0.send(SignalType::Run).unwrap();
            }
            Command::Extended(_) => unreachable!(),
        })?;
    Ok(())
}
