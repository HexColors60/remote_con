use windows::Win32::System::Console::{AttachConsole, FreeConsole, GetConsoleWindow};
use windows::core::Error as WinError;
use anyhow::{Result, anyhow};

/// Attach to a process's console
pub fn attach_to_console(pid: u32) -> Result<()> {
    unsafe {
        // Free any current console attachment first
        let _ = FreeConsole();

        // Attach to the target process's console
        AttachConsole(pid)
            .map_err(|e| anyhow!("Failed to attach to console PID {}: {}", pid, e.to_string()))?;
    }
    Ok(())
}

/// Detach from the current console
pub fn detach_from_console() -> Result<()> {
    unsafe {
        FreeConsole()
            .map_err(|e| anyhow!("Failed to detach from console: {}", e.to_string()))?;
    }
    Ok(())
}

/// Check if we're currently attached to a console
pub fn is_attached() -> bool {
    unsafe {
        !GetConsoleWindow().is_invalid()
    }
}

/// Scoped console attachment that auto-detaches when dropped
pub struct ConsoleAttachment {
    pid: u32,
    attached: bool,
}

impl ConsoleAttachment {
    /// Try to attach to the console of the specified process
    pub fn new(pid: u32) -> Result<Self> {
        attach_to_console(pid)?;
        Ok(Self {
            pid,
            attached: true,
        })
    }

    /// Get the PID we're attached to
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Check if successfully attached
    pub fn is_attached(&self) -> bool {
        self.attached
    }
}

impl Drop for ConsoleAttachment {
    fn drop(&mut self) {
        if self.attached {
            // Best effort detach
            let _ = detach_from_console();
            self.attached = false;
        }
    }
}
