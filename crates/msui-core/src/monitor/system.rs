use std::fs;

#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub ram_usage_percent: f32,
}

pub struct SystemMonitor {
    prev_idle: u64,
    prev_total: u64,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let (idle, total) = Self::read_cpu_raw().unwrap_or((0, 0));
        Self {
            prev_idle: idle,
            prev_total: total,
        }
    }

    fn read_cpu_raw() -> Option<(u64, u64)> {
        let content = fs::read_to_string("/proc/stat").ok()?;
        let first_line = content.lines().next()?;
        if !first_line.starts_with("cpu ") {
            return None;
        }

        let parts: Vec<u64> = first_line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() < 4 {
            return None;
        }

        let idle = parts[3] + parts.get(4).copied().unwrap_or(0);
        let total: u64 = parts.iter().sum();
        Some((idle, total))
    }

    fn read_ram_raw() -> Option<(u64, u64, f32)> {
        let content = fs::read_to_string("/proc/meminfo").ok()?;
        let mut total_kb = 0u64;
        let mut avail_kb = 0u64;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line.split_whitespace().nth(1)?.parse().ok()?;
            } else if line.starts_with("MemAvailable:") {
                avail_kb = line.split_whitespace().nth(1)?.parse().ok()?;
            }
            if total_kb > 0 && avail_kb > 0 {
                break;
            }
        }

        if total_kb == 0 {
            return None;
        }

        let used_kb = total_kb.saturating_sub(avail_kb);
        let ram_used_mb = used_kb / 1024;
        let ram_total_mb = total_kb / 1024;
        let ram_usage_percent = (used_kb as f32 / total_kb as f32) * 100.0;

        Some((ram_used_mb, ram_total_mb, ram_usage_percent))
    }

    pub fn sample(&mut self) -> SystemMetrics {
        let mut cpu_percent = 0.0f32;
        if let Some((idle, total)) = Self::read_cpu_raw() {
            let total_delta = total.saturating_sub(self.prev_total);
            let idle_delta = idle.saturating_sub(self.prev_idle);

            if total_delta > 0 {
                cpu_percent = 100.0 * (1.0 - (idle_delta as f32 / total_delta as f32));
                if cpu_percent < 0.0 {
                    cpu_percent = 0.0;
                }
            }

            self.prev_idle = idle;
            self.prev_total = total;
        }

        let (ram_used_mb, ram_total_mb, ram_usage_percent) =
            Self::read_ram_raw().unwrap_or((0, 0, 0.0));

        SystemMetrics {
            cpu_usage_percent: cpu_percent,
            ram_used_mb,
            ram_total_mb,
            ram_usage_percent,
        }
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}
