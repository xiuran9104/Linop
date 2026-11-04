# Linop

原生的一体式终端工作区:SSH 交互 shell、SFTP 双窗格文件管理、本地端口转发,单个 TUI 应用全搞定。

## 功能

- **连接管理** — profile 持久化在 `~/.config/linop/connections.toml`,支持密码与私钥认证,主机指纹 TOFU 校验(`known_hosts`);
- **Shell**(F1) — 完整终端仿真(vt100 解析 + 256 色/RGB/加粗斜体下划线/反色),光标跟随,10000 行回滚浏览,粘贴走 bracketed-paste;
- **文件**(F2) — 远程/本地双窗格浏览、目录进入/上级、上传、下载(目录递归)、新建目录、重命名、删除;
- **转发**(F3) — 按 profile 自动启动的本地端口转发一览,失败规则及原因同页展示。

## 构建与运行

```bash
cargo build --release
./target/release/linop            # 进入连接选择器
./target/release/linop my-server  # 直接连接指定 profile
```

## 配置

`~/.config/linop/connections.toml`(可用 `LINOP_CONFIG_DIR` 覆盖目录):

```toml
[web-1]
host = "192.168.1.10"
port = 22                  # 可省略,默认 22
username = "root"
password = "secret"        # 密码认证;或改用下面两行做密钥认证
# key_path = "~/.ssh/id_ed25519"
# key_passphrase = "..."   # 私钥口令,无口令可省略

[[web-1.forwards]]         # 可选:连接后自动启动的本地转发
local_port = 8080
remote_host = "127.0.0.1"  # 以远端机器视角解析
remote_port = 80
```

首次连接会记录主机指纹到 `~/.config/linop/known_hosts`;指纹变化(疑似中间人)时拒绝连接。

## 按键

| 场景 | 按键 |
| --- | --- |
| 连接列表 | `↑/↓` 或 `j/k` 选择 · `Enter` 连接 · `r` 重载配置 · `q`/`Esc` 退出 |
| 标签页 | `F1` Shell · `F2` 文件 · `F3` 转发 |
| Shell 页 | 直接输入发送到远端;`PgUp`/`PgDn` 回滚浏览 |
| 文件页 | `Tab`/`←/→` 切换远程/本地窗格 · `↑/↓` 选择 · `Enter` 进入目录 · `Backspace` 上级 · `d` 下载选中到本地窗格目录 · `u` 上传选中到远端目录 · `n` 新建目录 · `R` 重命名 · `x` 删除(目录须为空) |
| 全局 | `F9` 断开回列表 · `Ctrl+Q` 退出应用 |

## 架构

| crate | 职责 |
| --- | --- |
| `linop-core` | 连接 profile 数据模型与 TOML 注册表 |
| `linop-ssh` | russh 客户端:认证、shell 通道、SFTP 通道、TOFU 指纹库、本地转发 |
| `linop-sftp` | russh-sftp 文件操作:浏览/上传/下载/增删改 |
| `linop-term` | vt100 终端仿真 + ratatui 渲染(含回滚) |
| `linop-testsupport` | 进程内 SSH/SFTP 测试服务器(密码/公钥、shell 回显、direct-tcpip、文件系统 SFTP) |
| `apps/linop` | TUI 应用:状态机、事件循环、渲染、按键编码 |

## 开发

```bash
cargo test --workspace      # 35+ 测试,含进程内 e2e(无需系统 sshd)
cargo clippy --workspace --all-targets
cargo fmt --all
```
