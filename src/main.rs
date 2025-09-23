//异步运行时
use futures::future::join_all;
use tokio;
use tokio::time::sleep;
//标准库
#[cfg(debug_assertions)]
use std::env::current_dir;
#[cfg(not(debug_assertions))]
use std::env::current_exe;
use std::fs::read_to_string;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;
//日志
#[cfg(not(debug_assertions))]
use tklog::MODE;
use tklog::{
    ASYNC_LOG, Format, LEVEL, LOG, async_debug, async_error, async_fatal, async_info, info,
};
//io
use json::{self, JsonValue};
use reqwest::{self, Client, ClientBuilder, retry};

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .no_proxy()
        .https_only(true)
        .gzip(true)
        .connect_timeout(Duration::new(5, 0))
        .read_timeout(Duration::new(5, 0))
        .retry(retry::for_host("*").max_retries_per_request(3))
        .build()
        .unwrap()
});

async fn ask_api(ip: IpAddr, info: &mut JsonValue) -> Result<(), ()> {
    let api_url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
        info.remove("zone_id"),
        info.remove("dns_id")
    );
    let header = format!("Bearer {}", info.remove("api_token"));
    info["content"] = ip.to_string().into();
    match CLIENT
        .put(api_url)
        .body(info.dump())
        .header("Content-Type", "application/json")
        .header("Authorization", header)
        .send()
        .await
    {
        Ok(success) => {
            if success.status().is_success() {
                async_debug!("更新: ", info["name"], "类型: ", info["type"], "成功");
                // success
            } else {
                async_error!(
                    "更新:",
                    info["name"],
                    ",类型:",
                    info["type"],
                    "时服务器返回码:",
                    success.status().as_u16()
                );
                return Err(());
            }
        }
        Err(error) => {
            async_debug!("{error}");
            if error.is_timeout() {
                async_error!("更新: ", info["name"], "类型: ", info["type"], "时链接超时");
            } else if error.is_connect() {
                async_error!("更新: ", info["name"], "类型: ", info["type"], "时链接错误");
            } else {
                async_error!(
                    "更新: ",
                    info["name"],
                    "类型: ",
                    info["type"],
                    "时发生其他错误: ",
                    error
                );
            }
            return Err(());
        }
    };
    Ok(())
}

async fn update_ip(ip_version: u8, global_config_json: &JsonValue) {
    let ip = match get_ip(ip_version).await {
        Ok(success) => {
            async_debug!("获取IPv", ip_version, "成功");
            success
        }
        Err(_) => return,
    };

    let mut self_config_json = global_config_json.clone();
    let mut ask_api_list = Vec::new();
    for i in self_config_json.members_mut() {
        if i["type"] == "A" && ip_version == 4 {
            ask_api_list.push(ask_api(ip, i));
        } else if i["type"] == "AAAA" && ip_version == 6 {
            ask_api_list.push(ask_api(ip, i));
        }
    }
    join_all(ask_api_list).await;
}

async fn get_ip(ip_version: u8) -> Result<IpAddr, ()> {
    let ip_response = match CLIENT
        .get(format!("https://ipv{ip_version}.icanhazip.com/"))
        .send()
        .await
    {
        Ok(success) => {
            if success.status().is_success() {
                success
            } else {
                async_error!(
                    "获取ipv",
                    ip_version,
                    "时状态码不正确",
                    success.status().as_u16()
                );
                return Err(());
            }
        }
        Err(error) => {
            if error.is_timeout() {
                async_error!("获取ipv", ip_version, "时链接超时")
            } else if error.is_connect() {
                async_error!("获取ipv", ip_version, "时链接错误")
            } else {
                async_error!("获取ipv", ip_version, "时发生其他错误: ", error)
            }
            return Err(());
        }
    };

    let ip_text = match ip_response.text().await {
        Ok(success) => success,
        Err(error) => {
            async_error!("获取IPv", ip_version, "时响应正文时发生错误: ", error);
            return Err(());
        }
    };

    match ip_version {
        4 => match Ipv4Addr::from_str(ip_text.trim()) {
            Ok(ip) => return Ok(IpAddr::V4(ip)),
            Err(_) => {
                async_error!("获取到格式不正确的ipv4");
                return Err(());
            }
        },
        6 => match Ipv6Addr::from_str(ip_text.trim()) {
            Ok(ip) => return Ok(IpAddr::V6(ip)),
            Err(_) => {
                async_error!("获取到格式不正确的ipv6");
                return Err(());
            }
        },
        _ => {
            async_fatal!("程序内部错误: get_ip函数获取到不可能的值: ", ip_version);
            panic!("程序内部错误: get_ip函数获取到不可能的值{ip_version}");
        }
    }
}

#[tokio::main]
async fn main() {
    /*当前二进制文件路径*/
    #[cfg(not(debug_assertions))]
    let exe_dir = {
        let mut path = current_exe().expect("无法读取二进制文件位置");
        path.pop();
        path
    };
    /*初始化日志*/
    ASYNC_LOG
        .set_console(true)
        .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName);
    // LOG.set_console(true)
    //     .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName);

    #[cfg(not(debug_assertions))]
    ASYNC_LOG
        .set_level(LEVEL::Info)
        .set_cutmode_by_time(
            exe_dir.join("logs/ddns.log").to_str().unwrap(),
            MODE::DAY,
            15,
            true,
        )
        .await;
    // #[cfg(not(debug_assertions))]
    // LOG.set_level(LEVEL::Info).set_cutmode_by_time(
    //     exe_dir.join("logs/ddns.log").to_str().unwrap(),
    //     MODE::DAY,
    //     15,
    //     true,
    // );

    #[cfg(debug_assertions)]
    ASYNC_LOG.set_level(LEVEL::Debug);
    // LOG.set_level(LEVEL::Debug);

    #[cfg(not(debug_assertions))]
    let config_path = exe_dir.join("config.json");
    #[cfg(debug_assertions)]
    let config_path = current_dir().unwrap().join("config.json");

    let config_json_text = match read_to_string(config_path) {
        Ok(success) => {
            async_debug!("成功读取配置文件");
            success
        }
        Err(_) => {
            async_fatal!("读取失败,请检查config.json是否存在并使用UTF-8编码");
            panic!("读取失败,请检查config.json是否存在并使用UTF-8编码");
        }
    };
    let config_json = match json::parse(config_json_text.as_str()) {
        Ok(success) => {
            async_debug!("成功解析json");
            success
        }
        Err(_) => {
            async_fatal!("json文件格式不正确");
            panic!("json文件格式不正确")
        }
    };

    let mut have_ipv4 = false;
    let mut have_ipv6 = false;

    for i in config_json.members() {
        if i["type"] == "A" {
            async_debug!("找到A记录需要解析");
            have_ipv4 = true;
        } else if i["type"] == "AAAA" {
            async_debug!("找到AAAA记录需要解析");
            have_ipv6 = true;
        }
        if have_ipv4 && have_ipv6 {
            break;
        }
    }

    let mut update_tasks = Vec::new();
    if have_ipv4 {
        update_tasks.push(update_ip(4, &config_json));
    }
    if have_ipv6 {
        update_tasks.push(update_ip(6, &config_json));
    }

    join_all(update_tasks).await;
    async_info!("本次更新任务完成");
    sleep(Duration::from_millis(500)).await;
}
