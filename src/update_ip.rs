use futures::future::join_all;
use json::{self, JsonValue};
use log::{debug, error, warn};
use reqwest::{self, Client, ClientBuilder, Version, retry};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let time_out_secs = Duration::from_secs(5);
    ClientBuilder::new()
        .no_proxy()
        .retry(retry::for_host("*").max_retries_per_request(3))
        .https_only(true)
        .http2_prior_knowledge()
        .gzip(true)
        .connect_timeout(time_out_secs)
        .read_timeout(time_out_secs)
        .build()
        .unwrap()
});

async fn get_ip(ip_version: u8) -> Result<IpAddr, ()> {
    let ip_response = match CLIENT
        .get(format!("https://ipv{ip_version}.icanhazip.com/"))
        .send()
        .await
    {
        Ok(success) => {
            if success.status().is_success() {
                debug_assert_eq!(success.version(), Version::HTTP_2);
                success
            } else {
                warn!(
                    "获取ipv{ip_version}时状态码不正确{}",
                    success.status().as_u16()
                );
                return Err(());
            }
        }
        Err(error) => {
            if error.is_timeout() {
                warn!("获取ipv{ip_version}时链接超时")
            } else if error.is_connect() {
                warn!("获取ipv{ip_version}时链接错误{error}")
            } else {
                warn!("获取ipv{ip_version}时发生未定义错误{}", error)
            }
            return Err(());
        }
    };

    let ip_text = match ip_response.text().await {
        Ok(success) => success,
        Err(error) => {
            warn!("获取IPv{ip_version}时响应正文时发生错误：{error}");
            return Err(());
        }
    };

    match ip_version {
        4 => match Ipv4Addr::from_str(ip_text.trim()) {
            Ok(ip) => return Ok(IpAddr::V4(ip)),
            Err(_) => {
                warn!("获取到格式不正确的ipv4");
                return Err(());
            }
        },
        6 => match Ipv6Addr::from_str(ip_text.trim()) {
            Ok(ip) => return Ok(IpAddr::V6(ip)),
            Err(_) => {
                warn!("获取到格式不正确的ipv6");
                return Err(());
            }
        },
        _ => {
            error!("程序内部错误：get_ip函数获取到不可能的值{ip_version}");
            panic!();
        }
    }
}

async fn ask_api(ip: IpAddr, info: &mut JsonValue) -> Result<(), ()> {
    info["content"] = ip.to_string().into();

    match CLIENT
        .put(format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            info.remove("zone_id"),
            info.remove("dns_id")
        ))
        .bearer_auth(info.remove("api_token"))
        .body(info.dump())
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(success) => {
            if success.status().is_success() {
                debug_assert_eq!(success.version(), Version::HTTP_2);
                debug!("更新: {}, 类型: {}成功", info["name"], info["type"]);
                // success
            } else {
                warn!(
                    "更新:{},类型:{}时服务器返回码:{}",
                    info["name"],
                    info["type"],
                    success.status().as_u16()
                );
                return Err(());
            }
        }
        Err(error) => {
            debug!("{error}");
            if error.is_timeout() {
                warn!("更新:{},类型:{}时链接超时", info["name"], info["type"]);
            } else if error.is_connect() {
                warn!("更新:{},类型:{}时链接错误", info["name"], info["type"]);
            } else {
                warn!(
                    "更新{}类型{}时发生未知错误:{}",
                    info["name"], info["type"], error
                );
            }
            return Err(());
        }
    };
    Ok(())
}

pub async fn update_ip(ip_version: u8, global_config_json: &JsonValue) {
    let ip = match get_ip(ip_version).await {
        Ok(success) => {
            debug!("获取IPv{ip_version}成功");
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
