#![windows_subsystem = "windows"]

mod capture;
mod input;

use rand::Rng;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WM_TRAY: u32 = WM_APP + 1;
const WM_STATUS: u32 = WM_USER;
const WM_PIN: u32 = WM_USER + 1;
const WM_STOP: u32 = WM_USER + 2;
const ID_PIN: i32 = 101;
const ID_STATUS: i32 = 102;
const ID_BTN_STOP: i32 = 103;
const SS_CENTER: u32 = 0x0001;

struct SafeHwnd(HWND);
unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

fn relay_url() -> String {
    std::env::var("XCONTROL_RELAY").unwrap_or_else(|_| {
        "ws://[2001:19f0:8000:385b:5400:06ff:fe43:eb83]/ws".to_string()
    })
}

fn gen_pin() -> String {
    let mut rng = rand::thread_rng();
    (0..5).map(|_| rng.gen_range(b'A'..=b'H') as char).collect()
}

fn wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn center_window(hwnd: HWND, w: i32, h: i32) {
    let mut rect = std::mem::zeroed::<RECT>();
    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect as *mut _ as _, 0);
    let x = (rect.right - rect.left - w) / 2 + rect.left;
    let y = (rect.bottom - rect.top - h) / 2 + rect.top;
    SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOZORDER);
}

unsafe fn add_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = LoadImageW(std::ptr::null_mut(), IDI_APPLICATION as *mut _, IMAGE_ICON, 16, 16, LR_SHARED) as HICON;
    let tip = wstr("X-Control Agent");
    for (i, &c) in tip.iter().enumerate().take(127) {
        let s: &mut u16 = &mut nid.szTip[i];
        *s = c;
    }
    Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn remove_tray(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let h = GetModuleHandleW(std::ptr::null());
            let bold = CreateFontW(-32, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32,
                DEFAULT_QUALITY as u32, DEFAULT_PITCH as u32 | FF_DONTCARE as u32,
                wstr("Consolas").as_ptr());
            let norm = CreateFontW(-14, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
                DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32,
                DEFAULT_QUALITY as u32, DEFAULT_PITCH as u32 | FF_DONTCARE as u32,
                wstr("Segoe UI").as_ptr());

            CreateWindowExW(0, wstr("STATIC").as_ptr(), wstr("Your PIN:").as_ptr(),
                WS_CHILD | WS_VISIBLE | SS_CENTER, 20, 15, 280, 18, hwnd, std::ptr::null_mut(), h, std::ptr::null_mut());
            let pin = CreateWindowExW(0, wstr("STATIC").as_ptr(), wstr("------").as_ptr(),
                WS_CHILD | WS_VISIBLE | SS_CENTER, 20, 38, 280, 40, hwnd, ID_PIN as _, h, std::ptr::null_mut());
            SendMessageW(pin, WM_SETFONT, bold as _, 1);
            let st = CreateWindowExW(0, wstr("STATIC").as_ptr(), wstr("Starting...").as_ptr(),
                WS_CHILD | WS_VISIBLE | SS_CENTER, 20, 85, 280, 18, hwnd, ID_STATUS as _, h, std::ptr::null_mut());
            SendMessageW(st, WM_SETFONT, norm as _, 1);
            CreateWindowExW(0, wstr("BUTTON").as_ptr(), wstr("Stop").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                120, 115, 80, 28, hwnd, ID_BTN_STOP as _, h, std::ptr::null_mut());
            add_tray(hwnd);
            center_window(hwnd, 320, 175);
        }
        WM_COMMAND => {
            if (wparam as u32 & 0xFFFF) as i32 == ID_BTN_STOP {
                remove_tray(hwnd);
                DestroyWindow(hwnd);
            }
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
        }
        WM_DESTROY => {
            remove_tray(hwnd);
            PostQuitMessage(0);
        }
        WM_TRAY => {
            let ev = (lparam as u32 & 0xFFFF) as u32;
            if ev == WM_LBUTTONDBLCLK {
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            } else if ev == WM_RBUTTONUP {
                let mut pt = std::mem::zeroed::<POINT>();
                GetCursorPos(&mut pt);
                let hmenu = CreatePopupMenu();
                AppendMenuW(hmenu, MF_STRING, 1, wstr("Show").as_ptr());
                AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(hmenu, MF_STRING, 2, wstr("Quit").as_ptr());
                SetForegroundWindow(hwnd);
                let cmd = TrackPopupMenu(hmenu, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, std::ptr::null_mut());
                DestroyMenu(hmenu);
                match cmd {
                    1 => { ShowWindow(hwnd, SW_SHOW); SetForegroundWindow(hwnd); }
                    2 => { remove_tray(hwnd); DestroyWindow(hwnd); }
                    _ => {}
                }
            }
        }
        WM_STATUS => {
            let s = STATUS.lock().unwrap();
            let child = GetDlgItem(hwnd, ID_STATUS);
            if !child.is_null() {
                let w = wstr(&s);
                SetWindowTextW(child, w.as_ptr());
            }
        }
        WM_PIN => {
            let p = PIN.lock().unwrap();
            let child = GetDlgItem(hwnd, ID_PIN);
            if !child.is_null() {
                let w = wstr(&p);
                SetWindowTextW(child, w.as_ptr());
            }
        }
        WM_STOP => {
            remove_tray(hwnd);
            DestroyWindow(hwnd);
        }
        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }
    0
}

