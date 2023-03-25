use crate::config::get_config;
use crate::selection::get_selection_text;
use crate::StringWrapper;
use crate::APP;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tauri::PhysicalPosition;
use tauri::{AppHandle, Manager, Window, WindowEvent};
use toml::Value;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use window_shadows::set_shadow;

pub fn build_window(label: &str, title: &str, handle: &AppHandle) -> Result<Window, String> {
    let (width, height) = get_window_size();
    let (x, y) = get_mouse_location().unwrap();
    let builder = tauri::WindowBuilder::new(
        handle,
        label,
        tauri::WindowUrl::App("index_translator.html".into()),
    )
    .inner_size(width, height)
    .always_on_top(true)
    .focused(true)
    .title(title);

    #[cfg(target_os = "macos")]
    {
        let builder = builder
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true);
        let window = match label {
            "persistent" => builder.center().skip_taskbar(false).build().unwrap(),
            _ => builder
                .position(x as f64, y as f64)
                .skip_taskbar(true)
                .build()
                .unwrap(),
        };
        set_shadow(&window, true).unwrap_or_default();
        window.set_focus().unwrap();
        match label {
            "persistent" => {}
            _ => {
                window.on_window_event(on_lose_focus);
            }
        };
        Ok(window)
    }

    #[cfg(target_os = "windows")]
    {
        let builder = builder.decorations(false);
        let window = match label {
            "persistent" => builder.skip_taskbar(false).build().unwrap(),
            _ => builder.skip_taskbar(true).build().unwrap(),
        };
        set_shadow(&window, true).unwrap_or_default();
        window.set_focus().unwrap();

        match label {
            "persistent" => {
                window.center().unwrap();
            }
            _ => {
                window.on_window_event(on_lose_focus);
                window.set_position(PhysicalPosition::new(x, y)).unwrap();
            }
        };
        Ok(window)
    }

    #[cfg(target_os = "linux")]
    {
        let builder = builder.transparent(true).decorations(false);
        let window = match label {
            "persistent" => builder.skip_taskbar(false).build().unwrap(),
            _ => builder.skip_taskbar(true).build().unwrap(),
        };

        window.set_focus().unwrap();
        match label {
            "persistent" => {
                window.center().unwrap();
            }
            _ => {
                window.on_window_event(on_lose_focus);
                window.set_position(PhysicalPosition::new(x, y)).unwrap();
            }
        };
        Ok(window)
    }
}

// 获取默认窗口大小
fn get_window_size() -> (f64, f64) {
    let width: f64 = get_config("window_width", Value::from(400), APP.get().unwrap().state())
        .as_integer()
        .unwrap() as f64;
    let height: f64 = get_config(
        "window_height",
        Value::from(500),
        APP.get().unwrap().state(),
    )
    .as_integer()
    .unwrap() as f64;
    return (width, height);
}

// 失去焦点自动关闭窗口
// Gnome 下存在焦点捕获失败bug，windows下拖动窗口会失去焦点
// #[cfg(any(target_os = "macos", target_os = "linux"))]
fn on_lose_focus(event: &WindowEvent) {
    match event {
        WindowEvent::Focused(v) => {
            if !v {
                let handle = APP.get().unwrap();
                match handle.get_window("translator") {
                    Some(window) => {
                        window.close().unwrap();
                    }
                    None => {}
                }
                match handle.get_window("popclip") {
                    Some(window) => {
                        window.close().unwrap();
                    }
                    None => {}
                }
            }
        }
        _ => {}
    }
}

// 获取鼠标坐标
#[cfg(target_os = "linux")]
fn get_mouse_location() -> Result<(i32, i32), String> {
    use std::process::Command;
    let output: String = match Command::new("xdotool").arg("getmouselocation").output() {
        Ok(v) => String::from_utf8(v.stdout).unwrap(),
        Err(e) => return Err(format!("xdotool执行出错{}", e.to_string())),
    };
    let output: Vec<&str> = output.split_whitespace().collect();
    let x = output
        .get(0)
        .unwrap()
        .replace("x:", "")
        .parse::<i32>()
        .unwrap();
    let y = output
        .get(1)
        .unwrap()
        .replace("y:", "")
        .parse::<i32>()
        .unwrap();
    return Ok((x, y));
}

#[cfg(target_os = "windows")]
fn get_mouse_location() -> Result<(i32, i32), String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetDesktopWindow};
    let (width, height) = get_window_size();
    let mut point = POINT { x: 0, y: 0 };
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    unsafe {
        if GetCursorPos(&mut point).as_bool() {
            let mut x = point.x as f64;
            let mut y = point.y as f64;
            // 获取桌面窗口的句柄
            let hwnd = GetDesktopWindow();
            let dpi = GetDpiForWindow(hwnd) as f64;
            if GetWindowRect(hwnd, &mut rect).as_bool() {
                // 由于获取到的屏幕大小以及鼠标坐标为物理像素，所以需要转换
                if point.x as f64 + width * (dpi / 100.0) > (rect.right - rect.left) as f64 {
                    x = (rect.right - rect.left) as f64 - width * (dpi / 100.0);
                }
                if point.y as f64 + height * (dpi / 100.0) > (rect.bottom - rect.top) as f64 {
                    y = (rect.bottom - rect.top) as f64 - height * (dpi / 100.0);
                }
            }
            return Ok((x as i32, y as i32));
        } else {
            return Err("error".to_string());
        }
    }
}

#[cfg(target_os = "macos")]
fn get_mouse_location() -> Result<(i32, i32), String> {
    use core_graphics::display::CGDisplay;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let display = CGDisplay::main();
    let mode = display.display_mode().unwrap();
    let event =
        CGEvent::new(CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap());
    let point = event.unwrap().location();
    let mut x = point.x;
    let mut y = point.y;
    let (width, height) = get_window_size();
    if point.x + width > mode.width() as f64 {
        x = mode.width() as f64 - width;
    }
    if point.y + height > mode.height() as f64 {
        y = mode.height() as f64 - height;
    }
    return Ok((x as i32, y as i32));
}

// 划词翻译
pub fn translate_window() {
    // 获取选择文本
    let text = get_selection_text().unwrap();
    let handle = APP.get().unwrap();
    // 写入状态备用
    let state: tauri::State<StringWrapper> = handle.state();
    state.0.lock().unwrap().replace_range(.., &text);
    // 创建窗口
    match handle.get_window("translator") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _window = build_window("translator", "Translator", handle).unwrap();
        }
    };
}

// 持久窗口
pub fn persistent_window() {
    let handle = APP.get().unwrap();
    match handle.get_window("persistent") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _window = build_window("persistent", "Persistent", handle).unwrap();
        }
    };
}

// popclip划词翻译
pub fn popclip_window(text: String) {
    let handle = APP.get().unwrap();

    let state: tauri::State<StringWrapper> = handle.state();
    state.0.lock().unwrap().replace_range(.., &text);

    match handle.get_window("popclip") {
        Some(window) => {
            window.close().unwrap();
        }
        None => {
            let _window = build_window("popclip", "PopClip", handle).unwrap();
        }
    };
}