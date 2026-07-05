use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use sysinfo::{Disk, DiskKind, Disks, Networks, System};
use tauri::State;

#[derive(Clone)]
pub struct SystemStatsState {
    collector: Arc<Mutex<SystemStatsCollector>>,
}

impl SystemStatsState {
    pub fn new() -> Self {
        Self {
            collector: Arc::new(Mutex::new(SystemStatsCollector::new())),
        }
    }
}

struct SystemStatsCollector {
    system: System,
    disks: Disks,
    networks: Networks,
    last_received: u64,
    last_transmitted: u64,
    last_sample: Instant,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    cpu_percent: Option<f32>,
    memory_percent: Option<f32>,
    disk_used_bytes: Option<u64>,
    disk_total_bytes: Option<u64>,
    disk_label: Option<String>,
    download_bytes_per_second: Option<f64>,
    upload_bytes_per_second: Option<f64>,
}

impl SystemStatsCollector {
    fn new() -> Self {
        let system = System::new_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let (last_received, last_transmitted) = network_totals(&networks);

        Self {
            system,
            disks,
            networks,
            last_received,
            last_transmitted,
            last_sample: Instant::now(),
        }
    }

    fn sample(&mut self) -> SystemStats {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh(true);
        self.networks.refresh(true);

        let elapsed_seconds = self.last_sample.elapsed().as_secs_f64();
        let (received, transmitted) = network_totals(&self.networks);

        let (download, upload) = if elapsed_seconds > 0.0 {
            (
                Some(received.saturating_sub(self.last_received) as f64 / elapsed_seconds),
                Some(transmitted.saturating_sub(self.last_transmitted) as f64 / elapsed_seconds),
            )
        } else {
            (None, None)
        };

        self.last_received = received;
        self.last_transmitted = transmitted;
        self.last_sample = Instant::now();

        let total_memory = self.system.total_memory();
        let memory_percent = (total_memory > 0)
            .then(|| self.system.used_memory() as f32 / total_memory as f32 * 100.0);

        let disk = primary_disk(&self.disks);
        let disk_total_bytes = disk.map(Disk::total_space).filter(|total| *total > 0);
        let disk_used_bytes = disk
            .map(|disk| disk.total_space().saturating_sub(disk.available_space()))
            .filter(|_| disk_total_bytes.is_some());
        let disk_label = disk.map(|disk| match disk.kind() {
            DiskKind::SSD => "SSD",
            DiskKind::HDD => "HD",
            _ => "DISCO",
        });

        SystemStats {
            cpu_percent: (!self.system.cpus().is_empty()).then(|| self.system.global_cpu_usage()),
            memory_percent,
            disk_used_bytes,
            disk_total_bytes,
            disk_label: disk_label.map(str::to_owned),
            download_bytes_per_second: download,
            upload_bytes_per_second: upload,
        }
    }
}

fn network_totals(networks: &Networks) -> (u64, u64) {
    networks
        .iter()
        .fold((0, 0), |(received, transmitted), (_, data)| {
            (
                received.saturating_add(data.total_received()),
                transmitted.saturating_add(data.total_transmitted()),
            )
        })
}

fn primary_disk(disks: &Disks) -> Option<&Disk> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(system_drive) = std::env::var("SystemDrive") {
            if let Some(disk) = disks.iter().find(|disk| {
                disk.mount_point()
                    .to_string_lossy()
                    .trim_end_matches(['\\', '/'])
                    .eq_ignore_ascii_case(system_drive.trim_end_matches(['\\', '/']))
            }) {
                return Some(disk);
            }
        }
    }

    disks
        .iter()
        .filter(|disk| !disk.is_removable())
        .max_by_key(|disk| disk.total_space())
        .or_else(|| disks.iter().max_by_key(|disk| disk.total_space()))
}

#[tauri::command]
pub async fn get_system_stats(state: State<'_, SystemStatsState>) -> Result<SystemStats, String> {
    let collector = Arc::clone(&state.collector);

    tauri::async_runtime::spawn_blocking(move || {
        collector
            .lock()
            .map_err(|_| "Não foi possível acessar o monitor do sistema.".to_owned())
            .map(|mut collector| collector.sample())
    })
    .await
    .map_err(|error| format!("Falha ao coletar status do sistema: {error}"))?
}
