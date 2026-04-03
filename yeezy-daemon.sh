#!/bin/bash
# Yeezy Daemon Launcher
# Runs Yeezy as a completely independent floating window with no parent process

# Ensure DISPLAY and XAUTHORITY are set
if [ -z "$DISPLAY" ]; then
    export DISPLAY=:0
fi
if [ -z "$XAUTHORITY" ]; then
    export XAUTHORITY="$HOME/.Xauthority"
fi

# Run Yeezy with --daemon flag to detach from parent
/home/rzy/Desktop/yeezy/target/release/yeezy --daemon
