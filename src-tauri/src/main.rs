// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tauri::{Manager, Window, SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, SystemTrayEvent};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

mod collector;
mod store;
mod macos;

use collector::{Collector, Metric, ProcessInfo};
use store::{Store, ProcessRanking};
use macos::{remove_window_shadow, make_window_draggable, disable_window_draggable};

struct AppState {
    store: Arc<Mutex<Store>>,
}

use store::TimeMark;

#[derive(Serialize, Deserialize, Clone)]
struct Report {
    before_window: TimeWindowStats,
    after_window: TimeWindowStats,
    comparison: ComparisonStats,
}

#[derive(Serialize, Deserialize, Clone)]
struct TimeWindowStats {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    avg_cpu: f64,
    peak_cpu: f64,
    avg_memory: f64,
    peak_memory: f64,
    avg_swap: f64,
    peak_swap: f64,
    avg_load: f64,
    sample_count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct ComparisonStats {
    cpu_better: bool,
    cpu_better_percent: f64,
    mem_better: bool,
    mem_better_percent: f64,
    swap_better: bool,
    swap_better_percent: f64,
    lag_score_before: i32,
    lag_score_after: i32,
    conclusion: String,
}

// Tauri commands
#[tauri::command]
async fn get_realtime(state: tauri::State<'_, AppState>) -> Result<Option<Metric>, String> {
    let store = state.store.lock().await;
    store.get_latest_metric().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_history(
    state: tauri::State<'_, AppState>,
    start: String,
    end: String,
) -> Result<Vec<Metric>, String> {
    let start_time = DateTime::parse_from_rfc3339(&start)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let end_time = DateTime::parse_from_rfc3339(&end)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);

    let store = state.store.lock().await;
    store.query_metrics(start_time, end_time).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_mark(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    let store = state.store.lock().await;
    store.set_time_mark(&name, "").await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_marks(state: tauri::State<'_, AppState>) -> Result<Vec<TimeMark>, String> {
    let store = state.store.lock().await;
    store.get_time_marks().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn generate_report(
    state: tauri::State<'_, AppState>,
    before_start: String,
    before_end: String,
    after_start: String,
    after_end: String,
) -> Result<Report, String> {
    let before_start_time = DateTime::parse_from_rfc3339(&before_start)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let before_end_time = DateTime::parse_from_rfc3339(&before_end)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let after_start_time = DateTime::parse_from_rfc3339(&after_start)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let after_end_time = DateTime::parse_from_rfc3339(&after_end)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);

    let store = state.store.lock().await;

    let before_metrics = store.query_metrics(before_start_time, before_end_time).await
        .map_err(|e| e.to_string())?;
    let after_metrics = store.query_metrics(after_start_time, after_end_time).await
        .map_err(|e| e.to_string())?;

    let before_stats = calculate_stats(before_metrics, before_start_time, before_end_time);
    let after_stats = calculate_stats(after_metrics, after_start_time, after_end_time);

    let comparison = compare_windows(&before_stats, &after_stats);

    Ok(Report {
        before_window: before_stats,
        after_window: after_stats,
        comparison,
    })
}

#[tauri::command]
async fn export_csv(
    state: tauri::State<'_, AppState>,
    start: String,
    end: String,
) -> Result<String, String> {
    let start_time = DateTime::parse_from_rfc3339(&start)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let end_time = DateTime::parse_from_rfc3339(&end)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);

    let store = state.store.lock().await;
    let metrics = store.query_metrics(start_time, end_time).await
        .map_err(|e| e.to_string())?;

    let mut csv = "Timestamp,CPU%,Memory%,SwapUsed(MB),DiskUsage%,LoadAvg\n".to_string();
    for m in metrics {
        csv.push_str(&format!(
            "{},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
            m.timestamp.to_rfc3339(),
            m.cpu,
            m.memory,
            m.swap_used,
            m.disk_usage,
            m.load_avg
        ));
    }

    let home_dir = dirs::home_dir().ok_or("Cannot get home dir")?;
    let filename = format!("performance_{}.csv", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let filepath = home_dir.join("Downloads").join(&filename);

    tokio::fs::write(&filepath, csv).await.map_err(|e: std::io::Error| e.to_string())?;

    Ok(filepath.to_string_lossy().to_string())
}

#[tauri::command]
async fn get_process_ranking(
    state: tauri::State<'_, AppState>,
    start: String,
    end: String,
) -> Result<Vec<ProcessRanking>, String> {
    let start_time = DateTime::parse_from_rfc3339(&start)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let end_time = DateTime::parse_from_rfc3339(&end)
        .map_err(|e| e.to_string())?
        .with_timezone(&Utc);
    let store = state.store.lock().await;
    store.get_process_ranking(start_time, end_time).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_mark(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let store = state.store.lock().await;
    store.delete_time_mark(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_processes() -> Result<Vec<ProcessInfo>, String> {
    let collector = Collector::new();
    collector.collect_top_processes().await.map_err(|e| e.to_string())
}

#[tauri::command]
fn kill_process(pid: i32) -> Result<String, String> {
    use std::process::Command;
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(format!("已终止进程 {}", pid))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("终止失败: {}", err))
    }
}

#[tauri::command]
fn get_settings() -> serde_json::Value {
    serde_json::json!({
        "collectionInterval": 3,
        "retentionDays": 30,
        "processInterval": 30,
    })
}

#[tauri::command]
fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
fn get_autostart() -> bool {
    let plist_path = dirs::home_dir()
        .map(|h| h.join("Library/LaunchAgents/com.example.PerfMonitor.plist"));
    plist_path.map(|p| p.exists()).unwrap_or(false)
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let plist_path = home.join("Library/LaunchAgents/com.example.PerfMonitor.plist");

    if enabled {
        // Get current executable path
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let plist_content = format!(
r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.PerfMonitor</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#, exe.display());

        // Ensure LaunchAgents directory exists
        let launch_dir = home.join("Library/LaunchAgents");
        std::fs::create_dir_all(&launch_dir).map_err(|e| e.to_string())?;
        std::fs::write(&plist_path, plist_content).map_err(|e| e.to_string())?;
        Ok("已启用开机自启动".to_string())
    } else {
        if plist_path.exists() {
            std::fs::remove_file(&plist_path).map_err(|e| e.to_string())?;
        }
        Ok("已关闭开机自启动".to_string())
    }
}

// Window control commands
#[tauri::command]
fn expand_window(window: Window) {
    // 关闭背景拖动，恢复正常点击事件
    disable_window_draggable(&window);

    // 先清除尺寸限制
    let _ = window.set_min_size(None::<tauri::Size>);
    let _ = window.set_max_size(None::<tauri::Size>);

    // 启用装饰
    let _ = window.set_decorations(true);
    let _ = window.set_skip_taskbar(false);
    let _ = window.set_always_on_top(false);
    let _ = window.set_resizable(true);

    // 设置窗口大小
    let win_w: f64 = 800.0;
    let win_h: f64 = 600.0;
    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: win_w,
        height: win_h,
    }));

    // 获取小球位置和屏幕尺寸，确保展开窗口完全可见
    let pos = window.outer_position().unwrap_or(tauri::PhysicalPosition { x: 100, y: 100 });
    if let Ok(monitor) = window.current_monitor() {
        if let Some(m) = monitor {
            let scale = m.scale_factor();
            let screen_w = m.size().width as f64 / scale;
            let screen_h = m.size().height as f64 / scale;
            let ball_x = pos.x as f64 / scale;
            let ball_y = pos.y as f64 / scale;

            // 如果窗口右边超出屏幕，向左推
            let x = if ball_x + win_w > screen_w {
                (screen_w - win_w - 10.0).max(0.0)
            } else {
                ball_x
            };
            // 如果窗口下边超出屏幕，向上推
            let y = if ball_y + win_h > screen_h {
                (screen_h - win_h - 10.0).max(0.0)
            } else {
                ball_y
            };

            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
    }
}

#[tauri::command]
fn collapse_window(window: Window) {
    // 禁用装饰
    let _ = window.set_decorations(false);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_always_on_top(true);
    let _ = window.set_resizable(false);

    // 使用 Logical 尺寸，确保在 Retina 屏幕上也是 64x64 CSS 像素
    let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize {
        width: 64.0,
        height: 64.0,
    })));
    let _ = window.set_max_size(Some(tauri::Size::Logical(tauri::LogicalSize {
        width: 64.0,
        height: 64.0,
    })));

    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: 64.0,
        height: 64.0,
    }));

    // 移除阴影和边框
    remove_window_shadow(&window);
    // 启用拖动
    make_window_draggable(&window);
}

