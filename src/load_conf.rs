use log::{debug, error};
use std::{fs, io};

#[derive(Debug, serde::Deserialize)]
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

pub fn init_conf() -> Result<Vec<DnsRecord>, String> {
    let config_file = match fs::File::open(crate::DATA_DIR.join("config.json")) {
        Ok(success) => success,
        Err(_) => {
            error!("找不到 config.json");
            return Err("找不到 config.json".to_string());
        }
    };

    match serde_json::from_reader(io::BufReader::new(config_file)) {
        Ok(success) => {
            debug!("成功解析配置文件");
            return Ok(success);
        }
        Err(error) => {
            error!("config.json 格式不正确：\n {}", error);
            return Err("config.json 格式不正确".to_string());
        }
    };
}
