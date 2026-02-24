### Background cmd.exe control (Windows 10)

- In Windows 10, a cmd.exe console may be **non-visible** and cannot be reached via
  **Alt-Tab**, making direct keyboard input impossible.
- This tool allows selecting a target **cmd.exe by PID** and attaching to its
  console **without bringing the window to the foreground**.
- When using **Claude Code**, you can attach to the background cmd.exe, send
  commands, save the conversation to `talk_<timestamp>.txt`, then send `/quit`.
- **Important:** You must press **Ctrl-M**.
  Sending text commands alone does **not** produce a correct newline in Claude Code.
- The **Ctrl-M** button sends a newline (**CR / Enter**) to the target cmd.exe.
