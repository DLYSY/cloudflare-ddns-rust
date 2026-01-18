use crate::obj::DATA_DIR;
use log::{debug, error, trace};
use std::sync::OnceLock;
use std::{fs, io};

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, serde::Deserialize, Clone, Copy, PartialEq)]
pub enum RecordType {
    A,
    AAAA,
}
impl RecordType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordType::A => "A",
            RecordType::AAAA => "AAAA",
        }
    }
    pub fn as_u8(&self) -> u8 {
        match self {
            RecordType::A => 4,
            RecordType::AAAA => 6,
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct DnsRecord {
    pub api_token: String,
    pub zone_id: String,
    pub dns_id: String,
    #[serde(rename = "type")]
    pub record_type: RecordType,
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
fn get_default_log_level() -> String {
    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "info".to_string()
    }
}
fn get_default_ipv4_url() -> url::Url {
    url::Url::parse("https://ipv4.icanhazip.com/").unwrap()
}
fn get_default_ipv6_url() -> url::Url {
    url::Url::parse("https://ipv6.icanhazip.com/").unwrap()
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct Config {
    #[serde(default = "get_default_delay")]
    pub delay: u64,
    #[serde(default = "get_default_mutli_thread")]
    pub mutli_thread: bool,
    #[serde(default = "get_default_log_level")]
    pub log_level: String,
    #[serde(default = "get_default_ipv4_url")]
    pub ipv4_url: url::Url,
    #[serde(default = "get_default_ipv6_url")]
    pub ipv6_url: url::Url,
    pub dns_records: Vec<DnsRecord>,
}

enum ConfigFile {
    Json(fs::File),
    Toml(Vec<u8>),
}
impl ConfigFile {
    fn parse(self) -> Result<Config, String> {
        match self {
            ConfigFile::Json(f) => serde_json::from_reader(io::BufReader::new(f))
                .map_err(|e| format!("config.json 格式不正确 | {}", e)),
            ConfigFile::Toml(f) => toml::from_slice::<Config>(&f)
                .map_err(|e| format!("config.toml 格式不正确 | {}", e)),
        }
    }
}

impl Config {
    pub fn init() -> Result<(), String> {
        let config_file = if DATA_DIR.join("config.toml").exists() {
            trace!("找到 config.toml");
            ConfigFile::Toml(fs::read(DATA_DIR.join("config.toml")).unwrap())
        } else if DATA_DIR.join("config.json").exists() {
            trace!("找到 config.json");
            ConfigFile::Json(fs::File::open(DATA_DIR.join("config.json")).unwrap())
        } else {
            error!("找不到 config.toml 或 config.json");
            return Err("找不到 config.toml 或 config.json".to_string());
        };

        match config_file.parse() {
            Ok(config) => {
                debug!("成功解析配置文件");
                CONFIG.set(config).expect("Config should only be set once");
            }
            Err(e) => {
                error!("{}", e);
                return Err(e);
            }
        }
        Ok(())
    }
}
