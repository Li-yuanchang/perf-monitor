// macOS native window handling
use cocoa::base::{id, YES, NO, nil};
use cocoa::foundation::NSString;
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

/// 显示系统标题栏（Titled + Closable + Miniaturizable + Resizable）
#[cfg(target_os = "macos")]
pub fn show_title_bar(window: &Window) {
    unsafe {
        let ns_window = window.ns_window().unwrap() as id;
        // NSWindowStyleMaskTitled(1) | Closable(2) | Miniaturizable(4) | Resizable(8) = 15
        let _: () = msg_send![ns_window, setStyleMask: 15u64];
        let _: () = msg_send![ns_window, setHasShadow: YES];
        let _: () = msg_send![ns_window, setMovable: YES];
        let _: () = msg_send![ns_window, setTitlebarAppearsTransparent: NO];
        let _: () = msg_send![ns_window, setOpaque: YES];
        // 设置窗口背景色为系统默认
        let bg_color: id = msg_send![class!(NSColor), windowBackgroundColor];
        let _: () = msg_send![ns_window, setBackgroundColor: bg_color];
        // 隐藏标题文字和文档图标
        // NSWindowTitleHidden = 1
        let _: () = msg_send![ns_window, setTitleVisibility: 1i64];
        let _: () = msg_send![ns_window, setRepresentedURL: nil];
        let title = NSString::alloc(nil).init_str("");
        let _: () = msg_send![ns_window, setTitle: title];
    }
}

/// 隐藏系统标题栏（Borderless）
#[cfg(target_os = "macos")]
pub fn hide_title_bar(window: &Window) {
    unsafe {
        let ns_window = window.ns_window().unwrap() as id;
        // NSWindowStyleMaskBorderless = 0
        let _: () = msg_send![ns_window, setStyleMask: 0u64];
        let _: () = msg_send![ns_window, setHasShadow: NO];
        let clear_color: id = msg_send![class!(NSColor), clearColor];
        let _: () = msg_send![ns_window, setBackgroundColor: clear_color];
        let _: () = msg_send![ns_window, setOpaque: NO];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn show_title_bar(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn hide_title_bar(_window: &Window) {}

/// 隐藏 Dock 图标（设为 accessory app）
#[cfg(target_os = "macos")]
pub fn hide_dock_icon() {
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyAccessory = 1
        let _: () = msg_send![app, setActivationPolicy: 1i64];
    }
}

/// 显示 Dock 图标（设为 regular app），并恢复应用图标
#[cfg(target_os = "macos")]
pub fn show_dock_icon() {
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyRegular = 0
        let _: () = msg_send![app, setActivationPolicy: 0i64];

        // 通过可执行文件路径找到 icons 目录，加载应用图标
        if let Ok(exe) = std::env::current_exe() {
            // dev 模式: exe 在 target/debug/ 下，icons 在 src-tauri/icons/
            // release 模式: exe 在 .app/Contents/MacOS/ 下
            let candidates: Vec<std::path::PathBuf> = vec![
                // dev 模式路径
                exe.parent().unwrap_or(std::path::Path::new("."))
                    .join("../../icons/icon.icns"),
                exe.parent().unwrap_or(std::path::Path::new("."))
                    .join("../../icons/128x128@2x.png"),
                // release 模式路径
                exe.parent().unwrap_or(std::path::Path::new("."))
                    .join("../Resources/icon.icns"),
            ];
            for path in &candidates {
                if let Ok(canonical) = path.canonicalize() {
                    let path_str = canonical.to_string_lossy().to_string();
                    let ns_path = NSString::alloc(nil).init_str(&path_str);
                    let image: id = msg_send![class!(NSImage), alloc];
                    let image: id = msg_send![image, initByReferencingFile: ns_path];
                    if image != nil {
                        let is_valid: bool = msg_send![image, isValid];
                        if is_valid {
                            let _: () = msg_send![app, setApplicationIconImage: image];
                            break;
                        }
                    }
                }
            }
        }

        // 激活应用到前台
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn remove_window_shadow(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn make_window_draggable(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn disable_window_draggable(_window: &Window) {}

#[cfg(not(target_os = "macos"))]
pub fn hide_dock_icon() {}

#[cfg(not(target_os = "macos"))]
pub fn show_dock_icon() {}
