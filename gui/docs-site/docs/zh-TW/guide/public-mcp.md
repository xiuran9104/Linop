# Public MCP 與外部自動化

Public MCP 讓 Codex、Claude Desktop、Claude Code 與相容用戶端呼叫目前正在執行的 Linop。它不是固定雲端 API：使用動態 loopback 連接埠與僅目前使用者可讀的 discovery token，真實工具和 Schema 來自本機應用程式。

## 服務模式與探索

Temporary 適合短期工作，任務或應用程式結束後不應依賴它持續存在；Persistent 適合需要長期本機探索的用戶端。兩者都應只監聽 loopback，並使用受保護的使用者級探索資料。

不要把 discovery 檔、Token 或連接埠公開到網際網路。切換模式、重新啟動 Linop 或變更 Tool Exposure 可能重啟 endpoint，MCP/ACP 用戶端需重新連線。

## 權限檔位與 Tool Exposure

Safe、Confirm 和 Auto 控制審批強度。首次使用選 Safe 或 Confirm；只有任務、目標與工具高度可控時才使用 Auto。這些設定不會取代資料庫或伺服器權限。

Tool Exposure 可分別開放 Terminal、SSH Exec、可見終端、Connections、SFTP、Redis、MongoDB、Database 和內部函式。只啟用當前用戶端需要的群組，任務結束後關閉。

## 安裝用戶端依賴

橋接需要 Node.js 20+ 和可用的 `npx`。先確認版本，再從 Linop 設定複製 Codex、Claude Desktop、Claude Code 或通用 MCP JSON。依各用戶端的路徑和重啟方式完成設定。

Linop 也可安裝或更新供 Codex 與 Agents 使用的 Linop Skill。使用 Skill 前必須全域安裝 `@linop/cli`；AI Agent 透過 `linop ... --json` 按需探索並操作 Linop 中的資料庫、SSH、終端、檔案、連線和工作區資源。Skill 不會把全部工具靜態寫進提示詞，而是指導 Agent 在需要時讀取狀態、命令與即時 Schema。

## 為什麼使用 Linop Skill

直接把 Linop 設定為原生 MCP Server 時，用戶端可能在每輪對話向模型攜帶大量已開放工具的名稱、描述和 JSON Schema。工具越多，重複定義就越占用上下文與 Token。Linop Skill 讓 AI Agent 平時只保留精簡工作流程，真正執行任務時才從終端按需執行 `linop` 的狀態、領域命令或 `tool schema/call`。

```bash
npm install -g @linop/cli@latest
linop skill install --target codex --scope user
linop status --json
linop db query --help
linop tool schema <tool-name> --json
linop tool call <tool-name> --arguments '<json-object>' --json
```

以下是常見的唯讀操作範例。占位符必須替換為 Linop 即時回傳的連線或工作階段 ID，執行前應先查看 `--help` 或即時 Schema：

```bash
linop connections list --json
linop connections sessions --json
linop ssh exec --target <ssh-session-id> --command 'uname -a' --json
linop sftp list --connection <ssh-connection-id-or-name> --path /var/log --json
linop redis get --connection-id <redis-connection-id-or-name> --key app:status --json
linop mongo find --connection-id <mongo-session-id> --database app --collection users --filter '{"active":true}' --limit 20 --json
linop db query --connection <database-connection-id-or-name> --sql 'SELECT 1' --json
linop terminal read --target <terminal-session-id> --lines 80 --json
```

這種方式適合 Codex 等可以執行終端命令的 Agent：不必把完整 Linop 工具目錄預先註冊到每一輪模型上下文，也能按任務探索目前資源與操作，通常可以減少重複上下文和 Token 開銷。實際節省量取決於用戶端如何注入 MCP 工具定義及開放的工具數量。

Skill 不代表底層完全繞過 MCP。`linop` CLI 內部仍連接 Linop 的本機認證 Public MCP endpoint，Linop 繼續負責 Tool Exposure、資源 ID、權限、審批、工作階段、結果和稽核。Skill 改變的是 Agent 端的使用方式：從「每輪攜帶整套工具」改為「透過終端按需探索與呼叫」。

## 使用 @linop/cli

`@linop/cli` 提供 `status`、`tools`、`schema`、`call` 與各資源領域命令。獨立的 `@linop/mcp` 只負責為相容 MCP 用戶端執行 stdio 橋接。

執行 `linop ...` 前確認套件來源。資源 ID 與參數必須來自當前 tools/schema，不得猜測、重用其他裝置 ID 或試圖繞過審批。

需要確認或更新 CLI 時執行：

```bash
npm view @linop/cli version
linop --version
```

使用 Skill 前執行 `npm install -g @linop/cli@latest`，更新已安裝的 CLI 可執行 `npm update -g @linop/cli`。原生 MCP 用戶端使用 `npx -y @linop/mcp@latest` 作為 stdio 橋接。

## 審批與排查

審批視窗會顯示用戶端、操作、資源和參數。非預期請求應拒絕並修正用戶端，不要全面放寬權限。ACP 授權不代表 Public MCP 自動允許。

失敗時檢查 Linop 是否執行、服務模式、Node.js、用戶端設定、discovery 權限、Tool Exposure 和即時 Schema。endpoint 變更後重新連線，分享日誌前遮蔽 Token、路徑和業務參數。
