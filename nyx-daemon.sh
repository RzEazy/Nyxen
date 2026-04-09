#!/bin/bash
# Nyxen Daemon Launcher
# Runs Nyxen as a completely independent floating window with no parent process

# Ensure DISPLAY and XAUTHORITY are set
if [ -z "$DISPLAY" ]; then
    export DISPLAY=:0
fi
if [ -z "$XAUTHORITY" ]; then
    export XAUTHORITY="$HOME/.Xauthority"
fi

# Get the actual binary path
NYXEN_BIN="$(dirname "$0")/target/release/nyxen"
if [ ! -f "$NYXEN_BIN" ]; then
    NYXEN_BIN="$(which nyxen 2>/dev/null)"
fi

# Run Nyxen with --daemon flag to detach from parent, preserving environment
exec setsid env DISPLAY="$DISPLAY" XAUTHORITY="$XAUTHORITY" "$NYXEN_BIN" --daemon &
disown
exit 0
