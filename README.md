# Linop

原生的一体式终端工作区:SSH 交互 shell、SFTP 双窗格文件管理、端口转发、服务器资源监控、会话录制回放、Telnet 终端,单个 TUI 应用全搞定。数据库管理交给 DBX(dbxio.com)集成,见文末。

## 功能

- **多会话** — 多台主机同时在线,列表内一键切换;会话终端状态各自保留;
- **断线自动重连** — 意外断线保留会话与终端历史,自动重连(最多 5 次,指数退避);成功后恢复 shell、远端目录与端口转发,重连期间列表标记 `[重连中]`;
- **连接管理** — profile 持久化在 `~/.config/linop/connections.toml`,支持密码与私钥认证,主机指纹 TOFU 校验(`known_hosts`);TUI 内即可新建/编辑/删除连接(表单,密码掩码);`i` 一键从 `~/.ssh/config` 导入(同名跳过);空密码时自动尝试默认密钥(`~/.ssh/id_ed25519|id_ecdsa|id_rsa`),加密私钥/无默认密钥时在连接时弹掩码输入框就地输入(均不落盘);
- **Shell**(F1) — 完整终端仿真(vt100 解析 + 256 色/RGB/加粗斜体下划线/反色),光标跟随,10000 行回滚浏览,粘贴走 bracketed-paste;
- **文件**(F2) — 远程/本地双窗格浏览、目录进入/上级、上传、下载(目录递归)、新建目录、重命名、删除;传输为后台流式(32 KiB 分块,大文件不进内存),状态栏实时进度条,`c` 可取消,下载先落 `.linop-part` 再原子改名;
- **转发**(F3) — 本地转发(-L)、反向转发(-R)、SOCKS 动态转发(-D)自动启动、统一展示(失败原因同页),会话内 `↑/↓` 选中、`x` 停止单条;
- **Telnet 终端** — `telnet.toml` 配置目标(端口缺省 23),列表页 `Enter` 进入交互式 NVT 终端(复用 vt100 仿真与回滚);RFC 854 选项协商自动应答(WONT/DONT,哑终端策略),0xFF 转义、跨包 IAC 状态机;读写半边分离,读轮询不阻塞输入;`F9` 断开;
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
| Telnet | 列表页选中 `Enter` 连接 · `F9` 断开 |
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

## 数据库:直接复用 DBX(已并入本仓)

数据库管理不再自研,改为**整体复用 [DBX](https://github.com/t8y2/dbx)**
(Tauri+Rust,Apache-2.0,100+ 数据库引擎,桌面/Docker/CLI/MCP 四形态)。
vendored 于 `vendor/dbx/`,与 navop 同法本地化(gaussdb/mysql_async fork
改 `file://` bare 仓库,patch 上移至根 workspace)。

- **GPUI 原生数据库面板已接线**:`gui/crates/dbx_view` —— 数据域重新落座
  (但数据层换成 DBX):`ConnectionType::Database/Redis` 的连接打开入口
  指向 `DbxPanel`(连接选择 + 表列表 + SQL 执行 + 结果网格,GPUI 原生,
  走 `one_core::gpui_tokio` 桥在 tokio 线程池执行 dbx 查询);rusqlite
  已统一到 0.38(navop/dbx 对齐,libsqlite3-sys links 冲突消失),
  迁移点:DatabaseName→MAIN_DB、u64/usize 绑定→i64
- **GUI 抽出为顶层 `gui/`**:GPUI 桌面壳整体移出 vendor,**navop 名字全树清除**
  (目录/包名/资源/身份串:apps/linop、linop_app、linop_runtime、linop-shell);
  **数据库域整体删除**(db/db_view/db-value/redis/mongodb/er_flow/runtime_database/
  runtime_redis/data_domain 共 12 个 crate 及 feature 门控 —— 壳按域注册表渲染,
  域删除后数据库入口自然消失);russh fork 同步 bare 化;数据目录变为
  `~/.linop-dev`(原 `~/.navop-dev` 不自动迁移)
- **`crates/linop-dbx`** —— 数据库模块,**DBX 仅作驱动执行层**(用户决策:
  DBX 的 Web 前端与其 dbx.db 业务存储不采用):`AppState` 挂在 Linop 数据
  目录下的空技术库上,连接配置完全归 Linop 自己管(`databases.toml` →
  会话内注入),`query`/`list_tables` 直调 `dbx_core` 的执行路径
  (连接池/超时/风控/取消全部复用);CLI 桥 `linop-dbx connections|tables|query`;
  Postgres 真实容器 e2e(表元数据/查询/非法 SQL)验证
- **连接配置**:`~/.linop-dev/databases.toml`(驱动:sqlite/redis/
  postgres/mysql,支持 `via` 跳板;GUI 数据库面板用 GPUI 原生实现,
  开发中)
- **russh 已统一**:dbx 侧升级 0.60→0.65(与 navop 侧同版本,
  ed25519-dalek 3 冲突消失),为 navop+dbx 合并清障;移植面仅
  `check_server_key` 回调签名一处。
- **品牌**:用户可见层已全部 Linop(窗口标题/WM_CLASS/数据目录
  `~/.linop-dev`/托盘/常量与文案;构建镜像 `linop-build`);
  vendored 上游的**内部 crate 名**(dbx-core 等)与**上游 fork 的
  git tag 标识**(`v0.65.0-navop.1` 等,是外部仓库真实 tag,改名即
  无法拉取)暂保留,合并阶段一次性机械重命名。
- **合并路线**:navop(GUI 壳)⊕ dbx(数据库内核)→ 统一 Linop;
  两棵 vendored 树已在本 workspace 共存(russh/ed25519 对齐)。

> 注:DBX 许可证以仓库 LICENSE 为准(Apache-2.0)。
