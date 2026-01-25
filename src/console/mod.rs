pub mod attach;
pub mod read;
pub mod write;

pub use attach::{attach_to_console, detach_from_console, is_attached, ConsoleAttachment};
pub use read::{read_console_lines, read_all_console};
pub use write::{send_command, send_ctrl_c, send_control_char};
