use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatus {
    pub total_mb: u64,
    pub available_mb: u64,
    pub free_mb: u64,
    pub used_mb: u64,
    pub swap_total_mb: u64,
    pub swap_free_mb: u64,
    pub swap_used_mb: u64,
}

pub struct MemorySentinel;

impl MemorySentinel {
    pub fn read_memory_status() -> std::io::Result<MemoryStatus> {
        let content = std::fs::read_to_string("/proc/meminfo")?;
        let mut total_kb = 0u64;
        let mut free_kb = 0u64;
        let mut avail_kb = 0u64;
        let mut swap_total_kb = 0u64;
        let mut swap_free_kb = 0u64;

        for line in content.lines() {
            let mut parts = line.split_whitespace();
            if let Some(key) = parts.next() {
                let val = parts.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                match key {
                    "MemTotal:" => total_kb = val,
                    "MemFree:" => free_kb = val,
                    "MemAvailable:" => avail_kb = val,
                    "SwapTotal:" => swap_total_kb = val,
                    "SwapFree:" => swap_free_kb = val,
                    _ => {}
                }
            }
        }

        let total_mb = total_kb / 1024;
        let available_mb = avail_kb / 1024;
        let free_mb = free_kb / 1024;
        let used_mb = total_mb.saturating_sub(available_mb);
        let swap_total_mb = swap_total_kb / 1024;
        let swap_free_mb = swap_free_kb / 1024;
        let swap_used_mb = swap_total_mb.saturating_sub(swap_free_mb);

        Ok(MemoryStatus {
            total_mb,
            available_mb,
            free_mb,
            used_mb,
            swap_total_mb,
            swap_free_mb,
            swap_used_mb,
        })
    }

    pub fn can_start_inference(required_mb: u64, min_free_mb: u64) -> Result<(), String> {
        let mem = Self::read_memory_status().map_err(|e| format!("Failed to read memory: {e}"))?;
        let needed = required_mb + min_free_mb;
        if mem.available_mb < needed {
            return Err(format!(
                "Insufficient RAM: Available is {} MB, but {} MB is required ({} MB model + {} MB reserve)",
                mem.available_mb, needed, required_mb, min_free_mb
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_memory_status() {
        let status = MemorySentinel::read_memory_status().expect("Must read meminfo");
        assert!(status.total_mb > 0);
        assert!(status.available_mb > 0);
        println!("Memory status: {:?}", status);
    }
}
