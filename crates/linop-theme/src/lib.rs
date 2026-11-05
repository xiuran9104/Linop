//! linop-theme:Linop 前端共享的主题令牌。
//!
//! 纯数据(不依赖任何 GUI 框架):每个主题给出一组颜色令牌,
//! 由各前端(egui/GPUI/…)映射到自己的样式系统。测试保证令牌
//! 完整与基本可读性(前景/背景对比)。

/// 24-bit RGB 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// 相对亮度(ITU-R BT.709 加权,0-1)。
    pub fn luminance(&self) -> f64 {
        (0.2126 * self.r as f64 + 0.7152 * self.g as f64 + 0.0722 * self.b as f64) / 255.0
    }
}

/// 主题标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Theme {
    Dark,
    Light,
    Nord,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Nord];

    pub fn id(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Nord => "nord",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            "nord" => Some(Theme::Nord),
            _ => None,
        }
    }

    pub fn tokens(&self) -> Tokens {
        match self {
            Theme::Dark => DARK,
            Theme::Light => LIGHT,
            Theme::Nord => NORD,
        }
    }
}

/// 主题颜色令牌。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub name: &'static str,
    /// 窗口/面板背景。
    pub bg: Rgb,
    /// 更深一层的面板背景(侧栏/底栏)。
    pub panel_bg: Rgb,
    /// 主前景(文本)。
    pub fg: Rgb,
    /// 次级文本。
    pub fg_muted: Rgb,
    /// 强调色(选中态/主按钮)。
    pub accent: Rgb,
    /// 强调色上的文字。
    pub accent_fg: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub danger: Rgb,
    /// 边框/分隔线。
    pub border: Rgb,
    /// 文本选区背景。
    pub selection: Rgb,
}

const DARK: Tokens = Tokens {
    name: "dark",
    bg: Rgb::new(24, 24, 28),
    panel_bg: Rgb::new(18, 18, 22),
    fg: Rgb::new(224, 224, 228),
    fg_muted: Rgb::new(140, 140, 152),
    accent: Rgb::new(64, 156, 255),
    accent_fg: Rgb::new(10, 18, 26),
    success: Rgb::new(80, 200, 120),
    warning: Rgb::new(230, 180, 60),
    danger: Rgb::new(235, 90, 90),
    border: Rgb::new(56, 56, 64),
    selection: Rgb::new(44, 84, 128),
};

const LIGHT: Tokens = Tokens {
    name: "light",
    bg: Rgb::new(246, 246, 248),
    panel_bg: Rgb::new(236, 236, 240),
    fg: Rgb::new(32, 32, 38),
    fg_muted: Rgb::new(110, 110, 122),
    accent: Rgb::new(24, 110, 220),
    accent_fg: Rgb::new(255, 255, 255),
    success: Rgb::new(30, 150, 80),
    warning: Rgb::new(190, 140, 20),
    danger: Rgb::new(200, 60, 60),
    border: Rgb::new(205, 205, 212),
    selection: Rgb::new(180, 208, 240),
};

const NORD: Tokens = Tokens {
    name: "nord",
    bg: Rgb::new(46, 52, 64),          // nord0
    panel_bg: Rgb::new(41, 47, 58),    // darker polar night
    fg: Rgb::new(216, 222, 233),       // nord4
    fg_muted: Rgb::new(143, 155, 175), // nord3
    accent: Rgb::new(136, 192, 208),   // nord7
    accent_fg: Rgb::new(29, 32, 40),
    success: Rgb::new(163, 190, 140),  // nord14
    warning: Rgb::new(235, 203, 139),  // nord13
    danger: Rgb::new(191, 97, 106),    // nord11
    border: Rgb::new(59, 66, 82),      // nord1
    selection: Rgb::new(94, 129, 172), // nord10
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        for theme in Theme::ALL {
            assert_eq!(Theme::parse(theme.id()), Some(theme));
        }
        assert_eq!(Theme::parse("DARK"), Some(Theme::Dark));
        assert_eq!(Theme::parse("nope"), None);
    }

    /// 可读性下限:前景/背景亮度差、按钮文字/强调色亮度差。
    #[test]
    fn tokens_have_minimum_contrast() {
        for theme in Theme::ALL {
            let t = theme.tokens();
            assert!(
                (t.fg.luminance() - t.bg.luminance()).abs() > 0.25,
                "{} fg/bg 亮度差过小:{} vs {}",
                t.name,
                t.fg.luminance(),
                t.bg.luminance()
            );
            assert!(
                (t.accent_fg.luminance() - t.accent.luminance()).abs() > 0.25,
                "{} accent 对比不足",
                t.name
            );
            // 三色语义互不相同。
            assert_ne!(t.success, t.danger);
            assert_ne!(t.warning, t.danger);
        }
    }

    #[test]
    fn nord_uses_nord_palette_anchors() {
        let t = Theme::Nord.tokens();
        assert_eq!(t.bg, Rgb::new(46, 52, 64), "应为 nord0");
        assert_eq!(t.accent, Rgb::new(136, 192, 208), "应为 nord7");
    }
}
