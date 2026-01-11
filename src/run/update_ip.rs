use log::{debug, warn};
use parking_lot::Mutex;
use reqwest::{self, Client, ClientBuilder, Version, retry, tls};
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
        .pool_idle_timeout(Duration::from_secs(180))
        .connect_timeout(time_out_secs)
        .read_timeout(time_out_secs)
        .min_tls_version(tls::Version::TLS_1_3)
        .build()
        .unwrap()
});

static IPV4ADDR: LazyLock<Mutex<Ipv4Addr>> =
    LazyLock::new(|| Mutex::new(Ipv4Addr::new(127, 0, 0, 1)));
static IPV6ADDR: LazyLock<Mutex<Ipv6Addr>> =
    LazyLock::new(|| Mutex::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));

async fn get_ip(ip_version: u8) -> Result<IpAddr, ()> {
    let ip_response = match CLIENT
        .get(format!("https://ipv{ip_version}.icanhazip.com/"))
        .send()
        .await
    {
        Ok(success) => success,
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

    if !ip_response.status().is_success() {
        warn!(
            "获取ipv{ip_version}时状态码不正确{}",
            ip_response.status().as_u16()
        );
        return Err(());
    }

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

async fn ask_api(ip: IpAddr, info: crate::load_conf::DnsRecord) -> Result<(), ()> {
    #[derive(Debug, serde::Serialize)]
    struct ApiBody<'a> {
        #[serde(rename = "type")]
        record_type: &'a String,
        name: &'a String,
        ttl: u32,
        proxied: bool,
        content: String,
    }
    let json_body = ApiBody {
        record_type: &info.record_type,
        name: &info.name,
        ttl: info.ttl,
        proxied: info.proxied,
        content: ip.to_string(),
    };

    match CLIENT
        .put(format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            info.zone_id, info.dns_id
        ))
        .bearer_auth(&info.api_token)
        .json(&json_body)
        .header("Content-Type", "application/json")
        .send()
        .await
    {
        Ok(success) => {
            if success.status().is_success() {
                debug_assert_eq!(success.version(), Version::HTTP_2);
                debug!(" 成功: {}", serde_json::to_string(&json_body).unwrap());
            } else {
                warn!(
                    "更新:{},类型:{}时服务器返回码:{}",
                    json_body.name,
                    json_body.record_type,
                    success.status().as_u16()
                );
                return Err(());
            }
        }
        Err(error) => {
            debug!("{error}");
            if error.is_timeout() {
                warn!(
                    "更新:{},类型:{}时链接超时",
                    json_body.name, json_body.record_type
                );
            } else if error.is_connect() {
                warn!(
                    "更新:{},类型:{}时链接错误",
                    json_body.name, json_body.record_type
                );
            } else {
                warn!(
                    "更新{}类型{}时发生未知错误:{}",
                    json_body.name, json_body.record_type, error
                );
            }
            return Err(());
        }
    };
    Ok(())
}

pub async fn update_ip(ip_version: u8, config_json: Vec<&crate::load_conf::DnsRecord>) {
    if config_json.is_empty() {
        debug!("没有需要更新的IPv{ip_version}记录");
        return;
    }

    let ip = match get_ip(ip_version).await {
        Ok(success) => {
            debug!("获取成功，当前IPv{ip_version}地址为：{success}");
            success
        }
        Err(_) => return,
    };

    // 检查IP是否变化
    match ip {
        IpAddr::V4(ipv4) => {
            let mut ipv4_inner = IPV4ADDR.lock();
            if ipv4 == *ipv4_inner {
                debug!("IPv4地址未改变，跳过更新");
                return;
            } else {
                *ipv4_inner = ipv4;
            }
        }
        IpAddr::V6(ipv6) => {
            let mut ipv6_inner = IPV6ADDR.lock();
            if ipv6 == *ipv6_inner {
                debug!("IPv6地址未改变，跳过更新");
                return;
            } else {
                *ipv6_inner = ipv6;
            }
        }
    }
    let mut task_set = tokio::task::JoinSet::new();

    for i in config_json {
        task_set.spawn(ask_api(ip, i.clone()));
    }
    let _a = task_set.join_all().await;
}
