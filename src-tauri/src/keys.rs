//! Глобальный слушатель клавиш для push-to-talk (собственный, кроссплатформенный).
//!
//! Ключевые требования: ловить нажатия ГЛОБАЛЬНО (когда активно другое окно), поддерживать
//! ОДИНОЧНЫЕ клавиши и модификаторы (правый Shift), спрашивать разрешение ОДИН РАЗ и не
//! падать вне главного потока.
//!
//! macOS: `CGEventTap` (listen-only) на выделенном потоке с собственным CFRunLoop. В отличие
//! от rdev/device_query НЕ дёргает TSM (нет краша) и не пере-запрашивает доступ на каждом опросе.
//! Требует «Мониторинг ввода» (Input Monitoring). Windows: низкоуровневый хук `WH_KEYBOARD_LL`.
//!
//! Колбэк вызывается как `cb(canonical_name, is_down)`, где имя совпадает с тем, что шлёт фронтенд
//! (`ShiftRight`, `ControlLeft`, `KeyA`, `Num1`, `F5`, `Space`, …).

/// Запустить глобальный слушатель. Колбэк зовётся на каждое нажатие/отпускание клавиши.
#[cfg(target_os = "macos")]
pub fn listen<F: Fn(&str, bool) + Send + 'static>(cb: F) {
    std::thread::spawn(move || mac::run(cb));
}

#[cfg(target_os = "windows")]
pub fn listen<F: Fn(&str, bool) + Send + 'static>(cb: F) {
    std::thread::spawn(move || win::run(cb));
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn listen<F: Fn(&str, bool) + Send + 'static>(_cb: F) {}

/// Windows: авто-вставка Ctrl+V через SendInput с ВИРТУАЛЬНЫМИ клавишами. `enigo` шлёт
/// `Key::Unicode('v')` как `KEYEVENTF_UNICODE` — он не комбинируется с Ctrl (символ идёт
/// мимо виртуальных клавиш), поэтому там Ctrl+V не срабатывает. Здесь — честный VK_V.
#[cfg(target_os = "windows")]
pub fn paste_ctrl_v() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_CONTROL, VK_V,
    };
    fn ev(vk: VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
    let inputs = [
        ev(VK_CONTROL, false),
        ev(VK_V, false),
        ev(VK_V, true),
        ev(VK_CONTROL, true),
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

// ───────────────────────── macOS ─────────────────────────

#[cfg(target_os = "macos")]
mod mac {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        EventField,
    };

    pub fn run<F: Fn(&str, bool) + Send + 'static>(cb: F) {
        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::KeyDown,
                CGEventType::KeyUp,
                CGEventType::FlagsChanged,
            ],
            |_proxy, etype, event| {
                let keycode =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as i64;
                match etype {
                    CGEventType::KeyDown => {
                        if let Some(n) = key_name(keycode) {
                            cb(n, true);
                        }
                    }
                    CGEventType::KeyUp => {
                        if let Some(n) = key_name(keycode) {
                            cb(n, false);
                        }
                    }
                    CGEventType::FlagsChanged => {
                        if let Some((n, mask)) = mod_name(keycode) {
                            let down = event.get_flags().bits() & mask != 0;
                            cb(n, down);
                        }
                    }
                    _ => {}
                }
                None
            },
        );
        let Ok(tap) = tap else {
            eprintln!("keys: CGEventTap не создан — нужен доступ «Мониторинг ввода» (Input Monitoring)");
            return;
        };
        let Ok(source) = tap.mach_port.create_runloop_source(0) else {
            return;
        };
        let run_loop = CFRunLoop::get_current();
        unsafe {
            run_loop.add_source(&source, kCFRunLoopCommonModes);
        }
        tap.enable();
        CFRunLoop::run_current(); // блокирует этот поток навсегда
    }

    /// Виртуальный keycode macOS → каноническое имя (обычные клавиши).
    fn key_name(k: i64) -> Option<&'static str> {
        Some(match k {
            0 => "KeyA", 1 => "KeyS", 2 => "KeyD", 3 => "KeyF", 4 => "KeyH", 5 => "KeyG",
            6 => "KeyZ", 7 => "KeyX", 8 => "KeyC", 9 => "KeyV", 11 => "KeyB", 12 => "KeyQ",
            13 => "KeyW", 14 => "KeyE", 15 => "KeyR", 16 => "KeyY", 17 => "KeyT", 31 => "KeyO",
            32 => "KeyU", 34 => "KeyI", 35 => "KeyP", 37 => "KeyL", 38 => "KeyJ", 40 => "KeyK",
            45 => "KeyN", 46 => "KeyM",
            29 => "Num0", 18 => "Num1", 19 => "Num2", 20 => "Num3", 21 => "Num4", 23 => "Num5",
            22 => "Num6", 26 => "Num7", 28 => "Num8", 25 => "Num9",
            49 => "Space", 36 => "Return", 48 => "Tab", 53 => "Escape", 51 => "Backspace",
            122 => "F1", 120 => "F2", 99 => "F3", 118 => "F4", 96 => "F5", 97 => "F6",
            98 => "F7", 100 => "F8", 101 => "F9", 109 => "F10", 103 => "F11", 111 => "F12",
            123 => "LeftArrow", 124 => "RightArrow", 125 => "DownArrow", 126 => "UpArrow",
            _ => return None,
        })
    }

    /// Модификатор (keycode на FlagsChanged) → (имя, device-dependent маска в CGEventFlags).
    fn mod_name(k: i64) -> Option<(&'static str, u64)> {
        Some(match k {
            56 => ("ShiftLeft", 0x0000_0002),
            60 => ("ShiftRight", 0x0000_0004),
            59 => ("ControlLeft", 0x0000_0001),
            62 => ("ControlRight", 0x0000_2000),
            58 => ("Alt", 0x0000_0020),
            61 => ("AltGr", 0x0000_0040),
            55 => ("MetaLeft", 0x0000_0008),
            54 => ("MetaRight", 0x0000_0010),
            57 => ("CapsLock", 0x0001_0000),
            _ => return None,
        })
    }
}

