// macOS native window handling
use cocoa::base::{id, YES, NO};
use objc::{class, msg_send, sel, sel_impl};
use tauri::Window;

#[cfg(target_os = "macos")]
pub fn remove_window_shadow(window: &Window) {
    unsafe {
        let ns_window = window.ns_window().unwrap() as id;
        // 移除阴影
        let _: () = msg_send![ns_window, setHasShadow: NO];
        // 使用 NSColor.clearColor 设置完全透明背景
        let clear_color: id = msg_send![class!(NSColor), clearColor];
        let _: () = msg_send![ns_window, setBackgroundColor: clear_color];
        // 设置不透明为NO，允许透明
        let _: () = msg_send![ns_window, setOpaque: NO];
    }
}

#[cfg(target_os = "macos")]
pub fn make_window_draggable(window: &Window) {
    unsafe {
        let ns_window = window.ns_window().unwrap() as id;
        // 允许窗口拖动
        let _: () = msg_send![ns_window, setMovableByWindowBackground: YES];
    }
}

#[cfg(target_os = "macos")]
pub fn disable_window_draggable(window: &Window) {
    unsafe {
        let ns_window = window.ns_window().unwrap() as id;
        let _: () = msg_send![ns_window, setMovableByWindowBackground: NO];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn remove_window_shadow(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn make_window_draggable(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn disable_window_draggable(_window: &Window) {}
