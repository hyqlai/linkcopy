# linkcopy 使用说明

## 解决什么问题

Windows 的目录联接（Junction）是「目录 + NTFS 重解析点」。资源管理器复制、`robocopy /E`、`Copy-Item -Recurse`、`cp -a` 都按普通目录处理，会把链接解引用成实体目录，复制完链接就没了；`robocopy /XJ` 则直接跳过。

linkcopy 遍历时把链接当成独立条目：只读取链接自身保存的目标字符串，在目标目录重建同类型链接，不跟随、不读取链接指向的内容。

## 环境要求

- Windows + NTFS。创建 Junction 不需要管理员权限。
- 重建普通符号链接需要开发者模式或管理员权限；权限不足会报错并计入错误数，不会静默降级。
- 构建需要 Rust ≥ 1.95：`cargo build --release`，产物为 `target/release/linkcopy.exe`。

## 命令

```
linkcopy <SRC> <DST> [OPTIONS]
```

| 参数 | 作用 |
| --- | --- |
| `--exclude-from <FILE>` | 从文件读取排除规则，每行一条，`#` 开头为注释 |
| `--dry-run` | 只打印计划，不写入任何内容 |
| `--force` | 覆盖目标端已存在的同名文件或链接 |
| `--verbose` | 逐项输出处理过程 |
| `--quiet` | 只输出错误与最终摘要 |
| `-h` / `--help` | 帮助 |
| `-V` / `--version` | 版本 |

```powershell
linkcopy.exe E:\project\lsh\feishu\base E:\project\lsh\feishu\lf02
linkcopy.exe --dry-run --verbose E:\project\lsh\feishu\base E:\project\lsh\feishu\lf02
```

排除清单是纯文本，路径相对源根目录，`\` 与 `/` 均可；命中某个目录会排除整棵子树；**不支持通配符/glob**：

```
# 每行一条
node_modules
.agents/skills/lark-wiki
```

## 行为约定

- 不删除目标端多余文件，也不做镜像同步。
- 目标端已存在同名项时默认跳过；`--force` 只覆盖文件或链接，**拒绝删除实体目录**，避免不可恢复的数据丢失。
- 两端都是普通目录时复用目标目录并继续递归，因此可以对同一目标重复执行做增量复制。
- 遍历使用 `symlink_metadata`，遇到 Junction / 符号链接只重建，不递归进入，也不会因为链接指向目录外而越权读取。
- 所有写入路径必须落在目标根目录内；源与目标相同或互相嵌套时直接拒绝，避免递归复制。
- 单条出错时继续处理其余条目，最后汇总错误数。退出码：`0` 全部成功，`1` 部分条目失败，`2` 参数或致命错误。
- 不调用任何外部命令或 shell，排除规则集中在文件里。

## 已知限制

- Junction 内部存的是绝对路径（例如 `C:\Users\test\.skills\github\...`），工具原样保留。跨机器、换 Windows 用户名或目标仓库被移动后，链接仍存在但指向的位置可能不存在；需要可移植性时应另行设计路径，而不是靠复制工具改写目标。
- 只复制普通文件与目录，不处理硬链接、ACL、备用数据流、稀疏文件等属性；普通文件保留修改时间。
- 非 Windows 平台按 POSIX 符号链接处理，不涉及 Junction。

## 与系统命令对比

| 方式 | 复制后的结果 |
| --- | --- |
| `robocopy /E`、资源管理器、`Copy-Item -Recurse` | 链接被解引用成实体目录 |
| `robocopy /E /XJ` | 链接被跳过，目标里没有链接 |
| `xcopy /B` | 得到空目录 |
| `linkcopy` | 重建 Junction，目标路径与源一致 |
