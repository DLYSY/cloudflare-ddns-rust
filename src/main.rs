use reqwest::{self, Client, ClientBuilder};
use futures::future::join_all;
use tokio;
use std::sync::LazyLock;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::u8;
use std::fs::read_to_string;
use std::time::Duration;
use json::{self, JsonValue};


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
                    success
                }else{
                    println!("api请求失败服务器返回{}",success.status().as_u16());
                    return Err(());
                }
            },
            Err(error) =>{
                if error.is_timeout(){
                    println!("更新{}类型{}时链接超时", info["name"], info["type"]);
                }else if error.is_connect() {
                    println!("更新{}类型{}时链接错误", info["name"], info["type"]);
                }else {
                    println!("更新{}类型{}时发生未知错误:{}", info["name"], info["type"], error);
                }
                return Err(());
            }
        };
    Ok(())
}

async fn update_ip(ip_version: u8, global_config_json: &JsonValue){
    let ip = match get_ip(ip_version).await {
        Ok(success) => success,
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
    let url = format!("https://ipv{ip_version}.icanhazip.com/");
    let ip_response = match CLIENT.get(url).send().await{
        Ok(success) => {
            if success.status().is_success(){
                success
            }else {
                println!("获取ipv{ip_version}时状态码不正确{}",success.status().as_u16());
                return Err(());
            }
        },
        Err(error) => {
            if error.is_timeout(){
                println!("获取ipv{ip_version}时链接超时")
            }else if error.is_connect() {
                println!("获取ipv{ip_version}时链接错误")
            }else {
                println!("获取ipv{ip_version}时发生未定义错误{}",error)
            }
            return Err(());
        }
    };
    
    let ip_text = match ip_response.text().await{
        Ok(success) => success,
        Err(error) => {
            println!("获取相应正文时发生错误：{error}");
            return Err(());
        }
    };

    let _ = match ip_version {
        4 => {
            match Ipv4Addr::from_str(ip_text.trim()) {
                Ok(ip) => return Ok(IpAddr::V4(ip)),
                Err(_) => {
                    println!("获取到格式不正确的ipv4");
                    return Err(());
                }
            }
        },
        6 => {
            match Ipv6Addr::from_str(ip_text.trim()) {
                Ok(ip) => return Ok(IpAddr::V6(ip)),
                Err(_) => {
                    println!("获取到格式不正确的ipv6");
                    return Err(());
                }
            }
        },
        _ => panic!("程序内部错误：get_ip函数获取到不可能的值{ip_version}")
    };
}

fn main() {

    let config_json_text = match read_to_string("A:/code myself/cloudflare-ddns-rust/config.json"){
        Ok(success) => success,
        Err(_) => {
            println!("读取失败,请检查config.json是否存在并使用UTF-8编码");
            return;
        }
    };
    let config_json = match json::parse(config_json_text.as_str()){
        Ok(success) => success,
        Err(_) => {
            println!("json文件格式不正确");
            return;
        }
    };

    // println!("{json_data}");
    let mut have_ipv4 = false;
    let mut have_ipv6 = false;

    for i in config_json.members(){
        if i["type"] == "A"{
            have_ipv4 = true;
        }else if i["type"] == "AAAA"{
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
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(join_all(update_tasks));
}