use reqwest::{self, Client, ClientBuilder};
use futures::future::join_all;
use tokio;
use std::sync::LazyLock;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::env::current_exe;
use std::fs::read_to_string;
use std::time::Duration;
use json::{self, JsonValue};
use flexi_logger::{Logger, Criterion, Naming, Cleanup, Age, Duplicate, FileSpec, WriteMode, colored_detailed_format,detailed_format};
use log::{debug, error, info, warn, trace};

// schtasks /create /tn test /sc MINUTE /mo 2 /tr a:\test.bat /ru System

static CLIENT: LazyLock<Client> = LazyLock::new(||{
    ClientBuilder::new()
    .no_proxy()
    .https_only(true)
    .gzip(true)
    .connect_timeout(Duration::new(5, 0))
    .read_timeout(Duration::new(5, 0))
    .build()
    .unwrap()
});

async fn ask_api(ip: IpAddr, info: &mut JsonValue) -> Result<(),()> {
    let api_url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",info.remove("zone_id"), info.remove("dns_id"));
    let header = format!("Bearer {}", info.remove("api_token"));
    info["content"] = ip.to_string().into();
    match CLIENT.put(api_url)
        .body(info.dump())
        .header("Content-Type", "application/json")
        .header("Authorization", header)
        .send().await{
            Ok(success) => {
                if success.status().is_success(){
                    debug!("更新: {}, 类型: {}成功", info["name"], info["type"]);
                    // success
                }else{
                    warn!("更新:{},类型:{}时服务器返回码:{}", info["name"], info["type"], success.status().as_u16());
                    return Err(());
                }
            },
            Err(error) =>{
                debug!("{error}");
                if error.is_timeout(){
                    warn!("更新:{},类型:{}时链接超时", info["name"], info["type"]);
                }else if error.is_connect() {
                    warn!("更新:{},类型:{}时链接错误", info["name"], info["type"]);
                }else {
                    warn!("更新{}类型{}时发生未知错误:{}", info["name"], info["type"], error);
                }
                return Err(());
            }
        };
    Ok(())
}

async fn update_ip(ip_version: u8, global_config_json: &JsonValue){
    let ip = match get_ip(ip_version).await {
        Ok(success) => {
            debug!("获取IPv{ip_version}成功");
            success
        },
        Err(_) => return
    };

    let mut self_config_json = global_config_json.clone();
    let mut ask_api_list = Vec::new();
    for i in self_config_json.members_mut(){
        if i["type"]=="A" && ip_version==4{
            ask_api_list.push(ask_api(ip, i));
        }else if i["type"]=="AAAA" &&ip_version==6{
            ask_api_list.push(ask_api(ip, i));
        }
    }
    join_all(ask_api_list).await;
}

async fn get_ip(ip_version: u8) -> Result<IpAddr, ()> {
    // let url = format!("https://{ip_version}.ipw.cn/");
    let url = format!("https://ipv{ip_version}.icanhazip.com/");
    let ip_response = match CLIENT.get(url).send().await{
        Ok(success) => {
            if success.status().is_success(){
                success
            }else {
                warn!("获取ipv{ip_version}时状态码不正确{}",success.status().as_u16());
                return Err(());
            }
        },
        Err(error) => {
            if error.is_timeout(){
                warn!("获取ipv{ip_version}时链接超时")
            }else if error.is_connect() {
                warn!("获取ipv{ip_version}时链接错误")
            }else {
                warn!("获取ipv{ip_version}时发生未定义错误{}",error)
            }
            return Err(());
        }
    };
    
    let ip_text = match ip_response.text().await{
        Ok(success) => success,
        Err(error) => {
            warn!("获取IPv{ip_version}时响应正文时发生错误：{error}");
            return Err(());
        }
    };

    match ip_version {
        4 => {
            match Ipv4Addr::from_str(ip_text.trim()) {
                Ok(ip) => return Ok(IpAddr::V4(ip)),
                Err(_) => {
                    warn!("获取到格式不正确的ipv4");
                    return Err(());
                }
            }
        },
        6 => {
            match Ipv6Addr::from_str(ip_text.trim()) {
                Ok(ip) => return Ok(IpAddr::V6(ip)),
                Err(_) => {
                    warn!("获取到格式不正确的ipv6");
                    return Err(());
                }
            }
        },
        _ => {
            error!("程序内部错误：get_ip函数获取到不可能的值{ip_version}");
            panic!();
        }
    }
}

fn main() {
    let mut config_json_path = match current_exe() {
        Ok(success) => success,
        Err(_) => {
            error!("文件系统错误！无法读取config.json");
            return;
        }
    };
    config_json_path.pop();
    let mut log_path = config_json_path.clone();
    config_json_path.push("config.json");
    log_path.push("logs");

    let _logger = Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default()
            .directory(log_path) //定义日志文件位置
            .basename("ddns")) //定义日志文件名，不包含后缀
        .duplicate_to_stdout(Duplicate::Trace) //复制日志到控制台
        .rotate(
            Criterion::Age(Age::Day), // 按天轮转
            Naming::TimestampsCustomFormat {
                current_infix: None,
                format: "%Y-%m-%d"
            },       // 文件名包含日期并以天为单位轮换
            Cleanup::KeepCompressedFiles(15), // 保留15天日志并启用压缩
        )
        .format_for_stdout(colored_detailed_format) //控制台输出彩色带时间的日志格式
        .format_for_files(detailed_format) //文件中使用ANSI颜色会乱码，所以使用无颜色格式
        .write_mode(WriteMode::Async)
        .append() //指定日志文件为添加内容而不是覆盖重写
        .start()
        .unwrap();



    let config_json_text = match read_to_string(config_json_path){
        Ok(success) => {
            trace!("成功读取配置文件");
            success
        },
        Err(_) => {
            error!("读取失败,请检查config.json是否存在并使用UTF-8编码");
            return;
        }
    };
    let config_json = match json::parse(config_json_text.as_str()){
        Ok(success) => {
            trace!("成功解析json");
            success
        },
        Err(_) => {
            error!("json文件格式不正确");
            return;
        }
    };

    // println!("{json_data}");
    let mut have_ipv4 = false;
    let mut have_ipv6 = false;

    for i in config_json.members(){
        if i["type"] == "A"{
            debug!("找到A记录需要解析");
            have_ipv4 = true;
        }else if i["type"] == "AAAA"{
            debug!("找到AAAA记录需要解析");
            have_ipv6 = true;
        }
        if have_ipv4 && have_ipv6{
            break;
        }
    }

    let mut update_tasks = Vec::new();
    if have_ipv4{
        update_tasks.push(update_ip(4, &config_json));
    }
    if have_ipv6{
        update_tasks.push(update_ip(6, &config_json));
    } 
    
    let rt = match tokio::runtime::Runtime::new() {
        Ok(success) => {
            trace!("成功创建异步运行时");
            success
        },
        Err(error) => {
            error!("无法创建异步运行时,回溯错误:{error}");
            return;
        }
    };
    rt.block_on(join_all(update_tasks));
    info!("done");
}