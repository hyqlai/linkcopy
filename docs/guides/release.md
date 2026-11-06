# 发布流程

## 触发规则

- 普通分支推送：只运行 `test` job（格式检查、Clippy、测试），不编译 release 包。
- 推送 `v*` 标签：先运行 `test`，通过后编译 release 并创建 GitHub Release。

## 发布新版本

1. 更新 `Cargo.toml` 中的 `version`，提交并推送到 `main`。
2. 打标签并推送（标签名与版本号一致）：

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. Actions 运行完成后，Release 页面会出现 `linkcopy-v0.2.0-windows-x86_64.zip`，里面包含 `linkcopy.exe` 和 `docs/`。

## 说明

- release job 显式声明 `permissions: contents: write`，因此不依赖仓库默认的 Workflow 权限设置。
- 只有 `v*` 形式的标签会触发发布；其他标签不进入 workflow。
- 构建使用 `--locked`，保证 CI 与提交的 `Cargo.lock` 一致。
- 回滚：删除对应的 GitHub Release 与 tag 即可，不会影响已推送的代码。
