use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

pub fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path)
        .map_err(|err| anyhow::anyhow!("无法解析路径 {}: {err}", path.display()))
}

/// 所有写入路径都必须在目标根目录内，防止异常条目名把数据写到目录外。
pub fn ensure_within(root: &Path, child: &Path) -> Result<()> {
    let root = absolute(root)?;
    let child = absolute(child)?;
    if !child.starts_with(&root) {
        bail!("拒绝写入目标目录之外的路径：{}", child.display());
    }
    Ok(())
}

/// 源与目标互为祖先会造成递归复制或污染上级目录，直接拒绝。
pub fn ensure_disjoint(src: &Path, dst: &Path) -> Result<()> {
    let src = absolute(src)?;
    let dst = absolute(dst)?;
    if src == dst || src.starts_with(&dst) || dst.starts_with(&src) {
        bail!(
            "源目录与目标目录不能相同或互相嵌套：{} / {}",
            src.display(),
            dst.display()
        );
    }
    Ok(())
}
