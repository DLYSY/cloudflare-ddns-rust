# 命令行参考

运行 `ddns_rust --help` 可以获取帮助：

```bash
Usage: ddns_rust <COMMAND>

Commands:
  run        Run the application
  install    Install components
  uninstall  Uninstall components
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

每个子命令也都可以使用 `--help` 来查看参数。

## 更新DNS

要更新DNS，请使用 

```bash
ddns_rust run
```

这将会读取在二进制文件同目录下的 `data/config.json` 作为配置文件，并且将日志存放于 `data/logs/`。

如果不带任何参数运行，那么程序将会在运行一次后退出，这与使用 `ddns_rust run --once` 的行为是相同的。

### 指定日志等级

默认无参数情况下，日志的输出等级为 `info` 如果您需要指定日志等级，可以使用 `--log` 参数。

例如您需要使用 `debug` 日志：
```bash
ddns_rust run --log debug
```

日志共有6个等级，从详细到简略排名：
1. trace
   - 最详细的等级，还会输出很多 crate 日志，比如 reqwest 的 retry 策略等
2. debug
   - 推荐用于一般调试，如果您需要提交 issue，请使用 `debug`
3. info
   - 默认的等级
4. warn
   - 当程序发生非崩溃错误时，将会使用 `warn`
5. error
   - 当程序发生无法继续运行的错误时，将会抛出 `error` 并退出程序
6. off
   - 关闭日志，不会产生任何输出

### 循环运行与单次运行



## 安装

可以将二进制文件安装为服务或者定时任务

## 卸载