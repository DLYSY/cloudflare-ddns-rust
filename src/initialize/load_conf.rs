use crate::{obj::DATA_DIR, parse_args::LogLevel};
use log::{debug, error};
use std::{fs, io};
use std::sync::OnceLock;

pub static CONFIG_JSON: OnceLock<Config> = OnceLock::new();

#[derive(Debug, serde::Deserialize, Clone)]
pub struct DnsRecord {
    pub api_token: String,
    pub zone_id: String,
    pub dns_id: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub ttl: u32,
    pub proxied: bool,
}

fn get_default_delay() -> u64 {
    60
}
fn get_default_mutli_thread() -> bool {
    false
}
fn get_default_log_level() -> LogLevel {
    if cfg!(debug_assertions) {
        LogLevel::Debug
    } else {
        LogLevel::Info
    }
}
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "get_default_delay")]
    pub delay: u64,
    #[serde(default = "get_default_mutli_thread")]
    pub mutli_thread: bool,
    #[serde(default = "get_default_log_level")]
    pub log_level: LogLevel,
    pub dns_records: Vec<DnsRecord>,
}

impl Config {
    pub fn init() -> Result<(), String> {
        let config_file = match fs::File::open(DATA_DIR.join("config.json")) {
            Ok(success) => success,
            Err(_) => {
                error!("找不到 config.json");
                return Err("找不到 config.json".to_string());
            }
        };

        let config_json = match serde_json::from_reader(io::BufReader::new(config_file)) {
            Ok(success) => {
                debug!("成功解析配置文件");
                success
            }
            Err(error) => {
                let error = format!("config.json 格式不正确 | {}", error);
                error!("{}", error);
                return Err(error);
            }
        };
        CONFIG_JSON.set(config_json).unwrap();
        Ok(())
    }
}
