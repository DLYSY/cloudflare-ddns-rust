# 二进制文件安装

## 获取二进制文件

对于 windows 和 linux，可以直接从 [CI/CD 页面](https://github.com/DLYSY/cloudflare-ddns-rust/actions) 下载编译好的二进制文件。

如果你正在使用其他操作系统，请查看 [编译](install/build.md)。

## 在 Windows 上安装

运行：
```bash
ddns_rust install service
```

## 在 Linux 上安装

对于大多数使用 systemd 的 Linux 发行版：
  
```bash
ddns_rust install service

systemctl enable cloudflareddns.service
```

## 下一步

脚本在没有配置之前是无法使用的，请查看 [配置DNS](config.md)