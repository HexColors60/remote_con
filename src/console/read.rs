use windows::Win32::System::Console::{
    GetConsoleScreenBufferInfo, ReadConsoleOutputCharacterW,
    CONSOLE_SCREEN_BUFFER_INFO,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, FILE_GENERIC_READ,
    FILE_ATTRIBUTE_NORMAL,
};
use windows::Win32::Foundation::HANDLE;
use windows::core::PCWSTR;
use anyhow::{Result, anyhow};

/// Read the last N lines from the console screen buffer
pub fn read_console_lines(num_lines: usize) -> Result<Vec<String>> {
    // Open CONOUT$ for reading
    let conout = unsafe {
        CreateFileW(
            PCWSTR::from_raw(conout_wide().as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;

    if conout.is_invalid() {
        return Err(anyhow!("Failed to open CONOUT$"));
    }

    // Get console screen buffer info
    let mut csbi = CONSOLE_SCREEN_BUFFER_INFO::default();
    unsafe {
        GetConsoleScreenBufferInfo(conout, &mut csbi)
            .map_err(|e| anyhow!("Failed to get console buffer info: {}", e.to_string()))?;
    }

    // Get the cursor position (current line)
    let cursor_y = csbi.dwCursorPosition.Y;
    let buffer_width = csbi.dwSize.X as usize;
    let buffer_height = csbi.dwSize.Y as usize;

    // Calculate the starting line
    let start_y = if cursor_y >= num_lines as i16 {
        cursor_y - num_lines as i16
    } else {
        0
    };

    let lines_to_read = (cursor_y - start_y + 1) as usize;
    let mut lines = Vec::with_capacity(lines_to_read);

    // Read each line
    for y in start_y..=cursor_y {
        let line = read_line(conout, y as i16, buffer_width)?;
        lines.push(line);
    }

    Ok(lines)
}

/// Read a single line from the console buffer
fn read_line(conout: HANDLE, y: i16, width: usize) -> Result<String> {
    let mut buffer = vec![0u16; width];

    unsafe {
        let coord = windows::Win32::System::Console::COORD { X: 0, Y: y };
        let mut chars_read = 0;

        ReadConsoleOutputCharacterW(
            conout,
            &mut buffer,
            coord,
            &mut chars_read,
        )
        .map_err(|e| anyhow!("Failed to read console output: {}", e.to_string()))?;
    }

    // Convert to string and trim trailing nulls and spaces
    let text = String::from_utf16_lossy(&buffer)
        .trim_end_matches('\0')
        .trim_end()
        .to_string();

    Ok(text)
}

/// Convert "CONOUT$" to a wide null-terminated string
fn conout_wide() -> Vec<u16> {
    let mut s: Vec<u16> = "CONOUT$".encode_utf16().collect();
    s.push(0);
    s
}

/// Read all available console content (for debugging)
pub fn read_all_console() -> Result<String> {
    let lines = read_console_lines(500)?;
    Ok(lines.join("\n"))
}
