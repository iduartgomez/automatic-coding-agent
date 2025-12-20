//! System resource detection for container allocation.
//!
//! Provides utilities to detect system resources (memory, CPU) and
//! calculate appropriate allocations for containers.

use std::io;

/// Detected system resources
#[derive(Debug, Clone)]
pub struct SystemResources {
    /// Total memory in bytes
    pub total_memory_bytes: u64,
    /// Number of CPU cores
    pub cpu_cores: u32,
}

/// Resource allocation for containers
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Memory limit in bytes
    pub memory_bytes: i64,
    /// CPU quota (microseconds per period, 100000 = 1 core)
    pub cpu_quota: i64,
}

impl SystemResources {
    /// Detect system resources
    ///
    /// # Errors
    ///
    /// Returns an error if resource detection fails.
    pub fn detect() -> io::Result<Self> {
        let total_memory_bytes = Self::detect_memory()?;
        let cpu_cores = Self::detect_cpus();

        Ok(Self {
            total_memory_bytes,
            cpu_cores,
        })
    }

    /// Allocate a percentage of system resources
    ///
    /// # Arguments
    ///
    /// * `percentage` - Percentage to allocate (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Resource allocation with memory and CPU limits
    pub fn allocate_percentage(&self, percentage: f64) -> ResourceAllocation {
        let percentage = percentage.clamp(0.0, 1.0);
        let memory_bytes = (self.total_memory_bytes as f64 * percentage) as i64;

        // CPU quota: 100000 microseconds = 1 core
        // Allocate percentage of cores
        let cpu_quota = (self.cpu_cores as f64 * percentage * 100_000.0) as i64;

        ResourceAllocation {
            memory_bytes,
            cpu_quota,
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_memory() -> io::Result<u64> {
        use std::fs;
        let meminfo = fs::read_to_string("/proc/meminfo")?;
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Ok(kb * 1024);
                    }
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to parse /proc/meminfo",
        ))
    }

    #[cfg(target_os = "macos")]
    fn detect_memory() -> io::Result<u64> {
        use std::process::Command;
        let output = Command::new("sysctl").args(["-n", "hw.memsize"]).output()?;

        let memory_str = String::from_utf8_lossy(&output.stdout);
        memory_str
            .trim()
            .parse::<u64>()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    #[cfg(target_os = "windows")]
    fn detect_memory() -> io::Result<u64> {
        use std::process::Command;
        let output = Command::new("wmic")
            .args(["ComputerSystem", "get", "TotalPhysicalMemory"])
            .output()?;

        let memory_str = String::from_utf8_lossy(&output.stdout);
        // Parse the second line (first line is header)
        for line in memory_str.lines().skip(1) {
            if let Ok(bytes) = line.trim().parse::<u64>() {
                return Ok(bytes);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to parse wmic output",
        ))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn detect_memory() -> io::Result<u64> {
        // Default to 8GB if detection not supported
        tracing::warn!("Memory detection not supported on this platform, using default 8GB");
        Ok(8 * 1024 * 1024 * 1024)
    }

    fn detect_cpus() -> u32 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_detection() {
        let resources = SystemResources::detect();
        assert!(resources.is_ok());

        let resources = resources.unwrap();
        assert!(resources.total_memory_bytes > 0);
        assert!(resources.cpu_cores > 0);
    }

    #[test]
    fn test_resource_allocation() {
        let resources = SystemResources {
            total_memory_bytes: 16 * 1024 * 1024 * 1024, // 16 GB
            cpu_cores: 8,
        };

        let allocation = resources.allocate_percentage(0.5);
        assert_eq!(allocation.memory_bytes, 8 * 1024 * 1024 * 1024); // 8 GB
        assert_eq!(allocation.cpu_quota, 400_000); // 4 cores (4 * 100000)
    }

    #[test]
    fn test_allocation_percentage_clamping() {
        let resources = SystemResources {
            total_memory_bytes: 1024 * 1024 * 1024, // 1 GB
            cpu_cores: 4,
        };

        // Test > 1.0
        let allocation = resources.allocate_percentage(1.5);
        assert_eq!(allocation.memory_bytes, 1024 * 1024 * 1024);
        assert_eq!(allocation.cpu_quota, 400_000);

        // Test < 0.0
        let allocation = resources.allocate_percentage(-0.5);
        assert_eq!(allocation.memory_bytes, 0);
        assert_eq!(allocation.cpu_quota, 0);
    }
}
