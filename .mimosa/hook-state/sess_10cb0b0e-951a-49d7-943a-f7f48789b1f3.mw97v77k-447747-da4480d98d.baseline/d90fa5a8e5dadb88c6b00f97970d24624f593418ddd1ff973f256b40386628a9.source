//! linop-term: 远端 shell 输出的终端仿真与 ratatui 渲染。
//!
//! 对应 navop 的 `terminal` crate:`Terminal` 内部持有 vt100 解析器,
//! 吞入远端输出字节流,解析为屏幕网格;渲染时逐格映射为 ratatui
//! 单元格(颜色/属性/宽字符),并支持回滚浏览历史输出。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

/// 默认回滚缓冲容量(行)。
pub const SCROLLBACK_CAPACITY: usize = 10_000;

fn nonzero(v: u16) -> std::num::NonZeroU16 {
    std::num::NonZeroU16::new(v).unwrap_or(std::num::NonZeroU16::new(1).unwrap())
}

/// 一个仿真终端:解析输出、维护屏幕状态、渲染为 ratatui Widget。
pub struct Terminal {
    parser: vt100::Parser,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: vt100::Parser::new(nonzero(rows), nonzero(cols), SCROLLBACK_CAPACITY),
        }
    }

    /// 处理一批远端输出字节。
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    /// 终端尺寸变化(触发 vt100 重排,并需上层调用 shell.resize)。
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.parser.set_size(nonzero(rows), nonzero(cols));
    }

    pub fn cols(&self) -> u16 {
        self.parser.screen().size().1.get()
    }

    pub fn rows(&self) -> u16 {
        self.parser.screen().size().0.get()
    }

    /// 光标位置 `(col, row)`;远端隐藏光标时返回 `None`。
    pub fn cursor(&self) -> Option<(u16, u16)> {
        if self.parser.screen().hide_cursor() {
            return None;
        }
        let (row, col) = self.parser.screen().cursor_position();
        Some((col, row))
    }

    /// 可见区纯文本(按行拼接,不含属性)。
    pub fn text(&self) -> String {
        self.parser.screen().contents()
    }

    /// 回滚浏览:偏移 0 表示跟随最新输出;超出容量的值会被钳制。
    pub fn set_scrollback(&mut self, lines: usize) {
        self.parser.screen_mut().set_scrollback(lines);
    }

    /// 当前回滚偏移(0 = 最新)。
    pub fn scrollback_offset(&self) -> usize {
        self.parser.screen().scrollback()
    }

    /// 回滚区总行数。
    pub fn scrollback_len(&self) -> usize {
        self.parser.screen().scrollback_len()
    }

    /// 相对滚动回滚视图(负值向新滚动)。
    pub fn scroll(&mut self, delta: i32) {
        let cur = self.scrollback_offset() as i32;
        let max = self.scrollback_len() as i32;
        let target = (cur + delta).clamp(0, max);
        self.set_scrollback(target as usize);
    }

    /// 远端是否启用 application cursor 模式(影响方向键编码)。
    pub fn application_cursor(&self) -> bool {
        self.parser.screen().application_cursor()
    }

    /// 远端是否启用 bracketed paste 模式。
    pub fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }
}

impl Widget for &Terminal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let screen = self.parser.screen();
        for y in 0..area.height {
            for x in 0..area.width {
                let Some(cell) = screen.cell(y, x) else {
                    continue;
                };
                if let Some(buf_cell) = buf.cell_mut((area.x + x, area.y + y)) {
                    if cell.is_wide_continuation() {
                        // 宽字符续格:置空 symbol,让 ratatui 把它视为被覆盖格。
                        buf_cell.set_symbol("");
                    } else {
                        let contents = cell.contents();
                        buf_cell
                            .set_symbol(if contents.is_empty() { " " } else { contents })
                            .set_style(style_of(cell));
                    }
                }
            }
        }
    }
}

