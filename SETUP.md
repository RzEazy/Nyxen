# Yeezy - Setup & Configuration Guide

## Quick Start

### 1. Set Your API Keys in Settings
Press **Ctrl+Space** to open Yeezy, click the **⚙ Settings** button:
- **Groq API key**: Get from https://console.groq.com
- **Cohere API key** (optional): Get from https://cohere.com

### 2. Set Your Sudo Password (For Package Installation)
In Settings, enter your system password under **🔐 System Access**:
- This allows Yeezy to install/remove packages automatically
- Password is stored locally (be cautious with shared systems)

### 3. Run as Independent Window
Instead of running from terminal, use the daemon script:
```bash
/home/rzy/Desktop/yeezy/yeezy-daemon.sh
```

Or run with daemon flag:
```bash
/home/rzy/Desktop/yeezy/target/release/yeezy --daemon
```

**Why daemon mode?**
- Window appears as a completely independent floating window
- NOT a child of the terminal/parent window
- Works like native system applications (Siri, Spotlight, etc.)

---

## Full Feature Guide

### Keyboard Hotkey
- **Default**: `Ctrl+Space`
- **Customizable**: In Settings
- Press to open/activate the window

### System Access (What Yeezy Can Do)

**Time & System Info**:
- Ask "What time is it?"
- Ask "What's my system info?"
- Ask "Show running processes"

**Package Management** (requires sudo password):
- "Install vlc"
- "Remove chromium"
- "Update all packages"
- "Search for vim"

**File Operations**:
- "Read /etc/hostname"
- "Create a file at ~/test.txt with content..."
- "List files in /home"
- "Delete /tmp/file.txt" (asks for confirmation)

**Shell Commands**:
- "Show disk usage"
- "What files are in my home directory?"
- "Run: ls -la /tmp"

**Web & Apps**:
- "Open google.com"
- "Search for weather in New York"
- "Open VLC player"

### Chat Settings
- **Language style**: Blunt, Friendly, Formal, etc.
- **Custom system prompt**: Modify AI behavior
- **Max tool iterations**: Prevents LLM loops (default: 3)
- **Memory length**: How many previous messages to remember
- **Confirmation required**: For dangerous commands like `rm -rf`

### Voice Settings
- **Voice input**: Requires Vosk model (optional)
- **Text-to-speech**: Uses espeak-ng by default
- **Speed & pitch**: Adjust voice output
- **Activation chime**: Sound when window opens

### Visual Settings
- **Opacity**: Window transparency (0.0-1.0)
- **Corner radius**: Window roundness
- **Font size**: Text size
- **Palette**: Color scheme (Monochrome, Ocean, Forest, etc.)
- **Orb size**: Animated AI orb size

---

## Troubleshooting

### Window Still Shows as Child/Dialog
Make sure you're using daemon mode:
```bash
/home/rzy/Desktop/yeezy/yeezy-daemon.sh
```

### Package Installation Fails
1. Check if sudo password is set in Settings
2. Verify password is correct:
   ```bash
   echo "your_password" | sudo -S apt-get install -y hello
   ```
3. If it fails, sudo might need NOPASSWD setup (see below)

### Password Prompt Timeout
Configure sudoers for passwordless commands:
```bash
sudo visudo
```

Add this line:
```
%sudo ALL=(ALL) NOPASSWD: /usr/bin/apt-get
```

Or allow all commands (⚠️ security risk):
```
%sudo ALL=(ALL) NOPASSWD: ALL
```

### Hotkey Not Working
- Check logs: `tail -f ~/.local/share/yeezy/yeezy.log`
- Verify hotkey isn't taken by another app
- Try changing hotkey in Settings

### Can't See Full Window
- Window is resizable - drag corners to expand
- Increase opacity if text is hard to read
- Change font size in Settings

---

## Advanced Usage

### Command Line Flags
```bash
yeezy                    # Run normally (attached to terminal)
yeezy --daemon          # Run as independent daemon
```

### Log Files
```bash
tail -f ~/.local/share/yeezy/yeezy.log
```

### Settings Database
```bash
~/.local/share/yeezy/yeezy.db
```

### Custom Models
Modify in Settings:
- **Groq models**: llama-3.3-70b-versatile, llama3-8b-8192, gemma2-9b-it
- **Cohere models**: command-r-plus-08-2024, command-r-08-2024

---

## Security Notes

⚠️ **Password Storage**:
- Sudo password is stored in local SQLite database
- Not encrypted - use on trusted systems only
- Consider using NOPASSWD sudoers instead (see above)

⚠️ **API Keys**:
- Store in a secure location
- Groq/Cohere keys are stored in local database

⚠️ **Dangerous Commands**:
- By default, asks for confirmation before running:
  - `rm -rf`
  - `mkfs`
  - `chmod 777`
- Can be disabled in Settings (not recommended)

---

## Tips & Tricks

1. **Ask about system time**: "What time is it right now?"
2. **Install packages easily**: "Install vim"
3. **Quick info**: "Show top 10 processes"
4. **Check network**: "What's my IP address?"
5. **File operations**: "Create a new file at ~/notes.txt with content..."

