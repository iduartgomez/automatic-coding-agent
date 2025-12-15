//! Container resource monitoring.
//!
//! Provides APIs for monitoring container resource usage including
//! CPU, memory, network, and disk I/O.

use crate::container::{ContainerError, Result};
use bollard::Docker;
use bollard::models::ContainerStatsResponse;
use futures::stream::StreamExt;
use tracing::debug;

/// Container resource statistics.
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// CPU usage percentage (0-100 per core)
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Memory limit in bytes
    pub memory_limit: u64,
    /// Memory usage percentage (0-100)
    pub memory_percent: f64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Network bytes transmitted
    pub network_tx_bytes: u64,
    /// Block I/O bytes read
    pub block_io_read: u64,
    /// Block I/O bytes written
    pub block_io_written: u64,
}

/// Resource monitor for containers.
pub struct ResourceMonitor {
    docker: Docker,
}

impl ResourceMonitor {
    /// Create a new resource monitor.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Get current statistics for a container.
    ///
    /// # Errors
    ///
    /// Returns error if container not found or stats unavailable.
    pub async fn stats(&self, container_id: &str) -> Result<ContainerStats> {
        debug!("Fetching stats for container: {}", container_id);

        let mut stream = self
            .docker
            .stats(container_id, None::<bollard::container::StatsOptions>);

        if let Some(result) = stream.next().await {
            let stats = result?;

            // Calculate CPU percentage
            let cpu_percent = Self::calculate_cpu_percent(&stats);

            // Extract memory stats
            let (memory_usage, memory_limit) = stats
                .memory_stats
                .as_ref()
                .map(|mem| {
                    let usage = mem.usage.unwrap_or(0);
                    let limit = mem.limit.unwrap_or(1);
                    (usage, limit)
                })
                .unwrap_or((0, 1));

            let memory_percent = if memory_limit > 0 {
                (memory_usage as f64 / memory_limit as f64) * 100.0
            } else {
                0.0
            };

            // Extract network stats
            let (network_rx_bytes, network_tx_bytes) = stats
                .networks
                .as_ref()
                .and_then(|networks| networks.values().next())
                .map(|net| (net.rx_bytes.unwrap_or(0), net.tx_bytes.unwrap_or(0)))
                .unwrap_or((0, 0));

            // Extract block I/O stats - simplified for now
            let (block_io_read, block_io_written) = (0, 0);

            return Ok(ContainerStats {
                cpu_percent,
                memory_usage,
                memory_limit,
                memory_percent,
                network_rx_bytes,
                network_tx_bytes,
                block_io_read,
                block_io_written,
            });
        }

        Err(ContainerError::Other(format!(
            "No stats available for container: {}",
            container_id
        )))
    }

    /// Calculate CPU usage percentage from stats.
    fn calculate_cpu_percent(stats: &ContainerStatsResponse) -> f64 {
        let cpu_stats = match &stats.cpu_stats {
            Some(s) => s,
            None => return 0.0,
        };

        let precpu_stats = match &stats.precpu_stats {
            Some(s) => s,
            None => return 0.0,
        };

        let cpu_usage = cpu_stats.cpu_usage.as_ref();
        let precpu_usage = precpu_stats.cpu_usage.as_ref();

        let (cpu_total, precpu_total) = match (cpu_usage, precpu_usage) {
            (Some(c), Some(p)) => (c.total_usage.unwrap_or(0), p.total_usage.unwrap_or(0)),
            _ => return 0.0,
        };

        let cpu_delta = cpu_total.saturating_sub(precpu_total);

        let system_delta = cpu_stats
            .system_cpu_usage
            .unwrap_or(0)
            .saturating_sub(precpu_stats.system_cpu_usage.unwrap_or(0));

        let online_cpus = cpu_stats.online_cpus.unwrap_or(1) as u64;

        if system_delta > 0 && cpu_delta > 0 {
            (cpu_delta as f64 / system_delta as f64) * online_cpus as f64 * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_stats_creation() {
        let stats = ContainerStats {
            cpu_percent: 25.5,
            memory_usage: 1_073_741_824, // 1GB
            memory_limit: 2_147_483_648, // 2GB
            memory_percent: 50.0,
            network_rx_bytes: 1024,
            network_tx_bytes: 2048,
            block_io_read: 4096,
            block_io_written: 8192,
        };

        assert_eq!(stats.cpu_percent, 25.5);
        assert_eq!(stats.memory_percent, 50.0);
    }
}
