//! 功能：按住 Alt 键时临时提升鼠标滚轮滚动速度，松开后自动还原。
//!
//! 实现方式：直接注册低级键盘钩子（WH_KEYBOARD_LL），在 Alt 按下/抬起时
//! 立即切换系统滚动行数，而不是轮询 `GetAsyncKeyState`。

use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LMENU, VK_MENU, VK_RMENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP, SystemParametersInfoW, SPI_GETWHEELSCROLLLINES,
    SPI_SETWHEELSCROLLLINES,
};

/// 按住 Alt 时使用的加速滚动行数（可自由调节）。
const ACCEL_LINES: u32 = 15;

/// 系统默认滚动行数（在启动时读取并缓存）。
static DEFAULT_LINES: AtomicU32 = AtomicU32::new(0);
/// 当前是否已经处于加速状态。
static ACCELERATED: AtomicBool = AtomicBool::new(false);

/// 读取系统当前默认的滚轮滚动行数。
fn get_sys_scroll_lines() -> u32 {
    let mut lines: u32 = 0;
    unsafe {
        SystemParametersInfoW(
            SPI_GETWHEELSCROLLLINES,
            0,
            &mut lines as *mut u32 as *mut core::ffi::c_void,
            0,
        );
    }
    lines
}

/// 设置系统的滚轮滚动行数。
fn set_sys_scroll_lines(lines_count: u32) {
    unsafe {
        SystemParametersInfoW(
            SPI_SETWHEELSCROLLLINES,
            lines_count,
            ptr::null_mut(),
            0,
        );
    }
}

/// 恢复默认滚动速度，并同步状态标志。
fn restore_default_scroll() {
    let default_lines = DEFAULT_LINES.load(Ordering::SeqCst);
    set_sys_scroll_lines(default_lines);
    ACCELERATED.store(false, Ordering::SeqCst);
}

/// 低级键盘钩子：在 Alt 键按下时立即加速，在释放时立即恢复。
unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let msg = w_param as u32;
        if matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
            let hook_struct = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
            let vk_code = hook_struct.vkCode as i32;
            let is_alt = vk_code == VK_MENU as i32
                || vk_code == VK_LMENU as i32
                || vk_code == VK_RMENU as i32;

            if is_alt {
                if matches!(msg, WM_KEYDOWN | WM_SYSKEYDOWN) {
                    if !ACCELERATED.swap(true, Ordering::SeqCst) {
                        set_sys_scroll_lines(ACCEL_LINES);
                        println!("[scroll_accel] 🚀 已临时将滚动行数提升至: {ACCEL_LINES} 行");
                    }
                } else if ACCELERATED.swap(false, Ordering::SeqCst) {
                    let default_lines = DEFAULT_LINES.load(Ordering::SeqCst);
                    set_sys_scroll_lines(default_lines);
                    println!("[scroll_accel] 🔄 已恢复系统默认滚动行数: {default_lines} 行");
                }
            }
        }
    }

    unsafe { CallNextHookEx(ptr::null_mut(), code, w_param, l_param) }
}

/// 启动功能：注册全局键盘钩子，并在消息循环中保持运行。
pub fn run() {
    let default_lines = get_sys_scroll_lines();
    DEFAULT_LINES.store(default_lines, Ordering::SeqCst);
    println!("[scroll_accel] 系统当前默认滚动行数: {default_lines}");
    println!("[scroll_accel] 服务已启动，按住 Alt 键可立即提升滚动速度，松开后恢复默认");

    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            0 as HINSTANCE,
            0,
        )
    };

    if hook == ptr::null_mut() {
        panic!("[scroll_accel] 安装键盘钩子失败");
    }

    let mut msg = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut msg, ptr::null_mut(), 0, 0) };
        if result <= 0 {
            break;
        }

        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe {
        UnhookWindowsHookEx(hook);
    }

    restore_default_scroll();
    println!("[scroll_accel] 已恢复系统默认滚动行数: {default_lines} 行，安全退出");
}
