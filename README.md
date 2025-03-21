# cloudflare-ddns-rust

与二进制文件同目录创建`config.json`，结构如下：

```json
[
    {
        "api_token":"<cloudflare api token，要求具有DNS操作权限>",
        "zone_id":"<域名zone id>",
        "dns_id": "<获取方式见下文>",
        "type": "<A or AAAA，其他不支持>",
        "name": "<www.example.com>",
        "ttl": <int>,
        "proxied": <bool>
    },
    #<可以添加更多的记录，同字段配置方法类似>
    {
        "api_token":"",
        "zone_id":"",
        "dns_id": "",
        "type": "",
        "name": "example.com",
        "ttl": 60,
        "proxied": false
    }
]
```
要获取dns id，可以执行

```bash
curl https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records \
    -H "Authorization: Bearer $API_TOKEN"
```

然后运行`<yourpath>/ddns_rust<.exe>`即可，会产生日志存放于`./logs`下。

如果需要服务，可以参考systemd-timer

配置ddns.service
```toml
[Unit]
Description = Cloudflare DDNS

[Service]
Type = oneshot
ExecStart = <yourpath>/ddns_rust<.exe>

```

配置ddns.timer
```toml
[Unit]
Description = Cloudflare DDNS

[Timer]
OnStartupSec = 1m
OnUnitActiveSec = 90s # 自定义轮询时间

[Install]
WantedBy = timers.target
```
然后将这俩个文件放入`/etc/systemd/system`，执行`systemctl enable --now ddns.timer`。