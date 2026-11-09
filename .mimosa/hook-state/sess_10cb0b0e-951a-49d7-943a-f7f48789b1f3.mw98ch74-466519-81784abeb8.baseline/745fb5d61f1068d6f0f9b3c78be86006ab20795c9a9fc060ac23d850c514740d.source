//! linop-i18n:三语字符串目录(en / zh-CN / zh-TW)。
//!
//! 对应 navop 的三语支持。键为编译期常量,目录为静态表;
//! 缺译回退英文。完整度由测试保证(每键三语非空)。

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// 界面语言。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    #[default]
    En,
    ZhCn,
    ZhTw,
}

impl Lang {
    /// 全部支持的语言(设置页枚举用)。
    pub const ALL: [Lang; 3] = [Lang::En, Lang::ZhCn, Lang::ZhTw];

    /// BCP-47 风格标识。
    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhCn => "zh-CN",
            Lang::ZhTw => "zh-TW",
        }
    }

    /// 语言自显名(语言选择器里用它,不用当前语言翻译语言名)。
    pub fn display(&self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::ZhCn => "简体中文",
            Lang::ZhTw => "繁體中文",
        }
    }

    /// 取键的译文;缺译回退英文,未知键回退键名本身。
    pub fn t(&self, key: &str) -> Cow<'static, str> {
        let Some((en, cn, tw)) = entry(key) else {
            return Cow::Owned(key.to_string());
        };
        let picked = match self {
            Lang::En => en,
            Lang::ZhCn => cn,
            Lang::ZhTw => tw,
        };
        Cow::Borrowed(if picked.is_empty() { en } else { picked })
    }
}

