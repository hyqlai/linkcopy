use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// 排除规则集中在文件里，避免命令行散落多条规则造成遗漏或冲突。
#[derive(Debug, Default)]
pub struct ExcludeList {
    rules: Vec<String>,
}

impl ExcludeList {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let text = fs::read_to_string(path)
            .with_context(|| format!("无法读取排除清单 {}", path.display()))?;
        let rules = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(normalize)
            .collect();
        Ok(Self { rules })
    }

    /// 规则命中相对路径本身，或命中其任一上级目录，即整棵子树一起排除。
    pub fn is_excluded(&self, rel: &Path) -> bool {
        let rel = normalize(rel);
        self.rules
            .iter()
            .any(|rule| rel == *rule || rel.starts_with(&format!("{rule}/")))
    }
}

fn normalize(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}
