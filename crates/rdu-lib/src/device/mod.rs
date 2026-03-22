//! Device information module.

use std::path::Path;

/// Device information.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device name
    pub name: String,
    /// Mount point
    pub mount_point: String,
    /// Total size in bytes
    pub total_size: u64,
    /// Used size in bytes
    pub used_size: u64,
    /// Free size in bytes
    pub free_size: u64,
}

/// Get list of mounted devices.
#[cfg(target_os = "linux")]
pub fn get_devices() -> std::io::Result<Vec<DeviceInfo>> {
    use std::fs;

    let mut devices = Vec::new();

    // Read /proc/mounts
    let mounts = fs::read_to_string("/proc/mounts")?;
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let device = parts[0];
        let mount_point = parts[1];

        // Skip pseudo filesystems
        if device.starts_with("none")
            || device.starts_with("tmpfs")
            || device.starts_with("devtmpfs")
            || device.starts_with("cgmfs")
            || device.starts_with("mqueue")
            || device.starts_with("hugetlbfs")
            || device.starts_with("debugfs")
            || device.starts_with("tracefs")
            || device.starts_with("securityfs")
            || device.starts_with("pstore")
            || device.starts_with("configfs")
            || device.starts_with("fusectl")
            || device.starts_with("sysfs")
            || device.starts_with("proc")
            || device.starts_with("devpts")
            || device.starts_with("cgroup")
            || mount_point.starts_with("/sys")
            || mount_point.starts_with("/proc")
            || mount_point.starts_with("/dev")
        {
            continue;
        }

        // Get filesystem stats
        if let Ok(stat) = get_fs_stats(Path::new(mount_point)) {
            devices.push(DeviceInfo {
                name: device.to_string(),
                mount_point: mount_point.to_string(),
                total_size: stat.total,
                used_size: stat.used,
                free_size: stat.free,
            });
        }
    }

    Ok(devices)
}

/// Get list of mounted devices.
#[cfg(target_os = "windows")]
pub fn get_devices() -> std::io::Result<Vec<DeviceInfo>> {
    // Windows implementation would use GetLogicalDriveStrings
    // For now, return empty list
    Ok(Vec::new())
}

/// Get list of mounted devices.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_devices() -> std::io::Result<Vec<DeviceInfo>> {
    Ok(Vec::new())
}

/// Filesystem statistics.
#[derive(Debug, Clone)]
pub struct FsStats {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

/// Get filesystem statistics for a path.
#[cfg(target_os = "linux")]
pub fn get_fs_stats(path: &Path) -> std::io::Result<FsStats> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let path_c = CString::new(path.to_string_lossy().as_bytes()).unwrap();
    let mut statfs: libc::statfs = unsafe { MaybeUninit::zeroed().assume_init() };

    let result = unsafe { libc::statfs(path_c.as_ptr(), &mut statfs) };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let block_size = statfs.f_bsize as u64;
    let total_blocks = statfs.f_blocks as u64;
    let free_blocks = statfs.f_bavail as u64;
    let used_blocks = total_blocks - statfs.f_bfree as u64;

    Ok(FsStats {
        total: total_blocks * block_size,
        used: used_blocks * block_size,
        free: free_blocks * block_size,
    })
}

/// Get filesystem statistics for a path.
#[cfg(target_os = "windows")]
pub fn get_fs_stats(path: &Path) -> std::io::Result<FsStats> {
    // Windows implementation would use GetDiskFreeSpaceEx
    // For now, return zeros
    Ok(FsStats {
        total: 0,
        used: 0,
        free: 0,
    })
}

/// Get filesystem statistics for a path.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_fs_stats(path: &Path) -> std::io::Result<FsStats> {
    Ok(FsStats {
        total: 0,
        used: 0,
        free: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo {
            name: "/dev/sda1".to_string(),
            mount_point: "/".to_string(),
            total_size: 1000000000,
            used_size: 500000000,
            free_size: 500000000,
        };

        assert_eq!(device.name, "/dev/sda1");
        assert_eq!(device.mount_point, "/");
        assert_eq!(device.total_size, 1000000000);
        assert_eq!(device.used_size, 500000000);
        assert_eq!(device.free_size, 500000000);
    }

    #[test]
    fn test_fs_stats_creation() {
        let stats = FsStats {
            total: 1000,
            used: 600,
            free: 400,
        };

        assert_eq!(stats.total, 1000);
        assert_eq!(stats.used, 600);
        assert_eq!(stats.free, 400);
    }

    #[test]
    fn test_device_info_clone() {
        let device = DeviceInfo {
            name: "test".to_string(),
            mount_point: "/mnt".to_string(),
            total_size: 1000,
            used_size: 500,
            free_size: 500,
        };

        let cloned = device.clone();
        assert_eq!(cloned.name, device.name);
        assert_eq!(cloned.mount_point, device.mount_point);
    }

    #[test]
    fn test_fs_stats_clone() {
        let stats = FsStats {
            total: 1000,
            used: 500,
            free: 500,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.total, stats.total);
        assert_eq!(cloned.used, stats.used);
        assert_eq!(cloned.free, stats.free);
    }

    #[test]
    fn test_get_devices_returns_vec() {
        // This test just verifies that get_devices returns without error
        let result = get_devices();
        assert!(result.is_ok());
    }
}