fn style_of(cell: &vt100::Cell) -> Style {
    let mut modifiers = Modifier::empty();
    if cell.bold() {
        modifiers |= Modifier::BOLD;
    }
    if cell.dim() {
        modifiers |= Modifier::DIM;
    }
    if cell.italic() {
        modifiers |= Modifier::ITALIC;
    }
    if cell.underline() {
        modifiers |= Modifier::UNDERLINED;
    }
    if cell.inverse() {
        modifiers |= Modifier::REVERSED;
    }
    Style::new()
        .fg(convert_color(cell.fgcolor()))
        .bg(convert_color(cell.bgcolor()))
        .add_modifier(modifiers)
}

/// vt100 颜色 → ratatui 颜色。索引色原样保留交给前端终端解释。
fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Position;

    fn render(term: &Terminal) -> Buffer {
        let area = Rect::new(0, 0, term.cols(), term.rows());
        let mut buf = Buffer::empty(area);
        Widget::render(term, area, &mut buf);
        buf
    }

    fn row_text(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf.cell(Position::new(x, y)).unwrap().symbol().to_string())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn renders_text_and_tracks_cursor() {
        let mut term = Terminal::new(20, 5);
        term.feed(b"hello");
        let buf = render(&term);
        assert_eq!(row_text(&buf, 0), "hello");
        assert_eq!(term.cursor(), Some((5, 0)));
    }

    #[test]
    fn newline_carriage_return_and_wrap() {
        let mut term = Terminal::new(10, 3);
        term.feed(b"aaa\r\nbbb\r\nccc");
        assert_eq!(term.text(), "aaa\nbbb\nccc");
        assert_eq!(term.cursor(), Some((3, 2)));
    }

    #[test]
    fn colors_and_attributes_map_to_style() {
        let mut term = Terminal::new(10, 1);
        term.feed(b"\x1b[1;31mR\x1b[0mN");
        let buf = render(&term);
        let red = buf.cell(Position::new(0, 0)).unwrap();
        assert_eq!(red.fg, Color::Indexed(1));
        assert!(red.modifier.contains(Modifier::BOLD));
        let plain = buf.cell(Position::new(1, 0)).unwrap();
        assert_eq!(plain.fg, Color::Reset);
        assert!(!plain.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn hide_cursor_reports_none() {
        let mut term = Terminal::new(10, 2);
        assert_eq!(term.cursor(), Some((0, 0)));
        term.feed(b"\x1b[?25l");
        assert_eq!(term.cursor(), None);
        term.feed(b"\x1b[?25h");
        assert_eq!(term.cursor(), Some((0, 0)));
    }

    #[test]
    fn resize_keeps_contents() {
        let mut term = Terminal::new(10, 3);
        term.feed(b"0123456789");
        term.resize(20, 2);
        assert_eq!((term.cols(), term.rows()), (20, 2));
        assert!(term.text().contains("0123456789"));
    }

    #[test]
    fn scrollback_shows_history() {
        let mut term = Terminal::new(10, 3);
        // 5 行输出,前两行进入回滚区(末尾不发换行,避免底行再滚一行)。
        term.feed(b"one\r\ntwo\r\nthree\r\nfour\r\nfive");
        assert_eq!(term.scrollback_len(), 2);

        let buf = render(&term);
        assert_eq!(row_text(&buf, 0), "three");

        term.set_scrollback(2);
        let buf = render(&term);
        assert_eq!(row_text(&buf, 0), "one");

        term.scroll(-1);
        assert_eq!(term.scrollback_offset(), 1);
        let buf = render(&term);
        assert_eq!(row_text(&buf, 0), "two");

        term.set_scrollback(usize::MAX);
        assert_eq!(term.scrollback_offset(), 2);
    }

    #[test]
    fn terminal_modes_reported() {
        let mut term = Terminal::new(10, 2);
        assert!(!term.application_cursor());
        term.feed(b"\x1b[?1h");
        assert!(term.application_cursor());
        assert!(!term.bracketed_paste());
        term.feed(b"\x1b[?2004h");
        assert!(term.bracketed_paste());
    }

    #[test]
    fn wide_char_renders() {
        let mut term = Terminal::new(10, 1);
        term.feed("中文".as_bytes());
        let buf = render(&term);
        assert_eq!(row_text(&buf, 0), "中文");
    }
}