lazy_static::lazy_static! {
    static ref STATUS: Mutex<String> = Mutex::new(String::new());
    static ref PIN: Mutex<String> = Mutex::new(String::new());
    static ref RUNNING: AtomicBool = AtomicBool::new(true);
    static ref HWND_MAIN: Mutex<Option<SafeHwnd>> = Mutex::new(None);
}

fn main() {
    unsafe {
        let h = GetModuleHandleW(std::ptr::null());
        let class = wstr("XControlWindow");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = h;
        wc.hIcon = LoadImageW(std::ptr::null_mut(), IDI_APPLICATION as *mut _, IMAGE_ICON, 0, 0, LR_SHARED) as HICON;
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
        wc.lpszClassName = class.as_ptr();
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            0, class.as_ptr(), wstr("X-Control Agent").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), h, std::ptr::null_mut(),
        );
        *HWND_MAIN.lock().unwrap() = Some(SafeHwnd(hwnd));
        ShowWindow(hwnd, SW_SHOW);

        let pin = gen_pin();
        *PIN.lock().unwrap() = pin.clone();
        *STATUS.lock().unwrap() = "Starting...".into();
        post_msg(WM_PIN);
        post_msg(WM_STATUS);

        let url = relay_url();
        std::thread::spawn(move || worker_thread(url, pin));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn worker_thread(url: String, pin: String) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        loop {
            if !RUNNING.load(Ordering::Relaxed) { break; }
            *STATUS.lock().unwrap() = "Connecting...".into();
            post_msg(WM_STATUS);
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws, _)) => {
                    *STATUS.lock().unwrap() = format!("Connected. PIN: {}", pin);
                    post_msg(WM_STATUS);
                    if let Err(e) = run_session(ws, &pin).await {
                        *STATUS.lock().unwrap() = format!("Disconnected: {}", e);
                        post_msg(WM_STATUS);
                    }
                }
                Err(e) => {
                    *STATUS.lock().unwrap() = format!("Failed: {}. Retrying...", e);
                    post_msg(WM_STATUS);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

fn post_msg(msg: u32) {
    if let Some(ref hwnd) = *HWND_MAIN.lock().unwrap() {
        unsafe { PostMessageW(hwnd.0, msg, 0, 0); }
    }
}

async fn run_session(
    ws: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    pin: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite;

    let (mut sender, mut receiver) = ws.split();

    sender.send(tungstenite::Message::Text(
        json!({"type":"register","pin":pin}).to_string().into(),
    )).await?;

    let mut registered = false;
    let timeout = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&text) {
                            if resp.get("type").and_then(|v| v.as_str()) == Some("registered") {
                                registered = true;
                                *STATUS.lock().unwrap() = "Waiting for viewer...".into();
                                post_msg(WM_STATUS);
                                break;
                            }
                        }
                    }
                    _ => return Err("Connection lost".into()),
                }
            }
            _ = &mut timeout => return Err("Registration timeout".into()),
        }
    }

    if !registered { return Err("Registration failed".into()); }

    let sender = Arc::new(tokio::sync::Mutex::new(sender));
    let s = sender.clone();

    let input_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let tungstenite::Message::Text(text) = &msg {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
                    if v.get("type").and_then(|v| v.as_str()) == Some("input") {
                        handle_input(&v);
                        if v.get("event").and_then(|v| v.as_str()) == Some("mousedown") {
                            *STATUS.lock().unwrap() = "Connected - viewer active".into();
                            post_msg(WM_STATUS);
                        }
                    }
                }
            }
        }
    });

    loop {
        if let Some(jpeg) = capture::capture_screen() {
            let mut guard = s.lock().await;
            if guard.send(tungstenite::Message::Binary(jpeg.into())).await.is_err() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    input_handle.abort();
    Ok(())
}

fn handle_input(msg: &serde_json::Value) {
    let event = msg.get("event").and_then(|v| v.as_str()).unwrap_or("");
    let button = msg.get("button").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    match event {
        "mousedown" => {
            let x = msg.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            let y = msg.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            input::mouse_move(x, y);
            input::mouse_click_button(button, true);
        }
        "mouseup" => input::mouse_click_button(button, false),
        "mousemove" => {
            let x = msg.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            let y = msg.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            input::mouse_move(x, y);
        }
        "click" => {
            let x = msg.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            let y = msg.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as i32;
            input::mouse_move(x, y);
            input::mouse_click_button(button, true);
            std::thread::sleep(std::time::Duration::from_millis(50));
            input::mouse_click_button(button, false);
        }
        "scroll" => {
            let dy = msg.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            input::mouse_scroll(0, dy);
        }
        "keydown" => {
            if let Some(key) = msg.get("key").and_then(|v| v.as_str()) {
                input::key_press(key, true);
            }
        }
        "keyup" => {
            if let Some(key) = msg.get("key").and_then(|v| v.as_str()) {
                input::key_press(key, false);
            }
        }
        _ => {}
    }
}
