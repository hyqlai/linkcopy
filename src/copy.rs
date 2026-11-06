use crate::cli::Args;
use crate::exclude::ExcludeList;
use crate::links;
use crate::path_guard;
use anyhow::{Context, Result, bail};
use std::fs::{self, File, FileTimes, Metadata};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Stats {
    pub files: u64,
    pub dirs: u64,
    pub links: u64,
    pub skipped: u64,
    pub errors: u64,
}

enum Existing {
    Absent,
    ReuseDir,
    Skip,
    Replace,
}

struct Runner {
    dst_root: PathBuf,
    excludes: ExcludeList,
    dry_run: bool,
    force: bool,
    verbose: bool,
    quiet: bool,
    stats: Stats,
}

pub fn run(args: &Args) -> Result<Stats> {
    let src = path_guard::absolute(&args.src)?;
    let dst = path_guard::absolute(&args.dst)?;
    if !src.is_dir() {
        bail!("源目录不存在或不是目录：{}", src.display());
    }
    // 目标落在源内部会无限递归；源落在目标内部会把内容写进自己的上级目录。
    path_guard::ensure_disjoint(&src, &dst)?;

    let mut runner = Runner {
        dst_root: dst.clone(),
        excludes: ExcludeList::load(args.exclude_from.as_deref())?,
        dry_run: args.dry_run,
        force: args.force,
        verbose: args.verbose,
        quiet: args.quiet,
        stats: Stats::default(),
    };
    runner.ensure_dir(&dst, false)?;
    runner.walk(&src, &dst, Path::new(""))?;
    Ok(runner.stats)
}

impl Runner {
    fn ensure_dir(&mut self, dst: &Path, reuse: bool) -> Result<()> {
        if reuse {
            return Ok(());
        }
        match fs::symlink_metadata(dst) {
            Ok(meta) if meta.is_dir() => return Ok(()),
            Ok(_) => bail!("目标路径已存在且不是目录：{}", dst.display()),
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err).with_context(|| format!("无法检查目标路径 {}", dst.display()));
            }
        }
        if !self.dry_run {
            fs::create_dir_all(dst).with_context(|| format!("无法创建目录 {}", dst.display()))?;
        }
        self.stats.dirs += 1;
        self.log(&format!("目录 {}", dst.display()));
        Ok(())
    }

    fn walk(&mut self, src_dir: &Path, dst_dir: &Path, rel: &Path) -> Result<()> {
        let entries = match fs::read_dir(src_dir) {
            Ok(entries) => entries,
            Err(err) => {
                self.report_error(src_dir, &anyhow::Error::from(err));
                return Ok(());
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    self.report_error(src_dir, &anyhow::Error::from(err));
                    continue;
                }
            };
            let name = entry.file_name();
            let child_rel = rel.join(&name);
            if self.excludes.is_excluded(&child_rel) {
                self.stats.skipped += 1;
                self.log(&format!("排除 {}", child_rel.display()));
                continue;
            }
            let src_path = src_dir.join(&name);
            let dst_path = dst_dir.join(&name);
            // 名称来自 read_dir，仍校验一次边界，防止后续扩展引入路径穿越。
            if let Err(err) = path_guard::ensure_within(&self.dst_root, &dst_path) {
                self.report_error(&src_path, &err);
                continue;
            }
            if let Err(err) = self.copy_entry(&src_path, &dst_path, &child_rel) {
                self.report_error(&src_path, &err);
            }
        }
        Ok(())
    }

    fn copy_entry(&mut self, src: &Path, dst: &Path, rel: &Path) -> Result<()> {
        let link_kind = links::classify(src)?;
        let meta = fs::symlink_metadata(src)
            .with_context(|| format!("无法读取元数据 {}", src.display()))?;
        let src_is_dir = link_kind.is_none() && meta.is_dir();

        let existing = self.check_existing(dst, src_is_dir)?;
        if matches!(existing, Existing::Skip) {
            self.stats.skipped += 1;
            self.log(&format!("跳过（已存在） {}", rel.display()));
            return Ok(());
        }

        match link_kind {
            Some(kind) => {
                // 重建链接只依赖目标字符串，不读取目标目录内容，也不跟随链接递归。
                let target = links::read_target(src, kind)?;
                if !self.dry_run {
                    links::recreate(&target, dst, kind)?;
                }
                self.stats.links += 1;
                self.log(&format!("链接 {} -> {}", rel.display(), target.display()));
            }
            None if src_is_dir => {
                self.ensure_dir(dst, matches!(existing, Existing::ReuseDir))?;
                self.walk(src, dst, rel)?;
            }
            None if meta.is_file() => {
                if !self.dry_run {
                    copy_file(src, dst, &meta)?;
                }
                self.stats.files += 1;
                self.log(&format!("文件 {}", rel.display()));
            }
            None => bail!("不支持的特殊文件类型"),
        }
        Ok(())
    }

    fn check_existing(&self, dst: &Path, src_is_dir: bool) -> Result<Existing> {
        let meta = match fs::symlink_metadata(dst) {
            Ok(meta) => meta,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Existing::Absent),
            Err(err) => return Err(err).with_context(|| format!("无法检查目标 {}", dst.display())),
        };
        let dst_link = links::classify(dst)?;
        if src_is_dir && meta.is_dir() && dst_link.is_none() {
            return Ok(Existing::ReuseDir);
        }
        if !self.force {
            return Ok(Existing::Skip);
        }
        if meta.is_dir() && dst_link.is_none() {
            // 拒绝删除实体目录，避免 --force 造成不可恢复的数据丢失。
            bail!("目标端是实体目录，拒绝删除：{}", dst.display());
        }
        if !self.dry_run {
            if let Some(kind) = dst_link {
                links::remove_link(dst, kind)?;
            } else {
                fs::remove_file(dst).with_context(|| format!("无法覆盖 {}", dst.display()))?;
            }
        }
        Ok(Existing::Replace)
    }

    fn report_error(&mut self, path: &Path, err: &anyhow::Error) {
        self.stats.errors += 1;
        eprintln!("错误 {}: {err:#}", path.display());
    }

    fn log(&self, message: &str) {
        if !self.quiet && (self.verbose || self.dry_run) {
            println!("{message}");
        }
    }
}

fn copy_file(src: &Path, dst: &Path, meta: &Metadata) -> Result<()> {
    fs::copy(src, dst)
        .with_context(|| format!("复制文件失败 {} -> {}", src.display(), dst.display()))?;
    // 时间戳不影响链接语义，设置失败不应让整次复制失败。
    if let Ok(modified) = meta.modified() {
        let times = FileTimes::new().set_modified(modified);
        if let Ok(file) = File::options().write(true).open(dst) {
            let _ = file.set_times(times);
        }
    }
    Ok(())
}
