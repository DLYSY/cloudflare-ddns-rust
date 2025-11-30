# 二进制文件安装

## 获取二进制文件

对于 windows 和 linux，可以直接从 [CI/CD 页面](https://github.com/DLYSY/cloudflare-ddns-rust/actions) 下载编译好的二进制文件。

如果你正在使用其他操作系统，请 [编译](install/build.md)。

## 安装为服务

运行：
```bash
ddns_rust install service
```

- 对于 Windows，该命令会创建名为 `Cloudflare DDNS` 的服务，启动方式为“自动（延迟启动）”。

- 如果您正在使用 systemd 作为 init 的 Linux 发行版，该命令会在`/etc/systemd/system/`下创建`cloudflareddns.service`，您需要使用如下命令来启用它：
```bash
systemctl enable --now cloudflareddns.service
```

!> 如果您正在使用其他非 Systemd 的类 Unix 系统，这条命令依然会尝试创建 `/etc/systemd/system/cloudflareddns.service`，但您可能无法使用该服务。

?> 服务的循环周期为1分钟 （暂不支持调整），当 ip 没有改变时**且作为服务运行时**，将不会请求 Cloudflare API，因此可以不需要担心过度轮训超出 API 使用限制。

## 安装为定时任务