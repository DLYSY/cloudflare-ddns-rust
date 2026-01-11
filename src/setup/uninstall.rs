use std::process;

pub fn service() -> Result<(), String> {
    if cfg!(windows) {
        process::Command::new("sc")
            .args(["delete", "CloudflareDDNS"])
            .status()
            .map_err(|e| format!("删除服务失败，回溯错误：{e}"))?
            .success()
            .then(|| ())
            .ok_or("删除服务失败，请检查是否有管理员权限")?;
    } else if cfg!(unix) {
        std::fs::remove_file("/etc/systemd/system/cloudflareddns.service")
            .map_err(|e| format!("删除服务文件失败，请检查是否有管理员权限，回溯错误：{e}"))?;
    }
    Ok(())
}

pub fn schedule() -> Result<(), String> {
    if cfg!(windows) {
        process::Command::new("schtasks")
            .args(["/delete", "/tn", "CloudflareDDNS", "/f"])
            .status()
            .map_err(|e| format!("删除计划任务失败，回溯错误：{e}"))?
            .success()
            .then_some(())
            .ok_or("删除计划任务失败，请检查是否有管理员权限".to_string())?;
    } else if cfg!(unix) {
        std::fs::remove_file("/etc/systemd/system/cloudflareddns.service").and(
            std::fs::remove_file("/etc/systemd/system/cloudflareddns.timer"),
        ).map_err(|e| format!("删除systemd timer失败，请检查是否有管理员权限，回溯错误：{e}"))?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn cron() -> Result<(), String> {
    process::Command::new("sh")
        .arg("-c")
        .arg("crontab -l 2>/dev/null | grep -v \"run --once\" | crontab -")
        .status()
        .map_err(|e| format!("删除计划任务失败，回溯错误：{e}"))?;
    Ok(())
}
