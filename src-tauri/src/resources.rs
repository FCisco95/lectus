//! Read-only resource snapshot for model guidance. This is not a benchmark or
//! a prediction of GPU support, available VRAM, or inference latency.

#[derive(serde::Serialize)]
pub struct HardwareProfile {
    pub logical_cpus: usize,
    pub total_memory_bytes: Option<u64>,
    pub available_memory_bytes: Option<u64>,
}

pub fn hardware_profile() -> HardwareProfile {
    let (total, available) = physical_memory();
    HardwareProfile {
        logical_cpus: std::thread::available_parallelism().map_or(1, usize::from),
        total_memory_bytes: total,
        available_memory_bytes: available,
    }
}

#[cfg(target_os = "windows")]
fn physical_memory() -> (Option<u64>, Option<u64>) {
    // MEMORYSTATUSEX: DWORD length/load followed by seven DWORDLONG fields.
    #[repr(C)]
    struct MemoryStatus {
        length: u32, load: u32, total_physical: u64, available_physical: u64,
        total_page: u64, available_page: u64, total_virtual: u64,
        available_virtual: u64, available_extended_virtual: u64,
    }
    #[link(name = "kernel32")]
    extern "system" { fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> i32; }
    let mut status = MemoryStatus {
        length: std::mem::size_of::<MemoryStatus>() as u32, load: 0,
        total_physical: 0, available_physical: 0, total_page: 0, available_page: 0,
        total_virtual: 0, available_virtual: 0, available_extended_virtual: 0,
    };
    // SAFETY: fully initialized, correctly sized and aligned OS struct; the
    // synchronous call retains no pointer and only reads memory statistics.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 { return (None, None); }
    (Some(status.total_physical), Some(status.available_physical))
}

#[cfg(target_os = "macos")]
fn physical_memory() -> (Option<u64>, Option<u64>) {
    let mut total: u64 = 0;
    let mut size = std::mem::size_of::<u64>();
    // SAFETY: hw.memsize returns uint64_t to this correctly sized output;
    // null newp with zero newlen makes this a read-only sysctl.
    let result = unsafe {
        libc::sysctlbyname(c"hw.memsize".as_ptr(), (&mut total as *mut u64).cast(),
            &mut size, std::ptr::null_mut(), 0)
    };
    (if result == 0 && size == 8 { Some(total) } else { None }, None)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn physical_memory() -> (Option<u64>, Option<u64>) { (None, None) }

#[cfg(all(test, target_os = "windows"))]
mod tests {
    #[test]
    fn windows_resource_snapshot_reads_consistent_physical_memory() {
        let profile = super::hardware_profile();
        let total = profile.total_memory_bytes.expect("Windows physical memory snapshot");
        let available = profile.available_memory_bytes.expect("Windows available memory snapshot");
        assert!(total > 0 && available <= total);
        assert!(profile.logical_cpus > 0);
        println!("Read-only resources: total={total} available={available} logical_cpus={}", profile.logical_cpus);
    }
}

