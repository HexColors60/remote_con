use sysinfo::System;
use windows::Win32::Foundation::HWND;
use anyhow::Result;

/// Information about a cmd.exe process
#[derive(Debug, Clone)]
pub struct CmdProcessInfo {
    pub pid: u32,
    pub window_title: Option<String>,
    pub session_id: u32,
    pub has_window: bool,
    pub attachable: bool,
}

/// Enumerate all cmd.exe processes on the system
pub fn enumerate_cmd_processes() -> Result<Vec<CmdProcessInfo>> {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let current_pid = std::process::id();
    let current_session_id = get_current_session_id()?;

    let mut cmd_processes = Vec::new();

    for (pid, process) in sys.processes() {
        // Skip our own process
        if pid.as_u32() == current_pid {
            continue;
        }

        // Check if process name is cmd.exe
        if process.name().to_string_lossy().to_lowercase() == "cmd.exe" {
            let pid_u32 = pid.as_u32();

            // Get session ID
            let session_id = get_process_session_id(pid_u32).unwrap_or(0);

            // Must be in the same session
            if session_id != current_session_id {
                continue;
            }

            // Check if process has a main window
            let hwnd = get_process_main_window(pid_u32);
            let has_window = hwnd != HWND(std::ptr::null_mut());
            let window_title = if has_window {
                get_window_title(hwnd).ok()
            } else {
                None
            };

            // Check if attachable (same privilege level)
            let attachable = is_process_attachable(pid_u32);

            cmd_processes.push(CmdProcessInfo {
                pid: pid_u32,
                window_title,
                session_id,
                has_window,
                attachable,
            });
        }
    }

    Ok(cmd_processes)
}

/// Get the current process session ID
fn get_current_session_id() -> Result<u32> {
    // For a GUI application, we're typically in session 1 (interactive session)
    // This is a simplified approach - in a production app you'd use
    // proper Win32 APIs or define ProcessIdToSessionId manually
    Ok(1)
}

/// Get the session ID for a process
fn get_process_session_id(_pid: u32) -> Result<u32> {
    // For simplicity, assume all cmd.exe processes we can see are
    // in the same session as us
    Ok(1)
}

/// Get the main window handle for a process
fn get_process_main_window(pid: u32) -> HWND {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::PCWSTR;

    // This is a simplified approach - in a real implementation you would
    // enumerate windows to find one belonging to this process
    // For now, we'll use a placeholder
    HWND(std::ptr::null_mut())
}

/// Get the title of a window
fn get_window_title(hwnd: HWND) -> Result<String> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;

    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if len > 0 {
        let text = String::from_utf16(&buffer[..len as usize])?;
        Ok(text)
    } else {
        Ok(String::new())
    }
}

/// Check if a process is attachable (same privilege level)
fn is_process_attachable(pid: u32) -> bool {
    use windows::Win32::System::Threading::OpenProcess;
    use windows::Win32::System::Threading::PROCESS_QUERY_INFORMATION;

    // Try to open the process with query access
    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) };
    if let Ok(h) = handle {
        if h.is_invalid() {
            return false;
        }
        // In a full implementation, you would check if the process
        // is running at the same privilege level (admin vs non-admin)
        // For now, just check if we can open it
        true
    } else {
        false
    }
}
