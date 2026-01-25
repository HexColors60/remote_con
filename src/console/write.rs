use windows::Win32::System::Console::{
    WriteConsoleInputW, INPUT_RECORD, KEY_EVENT_RECORD,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, FILE_GENERIC_WRITE,
    FILE_ATTRIBUTE_NORMAL,
};
use windows::Win32::Foundation::HANDLE;
use windows::core::PCWSTR;
use anyhow::{Result, anyhow};

/// Send a command string to the console input
pub fn send_command(command: &str) -> Result<()> {
    // Open CONIN$ for writing
    let conin = unsafe {
        CreateFileW(
            PCWSTR::from_raw(conin_wide().as_ptr()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;

    if conin.is_invalid() {
        return Err(anyhow!("Failed to open CONIN$"));
    }

    // Build input records for each character
    let mut input_records = Vec::new();

    for ch in command.chars() {
        // Key down event
        input_records.push(create_key_event(ch, true));
        // Key up event
        input_records.push(create_key_event(ch, false));
    }

    // Add Enter key (carriage return)
    input_records.push(create_key_event('\r', true));
    input_records.push(create_key_event('\r', false));

    // Write the input records
    unsafe {
        let mut events_written = 0;
        WriteConsoleInputW(
            conin,
            &input_records,
            &mut events_written,
        )
            .map_err(|e| anyhow!("Failed to write console input: {}", e.to_string()))?;
    }

    Ok(())
}

/// Send Ctrl+C to the console
pub fn send_ctrl_c() -> Result<()> {
    // Open CONIN$ for writing
    let conin = unsafe {
        CreateFileW(
            PCWSTR::from_raw(conin_wide().as_ptr()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;

    if conin.is_invalid() {
        return Err(anyhow!("Failed to open CONIN$"));
    }

    // Create a Ctrl+C event (Ctrl = VK_CONTROL, C = 0x43)
    let mut input_records = vec![
        create_ctrl_key_event(0x43, true, true),  // Ctrl+C down
        create_ctrl_key_event(0x43, false, true), // Ctrl+C up
    ];

    unsafe {
        let mut events_written = 0;
        WriteConsoleInputW(
            conin,
            &input_records,
            &mut events_written,
        )
            .map_err(|e| anyhow!("Failed to write Ctrl+C: {}", e.to_string()))?;
    }

    Ok(())
}

/// Send a control character to the console
pub fn send_control_char(code: u16) -> Result<()> {
    // Open CONIN$ for writing
    let conin = unsafe {
        CreateFileW(
            PCWSTR::from_raw(conin_wide().as_ptr()),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;

    if conin.is_invalid() {
        return Err(anyhow!("Failed to open CONIN$"));
    }

    // Create control character event (key down and key up)
    let mut input_records = vec![
        create_control_char_event(code, true),   // Key down
        create_control_char_event(code, false),  // Key up
    ];

    unsafe {
        let mut events_written = 0;
        WriteConsoleInputW(
            conin,
            &input_records,
            &mut events_written,
        )
            .map_err(|e| anyhow!("Failed to write control char: {}", e.to_string()))?;
    }

    Ok(())
}

/// Create a KEY_EVENT input record
fn create_key_event(ch: char, key_down: bool) -> INPUT_RECORD {
    let mut key_event = KEY_EVENT_RECORD::default();
    key_event.bKeyDown = key_down.into();
    key_event.dwControlKeyState = 0;
    key_event.wRepeatCount = 1;
    key_event.wVirtualKeyCode = 0;
    key_event.wVirtualScanCode = 0;
    key_event.uChar.UnicodeChar = ch as u16;

    let mut event = INPUT_RECORD::default();
    event.EventType = 1; // KEY_EVENT
    unsafe {
        event.Event.KeyEvent = key_event;
    }

    event
}

/// Create a Ctrl+Key event
fn create_ctrl_key_event(vk_code: u16, key_down: bool, ctrl: bool) -> INPUT_RECORD {
    let mut key_event = KEY_EVENT_RECORD::default();
    key_event.bKeyDown = key_down.into();
    key_event.wRepeatCount = 1;
    key_event.wVirtualKeyCode = vk_code;
    key_event.uChar.UnicodeChar = '\0' as u16;

    if ctrl {
        key_event.dwControlKeyState = windows::Win32::System::Console::LEFT_CTRL_PRESSED;
    }

    let mut event = INPUT_RECORD::default();
    event.EventType = 1; // KEY_EVENT
    unsafe {
        event.Event.KeyEvent = key_event;
    }

    event
}

/// Create a control character KEY_EVENT input record
fn create_control_char_event(code: u16, key_down: bool) -> INPUT_RECORD {
    let mut key_event = KEY_EVENT_RECORD::default();
    key_event.bKeyDown = key_down.into();
    key_event.dwControlKeyState = 0;
    key_event.wRepeatCount = 1;
    key_event.wVirtualKeyCode = code;
    key_event.wVirtualScanCode = 0;
    key_event.uChar.UnicodeChar = code; // Use the code as the Unicode character

    let mut event = INPUT_RECORD::default();
    event.EventType = 1; // KEY_EVENT
    unsafe {
        event.Event.KeyEvent = key_event;
    }

    event
}

/// Convert "CONIN$" to a wide null-terminated string
fn conin_wide() -> Vec<u16> {
    let mut s: Vec<u16> = "CONIN$".encode_utf16().collect();
    s.push(0);
    s
}
