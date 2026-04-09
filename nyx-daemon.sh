#!/bin/bash
# Nyx Daemon Launcher
# Runs Nyx as a completely independent floating window with no parent process

# Ensure DISPLAY and XAUTHORITY are set
if [ -z "$DISPLAY" ]; then
    export DISPLAY=:0
fi
if [ -z "$XAUTHORITY" ]; then
    export XAUTHORITY="$HOME/.Xauthority"
fi

# Get the actual binary path
NYX_BIN="$(dirname "$0")/target/release/nyx"
if [ ! -f "$NYX_BIN" ]; then
    NYX_BIN="$(which nyx 2>/dev/null)"
fi

# Run Nyx with --daemon flag to detach from parent, preserving environment
exec setsid env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" "$NYX_BIN" --daemon &
disown
exit 0