fn calculate_stats(metrics: Vec<Metric>, start: DateTime<Utc>, end: DateTime<Utc>) -> TimeWindowStats {
    if metrics.is_empty() {
        return TimeWindowStats {
            start,
            end,
            avg_cpu: 0.0,
            peak_cpu: 0.0,
            avg_memory: 0.0,
            peak_memory: 0.0,
            avg_swap: 0.0,
            peak_swap: 0.0,
            avg_load: 0.0,
            sample_count: 0,
        };
    }

    let count = metrics.len() as f64;
    let sum_cpu: f64 = metrics.iter().map(|m| m.cpu).sum();
    let sum_mem: f64 = metrics.iter().map(|m| m.memory).sum();
    let sum_swap: f64 = metrics.iter().map(|m| m.swap_used).sum();
    let sum_load: f64 = metrics.iter().map(|m| m.load_avg).sum();

    TimeWindowStats {
        start,
        end,
        avg_cpu: sum_cpu / count,
        peak_cpu: metrics.iter().map(|m| m.cpu).fold(0.0, f64::max),
        avg_memory: sum_mem / count,
        peak_memory: metrics.iter().map(|m| m.memory).fold(0.0, f64::max),
        avg_swap: sum_swap / count,
        peak_swap: metrics.iter().map(|m| m.swap_used).fold(0.0, f64::max),
        avg_load: sum_load / count,
        sample_count: metrics.len() as i64,
    }
}

