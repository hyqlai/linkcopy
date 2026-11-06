use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Junction,
    SymlinkDir,
    SymlinkFile,
}

/// Windows 上 Junction 在目录枚举时表现为普通目录，只有重解析点标签能可靠区分，
/// 因此先判断 Junction，再退回标准库的符号链接判断。
pub fn classify(path: &Path) -> Result<Option<LinkKind>> {
    #[cfg(windows)]
    {
        // 普通文件/目录上 exists 返回 false；个别文件系统报错时按非链接处理，
        // 由后续的 symlink_metadata 暴露真实问题。
        if junction::exists(path).unwrap_or(false) {
            return Ok(Some(LinkKind::Junction));
        }
    }
    let meta =
        fs::symlink_metadata(path).with_context(|| format!("无法读取元数据 {}", path.display()))?;
    if meta.file_type().is_symlink() {
        let is_dir = fs::metadata(path).map(|m| m.is_dir()).unwrap_or(true);
        return Ok(Some(if is_dir {
            LinkKind::SymlinkDir
        } else {
            LinkKind::SymlinkFile
        }));
    }
    Ok(None)
}

/// 只读取链接自身保存的目标字符串，不解析、不遍历目标内容。
pub fn read_target(path: &Path, kind: LinkKind) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if kind == LinkKind::Junction {
            return junction::get_target(path)
                .with_context(|| format!("无法读取 Junction 目标 {}", path.display()));
        }
    }
    let _ = kind;
    fs::read_link(path).with_context(|| format!("无法读取链接目标 {}", path.display()))
}

pub fn recreate(target: &Path, dst: &Path, kind: LinkKind) -> Result<()> {
    #[cfg(windows)]
    {
        match kind {
            LinkKind::Junction => junction::create(target, dst).with_context(|| {
                format!(
                    "无法创建 Junction {} -> {}",
                    dst.display(),
                    target.display()
                )
            })?,
            LinkKind::SymlinkDir => {
                std::os::windows::fs::symlink_dir(target, dst).with_context(|| {
                    format!(
                        "无法创建目录符号链接 {}（需要开发者模式或管理员权限）",
                        dst.display()
                    )
                })?
            }
            LinkKind::SymlinkFile => {
                std::os::windows::fs::symlink_file(target, dst).with_context(|| {
                    format!(
                        "无法创建文件符号链接 {}（需要开发者模式或管理员权限）",
                        dst.display()
                    )
                })?
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = kind;
        std::os::unix::fs::symlink(target, dst)
            .with_context(|| format!("无法创建符号链接 {}", dst.display()))?;
    }
    Ok(())
}

/// 覆盖时只删除链接本身，绝不递归进入链接指向的目录。
/// RemoveDirectory/DeleteFile 作用在链接项上，不会触及目标内容；
/// junction::delete 只清除重解析点、仍会留下同名目录，因此这里不用它。
pub fn remove_link(path: &Path, kind: LinkKind) -> Result<()> {
    match kind {
        LinkKind::Junction | LinkKind::SymlinkDir => {
            fs::remove_dir(path).with_context(|| format!("无法删除目录链接 {}", path.display()))?
        }
        LinkKind::SymlinkFile => fs::remove_file(path)
            .with_context(|| format!("无法删除文件符号链接 {}", path.display()))?,
    }
    Ok(())
}
