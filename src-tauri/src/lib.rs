// Tauri 连点器核心逻辑 - 仅支持 Windows 平台
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use tauri::{AppHandle, State, Emitter, Manager};

// ===== 数据结构 =====

pub struct ClickerState {
    pub is_running: Arc<AtomicBool>,
    pub interval_ms: Arc<AtomicU64>,
    pub button: Arc<AtomicU8>,      // 0: 左键, 1: 右键, 2: 中键
    pub click_type: Arc<AtomicU8>,  // 0: 单击, 1: 双击
    pub current_hotkey: Mutex<String>,
}

// ===== Windows 专有鼠标模拟实现 =====

#[cfg(target_os = "windows")]
mod mouse_impl {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
        MOUSEINPUT,
    };

    pub fn send_click(button: u8, is_down: bool) {
        let flag = match (button, is_down) {
            (0, true)  => MOUSEEVENTF_LEFTDOWN,
            (0, false) => MOUSEEVENTF_LEFTUP,
            (1, true)  => MOUSEEVENTF_RIGHTDOWN,
            (1, false) => MOUSEEVENTF_RIGHTUP,
            (2, true)  => MOUSEEVENTF_MIDDLEDOWN,
            (2, false) => MOUSEEVENTF_MIDDLEUP,
            _          => MOUSEEVENTF_LEFTDOWN,
        };

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(1, &input as *const INPUT, std::mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn click_once(button: u8) {
        send_click(button, true);
        std::thread::sleep(std::time::Duration::from_millis(5));
        send_click(button, false);
    }

    pub fn click_double(button: u8) {
        click_once(button);
        std::thread::sleep(std::time::Duration::from_millis(30));
        click_once(button);
    }
}

// 非 Windows 平台空实现（仅为通过编译检查）
#[cfg(not(target_os = "windows"))]
mod mouse_impl {
    pub fn click_once(_button: u8) {}
    pub fn click_double(_button: u8) {}
}

// ===== Tauri Commands =====

#[tauri::command]
fn start_clicker(app: AppHandle, state: State<'_, ClickerState>) {
    state.is_running.store(true, Ordering::SeqCst);
    let _ = app.emit("clicker-status-changed", true);
}

#[tauri::command]
fn stop_clicker(app: AppHandle, state: State<'_, ClickerState>) {
    state.is_running.store(false, Ordering::SeqCst);
    let _ = app.emit("clicker-status-changed", false);
}

#[tauri::command]
fn update_settings(
    state: State<'_, ClickerState>,
    interval: u64,
    button: u8,
    click_type: u8,
) {
    state.interval_ms.store(interval, Ordering::SeqCst);
    state.button.store(button, Ordering::SeqCst);
    state.click_type.store(click_type, Ordering::SeqCst);
}

#[tauri::command]
fn get_status(state: State<'_, ClickerState>) -> bool {
    state.is_running.load(Ordering::SeqCst)
}

#[tauri::command]
fn register_hotkey(
    app: AppHandle,
    state: State<'_, ClickerState>,
    hotkey: String,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let shortcut_manager = app.global_shortcut();

    // 注销旧快捷键（忽略错误，例如第一次还没注册时）
    let mut current = state.current_hotkey.lock().unwrap();
    let _ = shortcut_manager.unregister(current.as_str());

    // 注册新快捷键
    shortcut_manager
        .register(hotkey.as_str())
        .map_err(|e| format!("快捷键注册失败: {}", e))?;

    // 保存新快捷键
    *current = hotkey;

    Ok(())
}

// ===== 主入口 =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let is_running = Arc::new(AtomicBool::new(false));
    let interval_ms = Arc::new(AtomicU64::new(100)); // 默认 100ms
    let button = Arc::new(AtomicU8::new(0));         // 默认左键
    let click_type = Arc::new(AtomicU8::new(0));     // 默认单击

    let state = ClickerState {
        is_running: is_running.clone(),
        interval_ms: interval_ms.clone(),
        button: button.clone(),
        click_type: click_type.clone(),
        current_hotkey: Mutex::new("F8".to_string()),
    };

    // 启动后台连点线程
    {
        let is_running = is_running.clone();
        let interval_ms = interval_ms.clone();
        let button = button.clone();
        let click_type = click_type.clone();

        std::thread::spawn(move || loop {
            if is_running.load(Ordering::SeqCst) {
                let btn = button.load(Ordering::SeqCst);
                let typ = click_type.load(Ordering::SeqCst);

                if typ == 0 {
                    mouse_impl::click_once(btn);
                } else {
                    mouse_impl::click_double(btn);
                }

                let ms = interval_ms.load(Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(ms));
            } else {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
    }

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() == ShortcutState::Pressed {
                        let clicker = app.state::<ClickerState>();
                        let current = clicker.is_running.load(Ordering::SeqCst);
                        let next = !current;
                        clicker.is_running.store(next, Ordering::SeqCst);
                        let _ = app.emit("clicker-status-changed", next);
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            // 启动时注册默认 F8 全局快捷键
            let _ = app.global_shortcut().register("F8");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_clicker,
            stop_clicker,
            update_settings,
            get_status,
            register_hotkey
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
