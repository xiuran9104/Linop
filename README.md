# Linop

原生的一体式终端工作区:SSH 交互 shell、SFTP 双窗格文件管理、端口转发、服务器资源监控、会话录制回放、SQLite 数据库浏览、Redis 键浏览、Telnet 终端,单个 TUI 应用全搞定。

## 功能

- **多会话** — 多台主机同时在线,列表内一键切换;会话终端状态各自保留;
- **断线自动重连** — 意外断线保留会话与终端历史,自动重连(最多 5 次,指数退避);成功后恢复 shell、远端目录与端口转发,重连期间列表标记 `[重连中]`;
- **连接管理** — profile 持久化在 `~/.config/linop/connections.toml`,支持密码与私钥认证,主机指纹 TOFU 校验(`known_hosts`);TUI 内即可新建/编辑/删除连接(表单,密码掩码);`i` 一键从 `~/.ssh/config` 导入(同名跳过);空密码时自动尝试默认密钥(`~/.ssh/id_ed25519|id_ecdsa|id_rsa`),加密私钥/无默认密钥时在连接时弹掩码输入框就地输入(均不落盘);
- **Shell**(F1) — 完整终端仿真(vt100 解析 + 256 色/RGB/加粗斜体下划线/反色),光标跟随,10000 行回滚浏览,粘贴走 bracketed-paste;
- **文件**(F2) — 远程/本地双窗格浏览、目录进入/上级、上传、下载(目录递归)、新建目录、重命名、删除;传输为后台流式(32 KiB 分块,大文件不进内存),状态栏实时进度条,`c` 可取消,下载先落 `.linop-part` 再原子改名;
- **转发**(F3) — 本地转发(-L)、反向转发(-R)、SOCKS 动态转发(-D)自动启动、统一展示(失败原因同页),会话内 `↑/↓` 选中、`x` 停止单条;
- **Telnet 终端** — `telnet.toml` 配置目标(端口缺省 23),列表页 `Enter` 进入交互式 NVT 终端(复用 vt100 仿真与回滚);RFC 854 选项协商自动应答(WONT/DONT,哑终端策略),0xFF 转义、跨包 IAC 状态机;读写半边分离,读轮询不阻塞输入;`F9` 断开;
- **数据库(SQLite)** — 表/视图列表(行数+列)、`Enter` 查看数据网格(分页、行号、NULL 灰显)、`s` 执行任意单条 SQL(SELECT 自动出网格,DML/DDL 显示影响行数并刷新表列表);SQL 全部在阻塞线程执行并带超时上限,大查询不冻结界面;驱动层已按多驱动划分,为 MySQL/PostgreSQL 留位;
- **Redis 键浏览** — SCAN 键列表(类型/长度/TTL,五类分色)、`Enter` 查看值(string/list/hash/set/zset 按类型排版,分数与字段对齐)、`x` 删除(确认)、`r` 重扫;可选 AUTH 口令;`via = "跳板名"` 时经 SSH 本地隧道访问内网 Redis(浏览期间隧道保活,退出即拆);纯 Rust RESP2 协议实现,零外部依赖;
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

数据库:`~/.config/linop/databases.toml`:

```toml
[local]
driver = "sqlite"
path = "/data/app.db"

[cache]                  # Redis:可选 password
driver = "redis"
host = "10.0.0.5"
port = 6379
# via = "jump"           # 可选:经 connections.toml 里的跳板机建 SSH 隧道访问
```

## 按键

| 场景 | 按键 |
| --- | --- |
| 连接列表 | `↑/↓` 或 `j/k` 选择 · `Enter` 连接 · `r` 重载配置 · `q`/`Esc` 退出 |
| 标签页 | `F1` Shell · `F2` 文件 · `F3` 转发 · `F4` 监控 |
| 录制 | Shell 页 `v` 开始/结束;列表页 `p` 回放 |
| 数据库 | 列表页选中数据库 `Enter` 进入 · `s` SQL · `←/→` 翻页 |
| Telnet | 列表页选中 `Enter` 连接 · `F9` 断开 |
| Redis | 键列表 `Enter` 值 · `x` 删除 · `r` 重扫 |
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
| `linop-db` | 数据库驱动层:SQLite(表/列/查询/执行),多驱动留位 |
| `linop-redis` | RESP2 客户端:SCAN/TYPE/TTL/取值/DEL/AUTH(纯 Rust) |
| `linop-telnet` | RFC 854 客户端:IAC 状态机/协商应答/转义(哑终端) |
| `linop-testsupport` | 进程内测试服务器:SSH/SFTP(密码/公钥、shell 回显、exec、direct-tcpip)+ 迷你 Redis + 迷你 Telnet |
| `apps/linop` | TUI 应用:状态机、事件循环、渲染、按键编码 |

