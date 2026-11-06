use clap::Parser;
use std::path::PathBuf;

/// 在 Windows 上复制目录树时保留 NTFS 目录联接（Junction）与符号链接。
#[derive(Debug, Parser)]
#[command(
    name = "linkcopy",
    version,
    about = "在 Windows 上复制目录树时保留 NTFS 目录联接（Junction）与符号链接",
    long_about = None
)]
pub struct Args {
    /// 源目录
    pub src: PathBuf,

    /// 目标目录（不会删除其中已有多余的文件）
    pub dst: PathBuf,

    /// 从文件读取排除规则，每行一条，# 开头为注释
    #[arg(long, value_name = "FILE")]
    pub exclude_from: Option<PathBuf>,

    /// 只打印计划，不写入任何内容
    #[arg(long)]
    pub dry_run: bool,

    /// 覆盖目标端已存在的同名文件或链接
    #[arg(long)]
    pub force: bool,

    /// 逐项输出处理过程
    #[arg(long)]
    pub verbose: bool,

    /// 只输出错误与最终摘要
    #[arg(long)]
    pub quiet: bool,
}
