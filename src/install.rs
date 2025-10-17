use std::env::current_exe;
use std::process;

pub fn service() -> Result<(), String> {
    if cfg!(windows) {
        process::Command::new("sc")
            .args([
                "create",
                "CloudflareDDNS",
                "start=delayed-auto",
                "binPath=",
                format!("\"{}\" run --loops", std::env::current_exe().unwrap().display()).as_str(),
            ])
            .status()
            .map_err(|e| format!("创建服务失败，回溯错误：{e}"))?
            .success()
            .then(|| ())
            .ok_or("创建服务失败，请检查是否有管理员权限")?;
    } else if cfg!(unix) {
        let service_file = concat!(
            "[Unit]\n",
            "Description=CloudflareDDNS Service\n",
            "After=network.target\n\n",
            "[Service]\n",
            "Type=simple\n",
            "ExecStart={} run --loops\n",
            "Restart=on-failure\n",
            "KillSignal=SIGINT\n",
            "TimeoutStopSec=20\n\n",
            "[Install]\n",
            "WantedBy=multi-user.target"
        )
        .replace("{}", current_exe().unwrap().to_str().unwrap());

        std::fs::write("/etc/systemd/system/cloudflareddns.service", service_file)
            .map_err(|e| format!("创建服务失败，请检查是否有管理员权限，回溯错误：{e}"))?;
    }
    Ok(())
}

pub async fn schedule() -> Result<(), String> {
    if cfg!(windows) {
        process::Command::new("schtasks")
            .args([
                "/create",
                "/tn",
                "CloudflareDDNS",
                "/sc",
                "MINUTE",
                "/mo",
                "2",
                "/tr",
                format!("\"{}\" run --once", std::env::current_exe().unwrap().display()).as_str(),
                "/ru",
                "System",
            ])
            .status()
            .map_err(|e| format!("创建计划任务失败，回溯错误：{e}"))?
            .success()
            .then(|| ())
            .ok_or("创建计划任务失败，请检查是否有管理员权限")?;
    } else if cfg!(unix) {
        let service_file = concat!(
            "[Unit]\n",
            "Description=CloudflareDDNS Once Service\n\n",
            "[Service]\n",
            "Type=oneshot\n",
            "ExecStart={} run --once",
        )
        .replace("{}", current_exe().unwrap().to_str().unwrap());

        let timer_file = concat!(
            "[Unit]\n",
            "Description=Runs CloudflareDDNS Once Service every 2 minutes\n",
            "After=network.target\n\n",
            "[Timer]\n",
            "OnBootSec=2min\n",
            "OnUnitActiveSec=2min\n\n",
            "[Install]\n",
            "WantedBy=timers.target"
        );

        tokio::try_join!(
            tokio::fs::write("/etc/systemd/system/cloudflareddns.service", service_file,),
            tokio::fs::write("/etc/systemd/system/cloudflareddns.timer", timer_file,)
        )
        .map_err(|e| format!("创建systemd timer失败，请检查是否有管理员权限，回溯错误：{e}"))?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn cron() -> Result<(), String> {
    let cron_job = format!(
        "*/2 * * * * {} run --once\n",
        std::env::current_exe().unwrap().display()
    );
    process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "(crontab -l 2>/dev/null; echo \"{}\") | crontab -",
            cron_job
        ))
        .status()
        .map_err(|e| format!("创建计划任务失败，回溯错误：{e}"))?;
    Ok(())
}
