#![cfg(windows)]

use linkcopy::cli::Args;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn args(src: PathBuf, dst: PathBuf) -> Args {
    Args {
        src,
        dst,
        exclude_from: None,
        dry_run: false,
        force: false,
        verbose: false,
        quiet: true,
    }
}

struct Fixture {
    _tmp: TempDir,
    src: PathBuf,
    dst: PathBuf,
    target: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = root.join("src");
        let target = src.join("target");
        fs::create_dir_all(src.join(".agents/skills")).unwrap();
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("hello.txt"), b"hi").unwrap();
        fs::write(src.join("normal.txt"), b"normal").unwrap();
        fs::write(src.join("sub/inner.txt"), b"inner").unwrap();
        fs::write(src.join("excluded.txt"), b"skip").unwrap();

        for name in ["lark-one", "lark-two", "lark-skip"] {
            junction::create(&target, src.join(".agents/skills").join(name)).unwrap();
        }

        Self {
            dst: root.join("dst-copy"),
            _tmp: tmp,
            src,
            target,
        }
    }

    fn skills(&self, root: &Path) -> PathBuf {
        root.join(".agents/skills")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // 先删链接本身再让 TempDir 清理目录，避免递归删除跟随链接。
        for root in [&self.src, &self.dst] {
            if let Ok(entries) = fs::read_dir(self.skills(root)) {
                for entry in entries.flatten() {
                    if junction::exists(entry.path()).unwrap_or(false) {
                        let _ = fs::remove_dir(entry.path());
                    }
                }
            }
        }
    }
}

#[test]
fn preserves_junctions_and_copies_plain_files() {
    let f = Fixture::new();
    let stats = linkcopy::copy::run(&args(f.src.clone(), f.dst.clone())).unwrap();

    assert_eq!(stats.errors, 0, "不应有错误");
    assert_eq!(stats.links, 3, "三个 Junction 都应重建");
    assert_eq!(fs::read(f.dst.join("normal.txt")).unwrap(), b"normal");
    assert_eq!(fs::read(f.dst.join("sub/inner.txt")).unwrap(), b"inner");

    let copied = f.skills(&f.dst).join("lark-one");
    assert!(
        junction::exists(&copied).unwrap(),
        "复制后必须仍是 Junction"
    );
    assert_eq!(junction::get_target(&copied).unwrap(), f.target);
    assert_eq!(fs::read(copied.join("hello.txt")).unwrap(), b"hi");
    assert!(junction::exists(f.skills(&f.src).join("lark-one")).unwrap());
}

#[test]
fn exclude_from_file_skips_matching_entries() {
    let f = Fixture::new();
    let rules = f._tmp.path().join("exclude.txt");
    fs::write(&rules, "# 注释\nexcluded.txt\n.agents/skills/lark-skip\n").unwrap();

    let mut a = args(f.src.clone(), f.dst.clone());
    a.exclude_from = Some(rules);
    let stats = linkcopy::copy::run(&a).unwrap();

    assert_eq!(stats.errors, 0);
    assert_eq!(stats.links, 2);
    assert!(!f.dst.join("excluded.txt").exists());
    assert!(!f.skills(&f.dst).join("lark-skip").exists());
    assert!(f.dst.join("normal.txt").exists());
}

#[test]
fn dry_run_writes_nothing() {
    let f = Fixture::new();
    let mut a = args(f.src.clone(), f.dst.clone());
    a.dry_run = true;
    let stats = linkcopy::copy::run(&a).unwrap();

    assert_eq!(stats.errors, 0);
    assert!(!f.dst.exists(), "预览模式不能创建目标目录");
    assert!(f.skills(&f.src).join("lark-one").exists());
}

#[test]
fn never_deletes_extra_files_in_destination() {
    let f = Fixture::new();
    fs::create_dir_all(&f.dst).unwrap();
    fs::write(f.dst.join("extra.txt"), b"keep").unwrap();

    linkcopy::copy::run(&args(f.src.clone(), f.dst.clone())).unwrap();
    assert_eq!(fs::read(f.dst.join("extra.txt")).unwrap(), b"keep");
}

#[test]
fn default_skips_existing_and_force_overwrites_files() {
    let f = Fixture::new();
    fs::create_dir_all(&f.dst).unwrap();
    fs::write(f.dst.join("normal.txt"), b"old").unwrap();

    let stats = linkcopy::copy::run(&args(f.src.clone(), f.dst.clone())).unwrap();
    assert_eq!(stats.skipped, 1);
    assert_eq!(fs::read(f.dst.join("normal.txt")).unwrap(), b"old");

    let mut a = args(f.src.clone(), f.dst.clone());
    a.force = true;
    linkcopy::copy::run(&a).unwrap();
    assert_eq!(fs::read(f.dst.join("normal.txt")).unwrap(), b"normal");
}

#[test]
fn force_replaces_existing_link_without_touching_its_target() {
    let f = Fixture::new();
    fs::create_dir_all(f.skills(&f.dst)).unwrap();
    junction::create(&f.src, f.skills(&f.dst).join("lark-one")).unwrap();

    let mut a = args(f.src.clone(), f.dst.clone());
    a.force = true;
    let stats = linkcopy::copy::run(&a).unwrap();

    assert_eq!(stats.errors, 0);
    assert_eq!(
        junction::get_target(f.skills(&f.dst).join("lark-one")).unwrap(),
        f.target
    );
    assert!(
        f.src.join("normal.txt").exists(),
        "不能删掉链接旧目标里的内容"
    );
}

#[test]
fn rejects_destination_inside_source() {
    let f = Fixture::new();
    let err = linkcopy::copy::run(&args(f.src.clone(), f.src.join("nested"))).unwrap_err();
    assert!(err.to_string().contains("互相嵌套"));
}

#[test]
fn refuses_writes_outside_destination_root() {
    let root = Path::new(r"C:\base\dst");
    assert!(linkcopy::path_guard::ensure_within(root, &root.join("ok")).is_ok());
    assert!(linkcopy::path_guard::ensure_within(root, Path::new(r"C:\base\outside")).is_err());
}
