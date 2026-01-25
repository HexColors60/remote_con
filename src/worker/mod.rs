use crossbeam_channel::{Sender, Receiver, bounded, unbounded, select};
use std::thread;
use std::time::{Duration, Instant};
use crate::console::{attach_to_console, detach_from_console, read_console_lines};

/// Message sent from worker to UI
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    /// New console output lines
    Output { lines: Vec<String>, timestamp: Instant },
    /// Error occurred
    Error(String),
    /// Status update
    Status(String),
    /// Disconnected from console
    Disconnected,
}

/// Message sent from UI to worker
#[derive(Debug, Clone)]
pub enum UiMessage {
    /// Attach to a console PID
    Attach(u32),
    /// Detach from current console
    Detach,
    /// Update polling interval
    SetInterval(Duration),
    /// Update number of lines to read
    SetLines(usize),
    /// Stop the worker
    Stop,
}

/// Configuration for the console worker
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub interval: Duration,
    pub lines: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(500),
            lines: 100,
        }
    }
}

/// Worker that polls console output in the background
pub struct ConsoleWorker {
    ui_tx: Sender<UiMessage>,
    worker_rx: Receiver<WorkerMessage>,
    _handle: thread::JoinHandle<()>,
}

impl ConsoleWorker {
    /// Create a new console worker
    pub fn new(config: WorkerConfig) -> Self {
        let (ui_tx, ui_rx) = unbounded::<UiMessage>();
        let (worker_tx, worker_rx) = unbounded::<WorkerMessage>();

        let handle = thread::spawn(move || {
            worker_main(config, ui_rx, worker_tx);
        });

        Self {
            ui_tx,
            worker_rx,
            _handle: handle,
        }
    }

    /// Send a message to the worker
    pub fn send(&self, msg: UiMessage) -> anyhow::Result<()> {
        self.ui_tx.send(msg)
            .map_err(|e| anyhow::anyhow!("Failed to send message to worker: {}", e))
    }

    /// Try to receive a message from the worker (non-blocking)
    pub fn try_recv(&self) -> Option<WorkerMessage> {
        self.worker_rx.try_recv().ok()
    }

    /// Receive a message from the worker (blocking with timeout)
    pub fn recv_timeout(&self, timeout: Duration) -> Option<WorkerMessage> {
        self.worker_rx.recv_timeout(timeout).ok()
    }
}

/// Main worker loop
fn worker_main(
    config: WorkerConfig,
    ui_rx: Receiver<UiMessage>,
    worker_tx: Sender<WorkerMessage>,
) {
    let mut current_pid: Option<u32> = None;
    let mut interval = config.interval;
    let mut lines = config.lines;
    let mut last_output: Option<String> = None;

    loop {
        // Check for UI messages
        match ui_rx.try_recv() {
            Ok(UiMessage::Attach(pid)) => {
                // Detach from previous if any
                if current_pid.is_some() {
                    let _ = detach_from_console();
                    current_pid = None;
                }

                // Try to attach to new PID
                match attach_to_console(pid) {
                    Ok(()) => {
                        current_pid = Some(pid);
                        last_output = None;
                        let _ = worker_tx.send(WorkerMessage::Status(format!("Attached to PID {}", pid)));
                    }
                    Err(e) => {
                        let _ = worker_tx.send(WorkerMessage::Error(format!("Failed to attach: {}", e)));
                    }
                }
            }
            Ok(UiMessage::Detach) => {
                if current_pid.is_some() {
                    let _ = detach_from_console();
                    current_pid = None;
                    last_output = None;
                    let _ = worker_tx.send(WorkerMessage::Status("Detached".to_string()));
                }
            }
            Ok(UiMessage::SetInterval(d)) => {
                interval = d;
            }
            Ok(UiMessage::SetLines(n)) => {
                lines = n;
            }
            Ok(UiMessage::Stop) => {
                if current_pid.is_some() {
                    let _ = detach_from_console();
                }
                break;
            }
            Err(_) => {}
        }

        // Poll console if attached
        if let Some(pid) = current_pid {
            // Reattach for this operation
            if let Err(e) = attach_to_console(pid) {
                let _ = worker_tx.send(WorkerMessage::Disconnected);
                current_pid = None;
                last_output = None;
                continue;
            }

            // Read console output
            match read_console_lines(lines) {
                Ok(output_lines) => {
                    let output = output_lines.join("\n");

                    // Only send if output changed
                    if last_output.as_ref() != Some(&output) {
                        last_output = Some(output.clone());
                        let _ = worker_tx.send(WorkerMessage::Output {
                            lines: output_lines,
                            timestamp: Instant::now(),
                        });
                    }
                }
                Err(e) => {
                    // Don't spam errors - only send if we haven't sent one recently
                    let _ = worker_tx.send(WorkerMessage::Error(format!("Read error: {}", e)));
                }
            }

            // Detach after reading
            let _ = detach_from_console();
        }

        // Sleep for the configured interval
        thread::sleep(interval);
    }
}
