use futures::future;
use log::{debug, warn};
use reqwest::{self, Client, ClientBuilder, Version, retry, tls};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

use crate::load_conf;
// mod load_conf;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let time_out_secs = Duration::from_secs(5);
    ClientBuilder::new()
        .no_proxy()
        .retry(retry::for_host("*").max_retries_per_request(3))
        .https_only(true)
        .http2_prior_knowledge()
        .gzip(true)
        .pool_idle_timeout(None)
        .connect_timeout(time_out_secs)
        .read_timeout(time_out_secs)
        .min_tls_version(tls::Version::TLS_1_3)
        .build()
        .unwrap()
});

static mut IPV4ADDR: Option<Ipv4Addr> = None;
static mut IPV6ADDR: Option<Ipv6Addr> = None;

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
        _ => unreachable!("程序内部错误：get_ip函数获取到不可能的值{ip_version}"),
    }
}

async fn ask_api(ip: IpAddr, info: &load_conf::DnsRecord) -> Result<(), ()> {
    let mut info = info.clone();
    info.content = Some(ip.to_string());

    match CLIENT
        .put(format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            info.zone_id.take().unwrap(),
            info.dns_id.take().unwrap()
        ))
        .bearer_auth(info.api_token.take().unwrap())
        .json(&info)
        // .body(info.dump())
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(success) => {
            if success.status().is_success() {
                debug_assert_eq!(success.version(), Version::HTTP_2);
                debug!("更新: {}, 类型: {}成功", info.name, info.record_type);
                // success
            } else {
                warn!(
                    "更新:{},类型:{}时服务器返回码:{}",
                    info.name,
                    info.record_type,
                    success.status().as_u16()
                );
                return Err(());
            }
        }
        Err(error) => {
            debug!("{error}");
            if error.is_timeout() {
                warn!("更新:{},类型:{}时链接超时", info.name, info.record_type);
            } else if error.is_connect() {
                warn!("更新:{},类型:{}时链接错误", info.name, info.record_type);
            } else {
                warn!(
                    "更新{}类型{}时发生未知错误:{}",
                    info.name, info.record_type, error
                );
            }
            return Err(());
        }
    };
    Ok(())
}

pub async fn update_ip(ip_version: u8, global_config_json: Vec<&load_conf::DnsRecord>) {
    if global_config_json.is_empty() {
        debug!("没有需要更新的IPv{ip_version}记录");
        return;
    }

    let ip = match get_ip(ip_version).await {
        Ok(success) => {
            debug!("获取IPv{ip_version}成功");
            success
        }
        Err(_) => return,
    };
    // 检查IP是否变化
    unsafe {
        if ip_version == 4 {
            if let Some(old_ip) = IPV4ADDR {
                if old_ip == ip {
                    debug!("IPv4地址未改变，跳过更新");
                    return;
                }
            }
            IPV4ADDR = Some(match ip {
                IpAddr::V4(ipv4) => ipv4,
                _ => unreachable!(),
            });
        } else if ip_version == 6 {
            if let Some(old_ip) = IPV6ADDR {
                if old_ip == ip {
                    debug!("IPv6地址未改变，跳过更新");
                    return;
                }
            }
            IPV6ADDR = Some(match ip {
                IpAddr::V6(ipv6) => ipv6,
                _ => unreachable!(),
            });
        }
    }
    
    future::join_all(global_config_json.iter().map(|&x| ask_api(ip, x))).await;
}
