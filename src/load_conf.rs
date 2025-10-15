use log::{debug, error};
use std::{fs, io};

#[derive(Debug, serde::Deserialize,serde::Serialize, Clone)]
pub struct DnsRecord {
    pub api_token: Option<String>,
    pub zone_id: Option<String>,
    pub dns_id: Option<String>,
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub ttl: u32,
    pub proxied: bool,
    pub content: Option<String>,
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
            Err(_) => {
                error!("config.json 格式不正确");
                return Err("config.json 格式不正确".to_string());
            }
        };
}
