use reqwest::{self, Client, ClientBuilder};
use futures::try_join;
use tokio;
use std::sync::LazyLock;
use std::net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use anyhow::{Result, anyhow};


static CLIENT: LazyLock<Client> = LazyLock::new(||{
    ClientBuilder::new()
    .no_proxy()
    .gzip(true)
    .build()
    .unwrap()
});

async fn ask_api(ip: IpAddr){
    println!("{}",ip);
    // let api_url = "https://api.cloudflare.com/client/v4/zones/<zone_id>/dns_records/dns_id";
    // let api_request = CLIENT.post(api_url)
    //     .body("body")
    //     .header("Authorization", "Bearer");
}

async fn update_ipv4()-> Result<u16>{
    let ipv4_response = CLIENT
        .get("https://ipv6.icanhazip.com/")
        // .get("https://httpbin.org/status/403")
        .send()
        .await?;
    if !ipv4_response.status().is_success() {
        return Ok(ipv4_response.status().as_u16());
    }
    let ipv4_addr = Ipv4Addr::from_str(
        ipv4_response
            .text()
            .await?
            .trim()
    )?;
    ask_api(IpAddr::V4(ipv4_addr)).await;
    Ok(0)
}

async fn get_ip() -> Result<u8, reqwest::Error> {
    let client = Client::new();
    
    // 并行发起两个请求
    let ip4_fut = client.get("https://ipv4.icanhazip.com/").send();
    let ip6_fut = client.get("https://ipv6.icanhazip.com/").send();
    
    // 同时等待两个请求完成
    let (ip4_res, ip6_res) = try_join!(ip4_fut, ip6_fut)?;
    
    // 并行读取响应内容
    let text4_fut = ip4_res.text();
    let text6_fut = ip6_res.text();
    
    // 同时等待两个响应解析
    let (ip4, ip6) = try_join!(text4_fut, text6_fut)?;
    
    let ipv4 = Ipv4Addr::from_str(ip4.trim());

    println!("IPv4: {}", ipv4.unwrap());
    println!("IPv6: {}", ip6.trim());
    Ok(0)
}

// fn handle_error(error: anyhow::Error){
//     if let Some(app_error) = error.downcast_ref() {
//         match app_error {
//             reqwest::Error => println!("network error"),
//             _ => {}
//         }
//     }
// }

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let a = rt.block_on(update_ipv4());
    let a = match a {
        Ok(s) =>{
            match s {
                0 => println!("success"),
                100..999 => eprintln!("status code {} error",s),
                _ => {}
            }
        }

        Err(e) => {
            // println!("{}",err_res);
            if let Some(reqwest_error) = e.downcast_ref::<reqwest::Error>(){
                if reqwest_error.is_timeout(){
                    eprintln!("链接超时");
                }else if reqwest_error.is_connect() {
                    eprintln!("链接错误");
                }
            }else if let Some(ip_error) = e.downcast_ref::<AddrParseError>(){
                eprintln!("获取到的IP不正确{}",ip_error);
            }
        }
    // println!("end");
    };
}