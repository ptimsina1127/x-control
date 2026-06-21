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

pub fn mouse_click(down: bool) {
    unsafe {
        let mut input: INPUT = mem::zeroed();
        input.r#type = INPUT_MOUSE;
        let flags = if down {
            MOUSEEVENTF_LEFTDOWN
        } else {
            MOUSEEVENTF_LEFTUP
        };
        input.Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: flags,
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
        "SHIFT" => VK_SHIFT,
        "CONTROL" | "CTRL" => VK_CONTROL,
        "ALT" => VK_MENU,
        "UP" => VK_UP,
        "DOWN" => VK_DOWN,
        "LEFT" => VK_LEFT,
        "RIGHT" => VK_RIGHT,
        "HOME" => VK_HOME,
        "END" => VK_END,
        "PAGEUP" => VK_PRIOR,
        "PAGEDOWN" => VK_NEXT,
        "CAPSLOCK" => VK_CAPITAL,
        "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M"
        | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" => {
            key.as_bytes()[0] as u16
        }
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => key.as_bytes()[0] as u16,
        _ => return None,
    })
}