impl FromStr for Lang {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "en" | "En" | "EN" => Ok(Lang::En),
            "zh-CN" | "zh_CN" | "zh-Hans" => Ok(Lang::ZhCn),
            "zh-TW" | "zh_TW" | "zh-Hant" => Ok(Lang::ZhTw),
            other => Err(format!("unsupported lang: {other}")),
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// 全部目录键(测试遍历 + 补全检查)。
#[rustfmt::skip]
pub const ALL_KEYS: &[&str] = &[
    "app.title",
    "action.connect", "action.disconnect", "action.refresh",
    "action.export", "action.import", "action.apply", "action.open",
    "state.connected", "state.connecting", "state.disconnected", "state.error",
    "side.sessions", "side.ssh", "side.telnet",
    "panel.terminal", "panel.settings", "panel.sync", "panel.empty",
    "term.input_hint", "term.send",
    "settings.language", "settings.theme",
    "theme.dark", "theme.light", "theme.nord",
    "sync.title", "sync.desc", "sync.passphrase", "sync.passphrase_short",
    "sync.path", "sync.exported", "sync.imported",
    "sync.wrong_passphrase", "sync.bad_format", "sync.corrupt",
    "sync.mode", "sync.mode.skip", "sync.mode.replace", "sync.files_none",
    "ssh.auth_failed", "ssh.connect_failed",
];

/// 目录:`key → (en, zh-CN, zh-TW)`。空串表示该语言缺译(回退英文)。
fn entry(key: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match key {
        // --- 通用 ---
        "app.title" => ("Linop Workspace", "Linop 工作台", "Linop 工作臺"),
        "action.connect" => ("Connect", "连接", "連線"),
        "action.disconnect" => ("Disconnect", "断开", "斷線"),
        "action.refresh" => ("Refresh", "刷新", "重新整理"),
        "action.export" => ("Export…", "导出…", "匯出…"),
        "action.import" => ("Import…", "导入…", "匯入…"),
        "action.apply" => ("Apply", "应用", "套用"),
        "action.open" => ("Open", "打开", "開啟"),
        "state.connected" => ("Connected", "已连接", "已連線"),
        "state.connecting" => ("Connecting…", "连接中…", "連線中…"),
        "state.disconnected" => ("Disconnected", "已断开", "已斷線"),
        "state.error" => ("Error", "错误", "錯誤"),
        // --- 侧栏 ---
        "side.sessions" => ("Sessions", "会话", "會話"),
        "side.ssh" => ("SSH", "SSH 连接", "SSH 連線"),
        "side.telnet" => ("Telnet", "Telnet", "Telnet"),
        // --- 面板 ---
        "panel.terminal" => ("Terminal", "终端", "終端"),
        "panel.settings" => ("Settings", "设置", "設定"),
        "panel.sync" => ("Sync", "同步", "同步"),
        "panel.empty" => (
            "Open a connection from the sidebar",
            "从侧栏打开一个连接",
            "從側欄開啟一個連線",
        ),
        // --- 终端 ---
        "term.input_hint" => (
            "Type a line and press Enter…",
            "输入一行,按回车发送…",
            "輸入一行,按 Enter 送出…",
        ),
        "term.send" => ("Send", "发送", "送出"),
        // --- 设置 ---
        "settings.language" => ("Language", "语言", "語言"),
        "settings.theme" => ("Theme", "主题", "主題"),
        "theme.dark" => ("Dark", "深色", "深色"),
        "theme.light" => ("Light", "浅色", "淺色"),
        "theme.nord" => ("Nord", "Nord", "Nord"),
        // --- 同步 ---
        "sync.title" => ("Encrypted Sync", "加密同步", "加密同步"),
        "sync.desc" => (
            "Bundle connections / databases / telnet profiles into one AES-256-GCM encrypted file. The passphrase never leaves this device.",
            "把 SSH / 数据库 / Telnet 配置打包为一个 AES-256-GCM 加密文件。口令不会离开本机。",
            "把 SSH / 資料庫 / Telnet 設定打包為一個 AES-256-GCM 加密檔案。通行碼不會離開本機。",
        ),
        "sync.passphrase" => ("Passphrase", "口令", "通行碼"),
        "sync.passphrase_short" => (
            "Passphrase must be at least 8 characters",
            "口令至少 8 个字符",
            "通行碼至少 8 個字元",
        ),
        "sync.path" => ("Bundle file", "备份文件", "備份檔案"),
        "sync.exported" => ("Bundle written", "备份已写入", "備份已寫入"),
        "sync.imported" => ("Bundle applied", "备份已应用", "備份已套用"),
        "sync.wrong_passphrase" => ("Wrong passphrase", "口令错误", "通行碼錯誤"),
        "sync.bad_format" => (
            "Not a Linop bundle",
            "不是 Linop 备份文件",
            "不是 Linop 備份檔案",
        ),
        "sync.corrupt" => ("Bundle is corrupted", "备份文件已损坏", "備份檔案已損壞"),
        "sync.mode" => ("On conflict", "冲突时", "衝突時"),
        "sync.mode.skip" => ("Keep existing", "保留现有", "保留現有"),
        "sync.mode.replace" => ("Overwrite", "覆盖", "覆蓋"),
        "sync.files_none" => (
            "No profiles to bundle",
            "没有可打包的配置",
            "沒有可打包的設定",
        ),
        // --- 数据库 ---
        // --- 错误 ---
        "ssh.auth_failed" => ("Authentication failed", "认证失败", "認證失敗"),
        "ssh.connect_failed" => ("Connection failed", "连接失败", "連線失敗"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 目录完整度:每个键三语非空。
    #[test]
    fn catalog_is_complete_across_languages() {
        let keys = ALL_KEYS;
        assert!(keys.len() >= 40, "目录规模意外缩小:{}", keys.len());
        for key in keys {
            let (en, cn, tw) = entry(key).unwrap_or_else(|| panic!("键 {key} 缺失"));
            assert!(!en.is_empty(), "{key} 缺 en");
            assert!(!cn.is_empty(), "{key} 缺 zh-CN");
            assert!(!tw.is_empty(), "{key} 缺 zh-TW");
        }
    }

    #[test]
    fn translate_per_language_and_fallback() {
        assert_eq!(Lang::ZhCn.t("action.connect"), "连接");
        assert_eq!(Lang::ZhTw.t("action.connect"), "連線");
        assert_eq!(Lang::En.t("action.connect"), "Connect");
        // 未知键:回退键名本身。
        assert_eq!(Lang::En.t("no.such.key"), "no.such.key");
        assert_eq!(Lang::ZhCn.t("no.such.key"), "no.such.key");
    }

    #[test]
    fn lang_parse_roundtrip() {
        assert_eq!("zh-CN".parse::<Lang>().unwrap(), Lang::ZhCn);
        assert_eq!("zh_TW".parse::<Lang>().unwrap(), Lang::ZhTw);
        assert_eq!("en".parse::<Lang>().unwrap(), Lang::En);
        assert!("fr".parse::<Lang>().is_err());
        assert_eq!(Lang::ZhTw.to_string(), "zh-TW");
    }

    #[test]
    fn display_names_are_native() {
        assert_eq!(Lang::ZhCn.display(), "简体中文");
        assert_eq!(Lang::ZhTw.display(), "繁體中文");
        assert_eq!(Lang::En.display(), "English");
    }

    /// 序列化 roundtrip(eframe 持久化用 code 字符串)。
    #[test]
    fn serde_style_roundtrip_via_code() {
        for lang in Lang::ALL {
            assert_eq!(lang.code().parse::<Lang>().unwrap(), lang);
        }
    }
}