## 开发

```bash
cargo test --workspace      # 79 测试,含进程内 e2e(无需系统 sshd)
cargo clippy --workspace --all-targets
cargo fmt --all
```

## GUI 形态:内置 navop 全量源码(vendored)

`vendor/navop/` 收录 [navop](https://github.com/feigeCode/navop) 完整源码
(GPUI GPU 加速原生 GUI、多面板、主题、三语 i18n、加密同步 —— 即 README
开篇对标的全部产品形态)。Linop 仓库已完成其本地化构建闭环:

- **构建**:`docker run --rm -v $PWD/vendor/navop:/w navop-build:2 bash -c 'cd /w && cargo build -p navop'`
  (镜像 = rust:1-slim + glib/gtk3/webkit2gtk-4.1/vulkan/mesa 全套系统依赖;
  gpui-kit 的 git 依赖已改为 `file://` 本地 bare 仓库,不依赖外网 git)
- **验证**:编译通过(~650 crates,6m50s);Xvfb + llvmpipe 软渲染下
  GUI 完整启动,截图见 [`docs/navop-gui-verified.png`](docs/navop-gui-verified.png)
- **许可**:navop 为 Apache-2.0,附 `NAVOP_LICENSE` 补充条款(禁止商业转售);
  本仓库按其许可条款原样收录并保留全部声明
- **在 Windows 桌面运行(WSLg)**:`script/navop-gui.sh start` —— 容器挂载
  X11 套接字与 GPU,Navop 窗口直接出现在 Windows 桌面
  (`stop` / `status` / `logs` 子命令同理)
- **release 构建**:`cargo build -p navop --release`(fat LTO + strip,
  约 25 分钟),二进制 164M(debug 3.7G 的 4.4%);GUI 容器已默认切换
- **渲染后端(定论)**:真 GPU 加速在本机 WSL 上不可行,证据链完整:
  1. 发行版(Debian/Ubuntu/Fedora)mesa 包均不含 dzn —— 已自编
     (mesa 23.3.6 + DirectX-Headers 1.610.0 对齐 WSL 驱动 SDK 610,
     `script/build-mesa-dzn.sh` 一键复现,产物 41M 编译通过);
  2. dzn ICD 注册/加载/到 DXCore 的整条链路全通(strace 确认
     /dev/dxg 打开成功);
  3. **断点在宿主驱动栈**:`script/dxg-probe.c`(裸 COM 探针)实测
     WSL 的 DXCore 层 D3D12 GRAPHICS / CORE_COMPUTE 适配器数均为 0
     (宿主直跑与容器内一致;nvidia-smi 可见 RTX 5060 Ti)—— 即
     本机 GPU 直通只启用了 CUDA 通道,未暴露 D3D12 适配器,任何
     Vulkan-on-D3D12(含 dzn)无从工作;
  4. 出路:Windows 侧更新 NVIDIA 驱动并确认 GPU 半虚拟化配置、或
     Windows 原生运行。当前 wgpu 自动落到 llvmpipe 软渲染,可用
- **Linop 扩展接入 navop**:`crates/linop-navop-importer` ——
  wasm32-wasip2 组件,实现 navop `connection-importer` WIT world,
  把 Linop 的 connections.toml(SSH + 本地/反向/SOCKS 转发)导入
  navop 连接库;协议形状对齐宿主权威模型(原样拷贝于 vendored
  connection-import-protocol,18 个单测含逐字段 JSON 断言与
  validate_shape 校验);安装:`docker cp` 到
  `/root/.navop-dev/config/extensions/composite/com.linop.importer.config/`
  (engines 需 `>=0.23.0-0` 匹配 dev 预发布版,见 semver 预发布规则)

Linop 自研部分(`crates/linop-*`、`apps/linop` TUI)与 navop 源码相互独立;
`linop-i18n` / `linop-theme` / `linop-sync` 三个 crate 为同能力的轻量实现,
可供任意前端(含 navop fork)复用。
