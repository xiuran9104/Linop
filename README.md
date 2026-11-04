# Linop

原生的一体式终端工作区:SSH 交互 shell、SFTP 双窗格文件管理、端口转发、服务器资源监控、会话录制回放,单个 TUI 应用全搞定。

## 功能

- **多会话** — 多台主机同时在线,列表内一键切换;会话终端状态各自保留;
- **断线自动重连** — 意外断线保留会话与终端历史,自动重连(最多 5 次,指数退避);成功后恢复 shell、远端目录与端口转发,重连期间列表标记 `[重连中]`;
- **连接管理** — profile 持久化在 `~/.config/linop/connections.toml`,支持密码与私钥认证,主机指纹 TOFU 校验(`known_hosts`);TUI 内即可新建/编辑/删除连接(表单,密码掩码);`i` 一键从 `~/.ssh/config` 导入(同名跳过);空密码时自动尝试默认密钥(`~/.ssh/id_ed25519|id_ecdsa|id_rsa`),加密私钥/无默认密钥时在连接时弹掩码输入框就地输入(均不落盘);
- **Shell**(F1) — 完整终端仿真(vt100 解析 + 256 色/RGB/加粗斜体下划线/反色),光标跟随,10000 行回滚浏览,粘贴走 bracketed-paste;
- **文件**(F2) — 远程/本地双窗格浏览、目录进入/上级、上传、下载(目录递归)、新建目录、重命名、删除;传输为后台流式(32 KiB 分块,大文件不进内存),状态栏实时进度条,`c` 可取消,下载先落 `.linop-part` 再原子改名;
- **转发**(F3) — 本地转发(-L)、反向转发(-R)、SOCKS 动态转发(-D)自动启动、统一展示(失败原因同页),会话内 `↑/↓` 选中、`x` 停止单条;
- **会话录制回放** — Shell 页 `v` 一键录制(asciinema cast v2,逐事件 flush 崩溃不丢);UTF-8 跨包自动拼合;状态栏 `⏺ REC` 标记;列表页 `p` 回放最新录制(可改路径),支持 `Space` 播放/暂停、`→` 逐事件、`←` 回退重放,标准格式可直接 `asciinema play` 回放;
- **监控**(F4) — 远端资源走势:CPU / 内存 sparkline、根分区磁盘用量、1/5/15 分钟负载;每 5s 经 SSH exec 读取 `/proc` 与 `df`(单条 POSIX 命令,CPU 用两次 `/proc/stat` 差分),保留 120 个样本(约 10 分钟);进入页面才启动采样,断线自动停止、重连后自动恢复;

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

[[web-1.forwards]]         # 可选:连接后自动启动的本地转发(-L)
local_port = 8080
remote_host = "127.0.0.1"  # 以远端机器视角解析
remote_port = 80

[[web-1.remote_forwards]]  # 可选:反向转发(-R)
bind_port = 15432          # 远端监听端口
target_host = "127.0.0.1"  # 回到本机后连接的目标
target_port = 5432
# bind_host = "0.0.0.0"    # 可选,默认 127.0.0.1

dynamic_ports = [1080]     # 可选:SOCKS5 动态转发(-D)本地端口
```

首次连接会记录主机指纹到 `~/.config/linop/known_hosts`;指纹变化(疑似中间人)时拒绝连接。

## 按键

| 场景 | 按键 |
| --- | --- |
| 连接列表 | `↑/↓` 或 `j/k` 选择 · `Enter` 连接 · `r` 重载配置 · `q`/`Esc` 退出 |
| 标签页 | `F1` Shell · `F2` 文件 · `F3` 转发 · `F4` 监控 |
| 录制 | Shell 页 `v` 开始/结束;列表页 `p` 回放 |
| Shell 页 | 直接输入发送到远端;`PgUp`/`PgDn` 回滚浏览 |
| 文件页 | `Tab`/`←/→` 切换远程/本地窗格 · `↑/↓` 选择 · `Enter` 进入目录 · `Backspace` 上级 · `d` 下载选中到本地窗格目录 · `u` 上传选中到远端目录 · `n` 新建目录 · `R` 重命名 · `x` 删除(目录须为空) |
| 全局 | `F9` 断开回列表 · `Ctrl+Q` 退出应用 |

## 架构

| crate | 职责 |
| --- | --- |
| `linop-core` | 连接 profile 数据模型与 TOML 注册表 |
| `linop-ssh` | russh 客户端:认证、shell/SFTP 通道、exec、TOFU 指纹库、本地/反向/SOCKS 转发 |
| `linop-sftp` | russh-sftp 文件操作:浏览/上传/下载/增删改 |
| `linop-term` | vt100 终端仿真 + ratatui 渲染(含回滚) |
| `linop-monitor` | 远端资源采样:/proc + df 探针命令、解析、采样循环 |
| `linop-cast` | 会话录制:asciinema cast v2 写入/解析(UTF-8 跨包拼合) |
| `linop-testsupport` | 进程内 SSH/SFTP 测试服务器(密码/公钥、shell 回显、exec、direct-tcpip、文件系统 SFTP) |
| `apps/linop` | TUI 应用:状态机、事件循环、渲染、按键编码 |

## 开发

```bash
cargo test --workspace      # 70+ 测试,含进程内 e2e(无需系统 sshd)
cargo clippy --workspace --all-targets
cargo fmt --all
```
