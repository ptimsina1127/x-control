use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use std::mem;

pub fn mouse_move(x: i32, y: i32) {
    unsafe {
        let mut input: INPUT = mem::zeroed();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx: x,
            dy: y,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
            time: 0,
            dwExtraInfo: 0,
        };
        SendInput(1, &mut input as *mut INPUT, mem::size_of::<INPUT>() as i32);
    }
}

pub fn mouse_click_button(button: u32, down: bool) {
    unsafe {
        let (down_flag, up_flag) = match button {
            2 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            1 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        };
        let mut input: INPUT = mem::zeroed();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: if down { down_flag } else { up_flag },
            time: 0,
            dwExtraInfo: 0,
        };
        SendInput(1, &mut input as *mut INPUT, mem::size_of::<INPUT>() as i32);
    }
}

pub fn mouse_scroll(_dx: i32, dy: i32) {
    unsafe {
        let mut input: INPUT = mem::zeroed();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: dy as u32,
            dwFlags: MOUSEEVENTF_WHEEL,
            time: 0,
            dwExtraInfo: 0,
        };
        SendInput(1, &mut input as *mut INPUT, mem::size_of::<INPUT>() as i32);
    }
}

pub fn key_press(key: &str, down: bool) {
    if let Some(vk) = char_to_vk(key) {
        unsafe {
            let mut input: INPUT = mem::zeroed();
            input.r#type = INPUT_KEYBOARD;
            let flags = if down { 0 } else { KEYEVENTF_KEYUP };
            input.Anonymous.ki = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &mut input as *mut INPUT, mem::size_of::<INPUT>() as i32);
        }
    }
}

fn char_to_vk(key: &str) -> Option<u16> {
    Some(match key {
        "ENTER" | "\r" => VK_RETURN,
        "BACKSPACE" | "\u{8}" => VK_BACK,
        "TAB" | "\t" => VK_TAB,
        "ESCAPE" => VK_ESCAPE,
        "SPACE" | " " => VK_SPACE,
        "DELETE" => VK_DELETE,
        "INSERT" => VK_INSERT,
        "SHIFT" => VK_SHIFT,
        "CONTROL" | "CTRL" => VK_CONTROL,
        "ALT" => VK_MENU,
        "CAPSLOCK" => VK_CAPITAL,
        "NUMLOCK" => VK_NUMLOCK,
        "SCROLLLOCK" => VK_SCROLL,
        "PRINTSCREEN" => VK_SNAPSHOT,
        "PAUSE" => VK_PAUSE,
        "UP" => VK_UP,
        "DOWN" => VK_DOWN,
        "LEFT" => VK_LEFT,
        "RIGHT" => VK_RIGHT,
        "HOME" => VK_HOME,
        "END" => VK_END,
        "PAGEUP" => VK_PRIOR,
        "PAGEDOWN" => VK_NEXT,
        "F1" => VK_F1,
        "F2" => VK_F2,
        "F3" => VK_F3,
        "F4" => VK_F4,
        "F5" => VK_F5,
        "F6" => VK_F6,
        "F7" => VK_F7,
        "F8" => VK_F8,
        "F9" => VK_F9,
        "F10" => VK_F10,
        "F11" => VK_F11,
        "F12" => VK_F12,
        "," | "<" => 0xBC,
        "." | ">" => 0xBE,
        ";" | ":" => 0xBA,
        "'" | "\"" => 0xDE,
        "[" | "{" => 0xDB,
        "]" | "}" => 0xDD,
        "\\" | "|" => 0xDC,
        "`" | "~" => 0xC0,
        "-" | "_" => 0xBD,
        "=" | "+" => 0xBB,
        "/" | "?" => 0xBF,
        "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M"
        | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
        | "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m"
        | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" => {
            key.to_ascii_uppercase().as_bytes()[0] as u16
        }
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => key.as_bytes()[0] as u16,
        _ => return None,
    })
}
