use clap::Parser;
use linkcopy::{cli::Args, copy};

fn main() {
    let args = Args::parse();
    match copy::run(&args) {
        Ok(stats) => {
            if !args.quiet {
                println!(
                    "完成：文件 {}，目录 {}，链接 {}，跳过 {}，错误 {}{}",
                    stats.files,
                    stats.dirs,
                    stats.links,
                    stats.skipped,
                    stats.errors,
                    if args.dry_run {
                        "（预览，未写入）"
                    } else {
                        ""
                    }
                );
            }
            if stats.errors > 0 {
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("错误：{err:#}");
            std::process::exit(2);
        }
    }
}
