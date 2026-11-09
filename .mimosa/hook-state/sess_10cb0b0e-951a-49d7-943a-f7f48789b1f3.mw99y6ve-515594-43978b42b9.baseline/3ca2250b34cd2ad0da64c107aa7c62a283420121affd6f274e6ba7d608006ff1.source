//! crossterm 按键 → 远端 PTY 字节序列。
//!
//! 覆盖常规输入所需子集:可打印字符与 UTF-8、Ctrl/Alt 组合、
//! 方向键/编辑键(含修饰参数形式)、功能键、括号粘贴。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// xterm CSI 修饰键参数:1 + shift(1) + alt(2) + ctrl(4);无修饰返回 None。
fn modifier_param(modifiers: KeyModifiers) -> Option<u8> {
    let mut param = 1u8;
    if modifiers.contains(KeyModifiers::SHIFT) {
        param += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        param += 2;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        param += 4;
    }
    (param > 1).then_some(param)
}

/// 把按键编码为发给远端 PTY 的字节;没有对应序列时(如仅按修饰键)返回 None。
pub fn encode(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    let KeyEvent {
        code, modifiers, ..
    } = key;
    let param = modifier_param(modifiers);
    match code {
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),
        KeyCode::Esc => Some(b"\x1b".to_vec()),
        KeyCode::Char(c) => encode_char(c, modifiers),
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            let final_char = match code {
                KeyCode::Up => 'A',
                KeyCode::Down => 'B',
                KeyCode::Right => 'C',
                _ => 'D',
            };
            Some(match param {
                None if application_cursor => format!("\x1bO{final_char}").into_bytes(),
                None => format!("\x1b[{final_char}").into_bytes(),
                Some(m) => format!("\x1b[1;{m}{final_char}").into_bytes(),
            })
        }
        KeyCode::Home => Some(letter_key(param, 'H')),
        KeyCode::End => Some(letter_key(param, 'F')),
        KeyCode::Insert => Some(tilde_key(param, 2)),
        KeyCode::Delete => Some(tilde_key(param, 3)),
        KeyCode::PageUp => Some(tilde_key(param, 5)),
        KeyCode::PageDown => Some(tilde_key(param, 6)),
        KeyCode::F(n) => encode_fkey(n, param),
        KeyCode::Null
        | KeyCode::CapsLock
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => None,
        _ => None,
    }
}

/// 字母结尾的 CSI 键(Home/End):无修饰 `\e[H`,带修饰 `\e[1;mH`。
fn letter_key(param: Option<u8>, final_char: char) -> Vec<u8> {
    match param {
        None => format!("\x1b[{final_char}").into_bytes(),
        Some(m) => format!("\x1b[1;{m}{final_char}").into_bytes(),
    }
}

/// 数字~结尾的 CSI 键:无修饰 `\e[5~`,带修饰 `\e[5;m~`。
fn tilde_key(param: Option<u8>, num: u8) -> Vec<u8> {
    match param {
        None => format!("\x1b[{num}~").into_bytes(),
        Some(m) => format!("\x1b[{num};{m}~").into_bytes(),
    }
}

/// 功能键:F1-F4 为 SS3 形式,F5-F12 为数字~形式。
fn encode_fkey(n: u8, param: Option<u8>) -> Option<Vec<u8>> {
    const SS3: [char; 4] = ['P', 'Q', 'R', 'S'];
    const TILDE: [u8; 8] = [15, 17, 18, 19, 20, 21, 23, 24];
    Some(match n {
        1..=4 => match param {
            None => format!("\x1bO{}", SS3[(n - 1) as usize]).into_bytes(),
            Some(m) => format!("\x1b[1;{m}{}", SS3[(n - 1) as usize]).into_bytes(),
        },
        5..=12 => tilde_key(param, TILDE[(n - 5) as usize]),
        _ => return None,
    })
}

fn encode_char(c: char, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let alt = modifiers.contains(KeyModifiers::ALT);
    if ctrl {
        // Ctrl 组合映射为控制字符;crossterm 已把 Ctrl+字母归一为小写。
        let byte = match c.to_ascii_lowercase() {
            '@' | ' ' => 0x00,
            'a'..='z' => c.to_ascii_lowercase() as u8 - b'a' + 1,
            '[' => 0x1b,
            '\\' => 0x1c,
            ']' => 0x1d,
            '^' => 0x1e,
            '_' => 0x1f,
            '?' => 0x7f,
            _ => return None,
        };
        return Some(vec![byte]);
    }
    let mut bytes = c.to_string().into_bytes();
    if alt {
        let mut prefixed = vec![0x1b];
        prefixed.append(&mut bytes);
        return Some(prefixed);
    }
    Some(bytes)
}

/// 粘贴文本编码:bracketed 模式包裹 200~/201~,并把 `\n` 归一为 `\r`。
pub fn paste(text: &str, bracketed: bool) -> Vec<u8> {
    let text = text.replace('\n', "\r");
    if bracketed {
        format!("\x1b[200~{text}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn printable_chars() {
        assert_eq!(
            encode(key(KeyCode::Char('a'), KeyModifiers::NONE), false),
            Some(b"a".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::Char('A'), KeyModifiers::SHIFT), false),
            Some(b"A".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::Char('中'), KeyModifiers::NONE), false),
            Some("中".as_bytes().to_vec())
        );
    }

    #[test]
    fn control_chars() {
        assert_eq!(
            encode(key(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            Some(vec![0x03])
        );
        assert_eq!(
            encode(key(KeyCode::Char('C'), KeyModifiers::CONTROL), false),
            Some(vec![0x03])
        );
        assert_eq!(
            encode(key(KeyCode::Char(' '), KeyModifiers::CONTROL), false),
            Some(vec![0x00])
        );
        assert_eq!(
            encode(key(KeyCode::Char('x'), KeyModifiers::ALT), false),
            Some(b"\x1bx".to_vec())
        );
    }

    #[test]
    fn editing_keys() {
        assert_eq!(
            encode(key(KeyCode::Enter, KeyModifiers::NONE), false),
            Some(b"\r".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::Backspace, KeyModifiers::NONE), false),
            Some(b"\x7f".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::Home, KeyModifiers::NONE), false),
            Some(b"\x1b[H".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::Delete, KeyModifiers::NONE), false),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::Home, KeyModifiers::CONTROL), false),
            Some(b"\x1b[1;5H".to_vec())
        );
    }

    #[test]
    fn arrows_respect_cursor_mode() {
        let up = key(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(encode(up, false), Some(b"\x1b[A".to_vec()));
        assert_eq!(encode(up, true), Some(b"\x1bOA".to_vec()));
        let right = key(KeyCode::Right, KeyModifiers::CONTROL);
        assert_eq!(encode(right, false), Some(b"\x1b[1;5C".to_vec()));
    }

    #[test]
    fn function_keys() {
        assert_eq!(
            encode(key(KeyCode::F(1), KeyModifiers::NONE), false),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::F(5), KeyModifiers::NONE), false),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(
            encode(key(KeyCode::F(12), KeyModifiers::NONE), false),
            Some(b"\x1b[24~".to_vec())
        );
    }

    #[test]
    fn paste_modes() {
        assert_eq!(paste("a\nb", false), b"a\rb".to_vec());
        assert_eq!(paste("x", true), b"\x1b[200~x\x1b[201~".to_vec());
    }
}