// ───────────────────────── Windows ─────────────────────────

#[cfg(target_os = "windows")]
mod win {
    use std::sync::{Mutex, OnceLock};

    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    type Cb = Box<dyn Fn(&str, bool) + Send>;

    fn cb_slot() -> &'static Mutex<Option<Cb>> {
        static S: OnceLock<Mutex<Option<Cb>>> = OnceLock::new();
        S.get_or_init(|| Mutex::new(None))
    }

    pub fn run<F: Fn(&str, bool) + Send + 'static>(cb: F) {
        *cb_slot().lock().unwrap() = Some(Box::new(cb));
        unsafe {
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) {
                Ok(h) => h,
                Err(_) => {
                    eprintln!("keys: не удалось установить WH_KEYBOARD_LL");
                    return;
                }
            };
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
            let _ = hook; // хук снимется при завершении процесса
        }
    }

    const LLKHF_EXTENDED: u32 = 0x01;

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let m = wparam.0 as u32;
            let down = m == WM_KEYDOWN || m == WM_SYSKEYDOWN;
            let up = m == WM_KEYUP || m == WM_SYSKEYUP;
            if down || up {
                if let Some(n) = vk_name(kb.vkCode, kb.scanCode, kb.flags.0) {
                    if let Some(cb) = cb_slot().lock().unwrap().as_ref() {
                        cb(n, down);
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// VK-код Windows → каноническое имя (L/R модификаторы через scancode/extended-флаг).
    fn vk_name(vk: u32, scan: u32, flags: u32) -> Option<&'static str> {
        let ext = flags & LLKHF_EXTENDED != 0;
        Some(match vk {
            0x10 => {
                if scan == 0x36 {
                    "ShiftRight"
                } else {
                    "ShiftLeft"
                }
            }
            0xA0 => "ShiftLeft",
            0xA1 => "ShiftRight",
            0x11 => {
                if ext {
                    "ControlRight"
                } else {
                    "ControlLeft"
                }
            }
            0xA2 => "ControlLeft",
            0xA3 => "ControlRight",
            0x12 => {
                if ext {
                    "AltGr"
                } else {
                    "Alt"
                }
            }
            0xA4 => "Alt",
            0xA5 => "AltGr",
            0x5B => "MetaLeft",
            0x5C => "MetaRight",
            0x20 => "Space",
            0x0D => "Return",
            0x09 => "Tab",
            0x1B => "Escape",
            0x08 => "Backspace",
            0x25 => "LeftArrow",
            0x26 => "UpArrow",
            0x27 => "RightArrow",
            0x28 => "DownArrow",
            0x41..=0x5A => match vk {
                0x41 => "KeyA", 0x42 => "KeyB", 0x43 => "KeyC", 0x44 => "KeyD", 0x45 => "KeyE",
                0x46 => "KeyF", 0x47 => "KeyG", 0x48 => "KeyH", 0x49 => "KeyI", 0x4A => "KeyJ",
                0x4B => "KeyK", 0x4C => "KeyL", 0x4D => "KeyM", 0x4E => "KeyN", 0x4F => "KeyO",
                0x50 => "KeyP", 0x51 => "KeyQ", 0x52 => "KeyR", 0x53 => "KeyS", 0x54 => "KeyT",
                0x55 => "KeyU", 0x56 => "KeyV", 0x57 => "KeyW", 0x58 => "KeyX", 0x59 => "KeyY",
                _ => "KeyZ",
            },
            0x30..=0x39 => match vk {
                0x30 => "Num0", 0x31 => "Num1", 0x32 => "Num2", 0x33 => "Num3", 0x34 => "Num4",
                0x35 => "Num5", 0x36 => "Num6", 0x37 => "Num7", 0x38 => "Num8", _ => "Num9",
            },
            0x70..=0x7B => match vk {
                0x70 => "F1", 0x71 => "F2", 0x72 => "F3", 0x73 => "F4", 0x74 => "F5", 0x75 => "F6",
                0x76 => "F7", 0x77 => "F8", 0x78 => "F9", 0x79 => "F10", 0x7A => "F11", _ => "F12",
            },
            _ => return None,
        })
    }
}
