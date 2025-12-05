# Docker 安装

本项目支持标准的 Linux 容器与 Windows 容器。

## Linux

先[配置](config.md)好 config.json，然后运行：

```bash
mkdir /srv/cloudflare-ddns-rust

# 在 /srv/cloudflare-ddns-rust 下配置  config.json

docker run -d \
-v /srv/cloudflare-ddns-rust/:/app/data \
--network=host \
--restart=always \
--name=cloudflareddns \
quay.io/dlysy/cloudflareddns:linux
```

## Windows

由于 Windows 容器较为少用且与 Linux 容器存在差异，具体使用请参考[微软官方文档](https://learn.microsoft.com/zh-cn/virtualization/windowscontainers/quick-start/set-up-environment?tabs=dockerce)。

容器镜像是基于 `nanoserver:ltsc2022` 所以根据[兼容性文档](https://learn.microsoft.com/zh-cn/virtualization/windowscontainers/deploy-containers/version-compatibility?tabs=windows-server-2025%2Cwindows-11)，理论上可以兼容 Server 2022 与 2025。

先[配置](config.md)好 config.json，然后运行：

```powershell
mkdir D:\cloudflare-ddns-rust

# 在 /srv/cloudflare-ddns-rust 下配置  config.json

docker run -d -v D:\cloudflare-ddns-rust:C:\app\data --restart=always --name=cloudflareddns quay.io/dlysy/cloudflareddns:windows
```