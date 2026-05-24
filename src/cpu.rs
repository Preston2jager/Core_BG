use windows_sys::Win32::System::Threading::GetSystemTimes;
use windows_sys::Win32::Foundation::FILETIME;

#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: i32,
        system_information: *mut std::ffi::c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[repr(C, align(8))]
#[derive(Copy, Clone, Default, Debug)]
struct SystemProcessorPerformanceInformation {
    idle_time: i64,
    kernel_time: i64,
    user_time: i64,
    dpc_time: i64,
    interrupt_time: i64,
    interrupt_count: u32,
    _padding: u32,
}

pub struct CpuMonitor {
    num_cores: usize,
    
    // Overall CPU tracking
    last_idle: u64,
    last_kernel: u64,
    last_user: u64,
    overall_usage: f32,
    
    // Per-core tracking
    last_cores: Vec<SystemProcessorPerformanceInformation>,
    core_usages: Vec<f32>,
}

impl CpuMonitor {
    fn get_overall_times() -> (u64, u64, u64) {
        let mut idle = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut kernel = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut user = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        unsafe {
            GetSystemTimes(&mut idle, &mut kernel, &mut user);
        }
        let idle_val = ((idle.dwHighDateTime as u64) << 32) | idle.dwLowDateTime as u64;
        let kernel_val = ((kernel.dwHighDateTime as u64) << 32) | kernel.dwLowDateTime as u64;
        let user_val = ((user.dwHighDateTime as u64) << 32) | user.dwLowDateTime as u64;
        (idle_val, kernel_val, user_val)
    }

    fn get_core_times(num_cores: usize) -> Vec<SystemProcessorPerformanceInformation> {
        let struct_size = std::mem::size_of::<SystemProcessorPerformanceInformation>();
        let mut buffer = vec![SystemProcessorPerformanceInformation::default(); num_cores];
        let size = (num_cores * struct_size) as u32;
        let mut ret_len = 0;
        unsafe {
            NtQuerySystemInformation(
                21, // SystemProcessorPerformanceInformation
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
                &mut ret_len,
            );
        }
        buffer
    }

    pub fn new() -> Self {
        crate::app::log_msg("CpuMonitor::new: Initializing native Win32 CPU monitor");
        
        // Accurate way to get logical processor count
        let num_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        
        let (idle, kernel, user) = Self::get_overall_times();
        let last_cores = Self::get_core_times(num_cores);
        
        crate::app::log_msg(&format!("CpuMonitor::new: Detected {} logical processors. Struct size: {}", num_cores, std::mem::size_of::<SystemProcessorPerformanceInformation>()));

        Self {
            num_cores,
            last_idle: idle,
            last_kernel: kernel,
            last_user: user,
            overall_usage: 0.0,
            last_cores,
            core_usages: vec![0.0; num_cores],
        }
    }

    pub fn refresh(&mut self) {
        // 1. Refresh overall usage via GetSystemTimes (reliable)
        let (idle, kernel, user) = Self::get_overall_times();
        let idle_diff = idle.wrapping_sub(self.last_idle);
        let kernel_diff = kernel.wrapping_sub(self.last_kernel);
        let user_diff = user.wrapping_sub(self.last_user);
        
        let total_diff = kernel_diff.wrapping_add(user_diff);
        if total_diff > 0 {
            let active_diff = total_diff.saturating_sub(idle_diff);
            self.overall_usage = (active_diff as f32 / total_diff as f32) * 100.0;
        }
        self.last_idle = idle;
        self.last_kernel = kernel;
        self.last_user = user;

        // 2. Refresh per-core usages via NtQuerySystemInformation
        let current_cores = Self::get_core_times(self.num_cores);
        for i in 0..self.num_cores {
            if i >= current_cores.len() || i >= self.last_cores.len() { break; }
            
            let last = &self.last_cores[i];
            let curr = &current_cores[i];
            
            // On Windows, KernelTime includes IdleTime. TotalTime = KernelTime + UserTime.
            let core_idle_diff = curr.idle_time.wrapping_sub(last.idle_time) as u64;
            let core_kernel_diff = curr.kernel_time.wrapping_sub(last.kernel_time) as u64;
            let core_user_diff = curr.user_time.wrapping_sub(last.user_time) as u64;
            
            let core_total_diff = core_kernel_diff.wrapping_add(core_user_diff);
            if core_total_diff > 0 {
                let core_active_diff = core_total_diff.saturating_sub(core_idle_diff);
                let usage = (core_active_diff as f32 / core_total_diff as f32) * 100.0;
                self.core_usages[i] = usage.clamp(0.0, 100.0);
            } else {
                self.core_usages[i] = 0.0;
            }
        }
        self.last_cores = current_cores;
    }

    pub fn get_overall_usage(&self) -> f32 {
        self.overall_usage
    }

    pub fn get_core_usages(&self) -> Vec<f32> {
        self.core_usages.clone()
    }
}