fn compare_windows(before: &TimeWindowStats, after: &TimeWindowStats) -> ComparisonStats {
    let cpu_better_percent = if before.avg_cpu > 0.0 {
        (before.avg_cpu - after.avg_cpu) / before.avg_cpu * 100.0
    } else {
        0.0
    };

    let mem_better_percent = if before.avg_memory > 0.0 {
        (before.avg_memory - after.avg_memory) / before.avg_memory * 100.0
    } else {
        0.0
    };

    let swap_better_percent = if before.avg_swap > 0.0 {
        (before.avg_swap - after.avg_swap) / before.avg_swap * 100.0
    } else {
        0.0
    };

    let lag_score_before = calculate_lag_score(before);
    let lag_score_after = calculate_lag_score(after);

    let conclusion = if cpu_better_percent > 5.0 && mem_better_percent > 5.0 {
        "系统性能有明显改善，建议正式升级"
    } else if cpu_better_percent > 0.0 || mem_better_percent > 0.0 {
        "系统性能略有提升，升级效果正面"
    } else if cpu_better_percent < -5.0 || mem_better_percent < -5.0 {
        "系统性能下降，建议检查升级配置"
    } else {
        "系统性能基本持平，无明显变化"
    };

    ComparisonStats {
        cpu_better: cpu_better_percent > 5.0,
        cpu_better_percent,
        mem_better: mem_better_percent > 5.0,
        mem_better_percent,
        swap_better: swap_better_percent > 5.0,
        swap_better_percent,
        lag_score_before,
        lag_score_after,
        conclusion: conclusion.to_string(),
    }
}

fn calculate_lag_score(stats: &TimeWindowStats) -> i32 {
    let score = stats.avg_cpu * 0.35
        + stats.avg_memory * 0.30
        + (if stats.avg_swap > 100.0 { (stats.avg_swap - 100.0) / 10.0 } else { 0.0 }) * 0.20
        + stats.avg_load * 10.0 * 0.15;

    score.min(100.0).max(0.0) as i32
}

#[tokio::main]
async fn main() {
    let store = Arc::new(Mutex::new(Store::new().await.expect("Failed to init store")));
    let collector = Arc::new(Collector::new());

    // Start metrics collection loop (every 5s)
    let store_clone = store.clone();
    let collector_clone = collector.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            if let Ok(metric) = collector_clone.collect().await {
                let store = store_clone.lock().await;
                let _ = store.insert_metric(&metric).await;
            }
        }
    });

    // Start process snapshot collection loop (every 60s, top 20 only)
    let store_clone2 = store.clone();
    let collector_clone2 = collector.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;

            if let Ok(mut processes) = collector_clone2.collect_top_processes().await {
                processes.truncate(20);
                let store = store_clone2.lock().await;
                let _ = store.save_process_snapshot(&processes).await;
            }
        }
    });

    // Cleanup old data daily
    let store_cleanup = store.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            let store = store_cleanup.lock().await;
            let _ = store.cleanup_old_data(30).await;
        }
    });

    // System tray
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show", "显示窗口"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "退出"));
    let system_tray = SystemTray::new()
        .with_menu(tray_menu)
        .with_icon_as_template(false);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick { .. } => {
                if let Some(window) = app.get_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .manage(AppState { store })
        .setup(|app| {
            // 应用启动时设置窗口属性
            let window = app.get_window("main").unwrap();
            remove_window_shadow(&window);
            make_window_draggable(&window);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_realtime,
            get_history,
            set_mark,
            get_marks,
            generate_report,
            export_csv,
            get_settings,
            quit_app,
            get_autostart,
            set_autostart,
            expand_window,
            collapse_window,
            get_processes,
            kill_process,
            delete_mark,
            get_process_ranking,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
