use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks, ProcessRefreshKind, MemoryRefreshKind};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub cpu: f64,
    pub memory: f64,
    pub swap_used: f64,
    pub disk_usage: f64,
    pub load_avg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub cpu: f64,
    pub mem: f64,
    pub rss: i64,
}

pub struct Collector {
    sys: Mutex<System>,
}

impl Collector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        std::thread::sleep(std::time::Duration::from_millis(200));
        sys.refresh_cpu_usage();
        Self { sys: Mutex::new(sys) }
    }

    pub async fn collect(&self) -> Result<Metric, String> {
        let (cpu, memory, swap_used) = {
            let mut sys = self.sys.lock().map_err(|e| e.to_string())?;
            sys.refresh_cpu_usage();
            sys.refresh_memory_specifics(MemoryRefreshKind::everything());
            let cpu = sys.global_cpu_info().cpu_usage() as f64;
            let total_mem = sys.total_memory() as f64;
            let used_mem = sys.used_memory() as f64;
            let memory = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };
            let swap_used = sys.used_swap() as f64 / 1024.0 / 1024.0; // bytes -> MB
            (cpu, memory, swap_used)
        };

        let disk_usage = Self::get_disk_usage();
        let load_avg = System::load_average().one;

        Ok(Metric {
            id: 0,
            timestamp: Utc::now(),
            cpu,
            memory,
            swap_used,
            disk_usage,
            load_avg,
        })
    }

    pub async fn collect_top_processes(&self) -> Result<Vec<ProcessInfo>, String> {
        let mut sys = self.sys.lock().map_err(|e| e.to_string())?;
        sys.refresh_processes_specifics(ProcessRefreshKind::everything());

        let total_mem = sys.total_memory() as f64;
        let mut processes: Vec<ProcessInfo> = sys.processes().iter().map(|(pid, proc_info)| {
            let rss = proc_info.memory() as i64; // bytes
            let mem = if total_mem > 0.0 { (rss as f64 / total_mem) * 100.0 } else { 0.0 };
            ProcessInfo {
                pid: pid.as_u32() as i32,
                name: proc_info.name().to_string(),
                cpu: proc_info.cpu_usage() as f64,
                mem,
                rss,
            }
        }).collect();

        processes.sort_by(|a, b| {
            b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)
                .then(b.mem.partial_cmp(&a.mem).unwrap_or(std::cmp::Ordering::Equal))
        });

        Ok(processes)
    }

    fn get_disk_usage() -> f64 {
        let disks = Disks::new_with_refreshed_list();
        for disk in disks.list() {
            if disk.mount_point().to_string_lossy() == "/" {
                let total = disk.total_space() as f64;
                let available = disk.available_space() as f64;
                if total > 0.0 {
                    return ((total - available) / total) * 100.0;
                }
            }
        }
        0.0
    }
}
