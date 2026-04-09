#!/bin/bash
# Run Nyx as a completely independent floating window

export DISPLAY=:0
export XAUTHORITY=$HOME/.Xauthority

# Start Nyx in background and capture its PID
/home/rzy/Desktop/nyx/target/release/nyx &
NYX_PID=$!

# Wait for window to appear and then set properties to make it independent
sleep 0.5

# Use wmctrl to ensure window is not in any group
wmctrl -l | grep -i "nyx\|Nyx" | awk '{print $1}' | while read WINDOW_ID; do
    # Remove any maximized state
    wmctrl -i -r "$WINDOW_ID" -b "remove,maximized_vert,maximized_horz"
    
    # Move window to bottom right (position set by app, but ensure it's visible)
    wmctrl -i -r "$WINDOW_ID" -e "0,1400,700,500,650"
done &

wait $NYX_PID
