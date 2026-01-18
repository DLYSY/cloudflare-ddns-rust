use clap::Parser;
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming, WriteMode,
    colored_detailed_format, detailed_format,
};
use log::debug;
use std::env::{current_dir, current_exe};
use std::sync::LazyLock;

use crate::initialize::parse_args;

pub static ARGS: LazyLock<parse_args::Commands> =
    LazyLock::new(|| parse_args::CliArgs::parse().command);

pub static DATA_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    let parse_args::Commands::Run { loops: _, datadir } = &*ARGS else {
        unreachable!()
    };

    datadir.clone().unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            current_dir().expect("无法读取当前工作目录")
        } else {
            current_exe()
                .expect("无法读取二进制文件路径")
                .parent()
                .expect("无法读取二进制文件所在目录")
                .to_path_buf()
        }
        .join("data")
    })
});

pub fn init_log(log_level: &String) -> Result<LoggerHandle, String> {
    let logger = Logger::try_with_str(log_level)
        .map_err(|e| format!("log 等级格式错误，参考 https://docs.rs/flexi_logger/latest/flexi_logger/struct.LogSpecification.html \n{e}"))?
        .log_to_file(
            FileSpec::default()
                .directory(DATA_DIR.join("logs")) //定义日志文件位置
                .basename("ddns"),
        ) //定义日志文件名，不包含后缀
        .duplicate_to_stdout(Duplicate::Trace) //复制日志到控制台
        .rotate(
            Criterion::Age(Age::Day), // 按天轮转
            Naming::TimestampsCustomFormat {
                current_infix: None,
                format: "%Y-%m-%d",
            }, // 文件名包含日期并以天为单位轮换
            Cleanup::KeepCompressedFiles(15), // 保留15天日志并启用压缩
        )
        .format_for_stdout(colored_detailed_format) //控制台输出彩色带时间的日志格式
        .format_for_files(detailed_format) //文件中使用ANSI颜色会乱码，所以使用无颜色格式
        .write_mode(WriteMode::Async)
        .append() //指定日志文件为添加内容而不是覆盖重写
        .start()
        .map_err(|e| format!("无法创建logger句柄,回溯错误:\n{e}"))?;
    debug!("日志初始化成功");
    Ok(logger)
}
