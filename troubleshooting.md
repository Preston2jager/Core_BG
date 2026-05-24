# Troubleshooting & Error Resolution Guide

This document records critical errors encountered during the development and execution of the live wallpaper application and provides step-by-step solutions to resolve them.

---

## 1. PowerShell Script Execution Disabled (SecurityError / UnauthorizedAccess)

### Symptom
Every time a PowerShell terminal command runs, the following error is displayed:
```text
. : File C:\Users\Sang\Documents\WindowsPowerShell\profile.ps1 cannot be loaded because running scripts is disabled on 
this system. For more information, see about_Execution_Policies at https:/go.microsoft.com/fwlink/?LinkID=135170.
At line:1 char:3
+ . 'C:\Users\Sang\Documents\WindowsPowerShell\profile.ps1'
+   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : SecurityError: (:) [], PSSecurityException
    + FullyQualifiedErrorId : UnauthorizedAccess
```

### Cause
Windows client systems default to the `Restricted` execution policy, which prevents any PowerShell scripts (including the automatic user profile `profile.ps1` script) from loading.

### Solution
You can fix this by changing the execution policy for your user account or bypassing it.

*   **Option A: Allow script execution for your user account (Recommended)**
    Open an Administrator PowerShell terminal and run:
    ```powershell
    Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
    ```
    This allows local scripts (like your profile) and signed remote scripts to run, resolving the error permanently for your session.

*   **Option B: Bypass policy for individual executions**
    If you are running scripts in a CI/CD environment or one-off runs where policy configuration is restricted, execute PowerShell with the `-ExecutionPolicy Bypass` argument:
    ```powershell
    powershell -ExecutionPolicy Bypass -File .\your_script.ps1
    ```

---

## 2. DX12 / Vulkan Graphics Driver Hang (`wgpu` Initialization Freeze)

### Symptom
The wallpaper application starts and logs `Initializing wgpu context...` or `Requesting GPU adapter...` and then hangs indefinitely. No panic occurs, and the CPU usage remains flat.

### Cause
When Rust applications are compiled with `#![windows_subsystem = "windows"]` to hide the console window, the Windows OS closes or nullifies standard streams (`stdout`, `stderr`, `stdin`).
Many GPU driver loaders (especially DirectX 12 and Vulkan loaders) attempt to write debug/discovery logs to standard streams. If these streams are nullified or closed, the driver's standard library file-descriptor lock hangs indefinitely, causing the graphics initialization to freeze.

### Solution
Configure the program to compile using the Windows GUI subsystem (which prevents any console window from opening or showing in the taskbar) and redirect standard handles to prevent driver hangs:

1.  **Set Windows Subsystem**: Add `#![windows_subsystem = "windows"]` at the very top of your main entry point file (usually `src/main.rs`).

2.  **Redirect Standard Handles**: Immediately at the start of `main()`, redirect `stdout`, `stderr`, and `stdin` to the Windows `"NUL"` device so that driver logs write safely to null handles without locking:
    ```rust
    unsafe fn redirect_std_handles_to_nul() {
        #[link(name = "msvcrt")]
        extern "C" {
            fn _open(filename: *const u8, oflag: i32, pmode: i32) -> i32;
            fn _dup2(fd1: i32, fd2: i32) -> i32;
        }

        let nul_name = window::to_wide("NUL");
        let h_nul = CreateFileW(
            nul_name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        );
        if h_nul != -1i32 as isize {
            SetStdHandle(STD_INPUT_HANDLE, h_nul);
            SetStdHandle(STD_OUTPUT_HANDLE, h_nul);
            SetStdHandle(STD_ERROR_HANDLE, h_nul);
        }

        let fd = _open(b"NUL\0".as_ptr(), 2, 0);
        if fd >= 0 {
            _dup2(fd, 0);
            _dup2(fd, 1);
            _dup2(fd, 2);
        }
    }
    ```

    By doing this, the OS does not allocate a command console (preventing the empty cmd window in the taskbar or desktop flashes), and standard handles are initialized cleanly to `"NUL"`, preventing driver hangs.

---

## 3. WorkerW Desktop Overlay Thread Hangs (COM / Registry Polling)

### Symptom
When parented under the desktop background manager (`WorkerW`), calls to poll system statistics (e.g. `sysinfo` queries, WMI commands, or Windows Performance Registry queries) stutter, block, or drop the rendering frame rate down to 10 FPS or lower.

### Cause
The Windows desktop overlay layer (`WorkerW`/`Progman`) runs on a high-priority shell thread. Standard registry queries or COM synchronization loops used by high-level system libraries can lock or conflict with the desktop thread's message queue, introducing massive delays.

### Solution
Avoid heavy high-level polling APIs. Fetch system times directly using low-level kernel APIs:
*   **Overall CPU**: Use Win32 `GetSystemTimes` which runs in sub-microseconds without thread overhead.
*   **Per-Core CPU**: Call `NtQuerySystemInformation` from `ntdll.dll` using system info class `21` (`SystemProcessorPerformanceInformation`), which queries the kernel directly.
*   **Check interval**: Limit system queries to 1-second intervals, decoupling them completely from the rendering frame rates (which run at 60 FPS).
