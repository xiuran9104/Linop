# @linop-db-app/plugin-cli

Precompiled `linop-db-plugin` CLI for creating and packaging DBX plugins. Installing this package does not compile the CLI and does not require a DBX source checkout.

```bash
npm install --global @linop-db-app/plugin-cli
linop-db-plugin create my-plugin
```

Or run it without a global installation:

```bash
npx @linop-db-app/plugin-cli create my-plugin
```

The package selects a precompiled binary for macOS, Linux, or Windows and bundles the matching Rust and Go DBX plugin SDK sources. Frontend-only plugins, including the Svelte template, require only Node.js. Rust and Go are needed only when the plugin itself has a Rust or Go backend.

```bash
linop-db-plugin create my-plugin --template frontend
linop-db-plugin create my-svelte-plugin --template svelte
linop-db-plugin create my-rust-plugin --template rust
linop-db-plugin create my-go-plugin --template go
linop-db-plugin package my-plugin
linop-db-plugin dev --path my-plugin --port 5190
```

The `dev` subcommand requires Node.js 22+ and runs the bundled browser development runtime, without DBX or source-tree dependencies. It supports frontend-only workbenches and Rust/Go backends, with JSONL or framed transport. Optional `[dev]` `ui_build` and `ui_watch` command arrays configure project builds. Development credentials are stored locally as plaintext under `.linop-db-dev/` (or `--data-dir`). Other commands retain their existing Node requirements.

## 中文说明

`@linop-db-app/plugin-cli` 提供预编译的 `linop-db-plugin` 命令。安装时不会编译 CLI，也不需要克隆 DBX 源码仓库。

```bash
npm install -g @linop-db-app/plugin-cli
linop-db-plugin create my-plugin
```

也可以直接使用：

```bash
npx @linop-db-app/plugin-cli create my-plugin
```

npm 主包会自动选择当前操作系统对应的预编译二进制，并携带匹配版本的 Rust 和 Go 插件 SDK。纯前端插件只需要 Node.js；只有插件自身包含 Rust 或 Go 后端时才需要对应语言的编译环境。

使用 `linop-db-plugin dev --path my-plugin --port 5190` 可在浏览器运行通用插件开发环境，无需启动 DBX。仅 `dev` 要求 Node.js 22+；运行时及外壳已包含在 npm 包中。开发配置和凭据以明文保存在 `.linop-db-dev/`，不要提交或打包该目录。
