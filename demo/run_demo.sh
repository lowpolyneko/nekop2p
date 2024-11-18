#!/bin/sh
cargo build --release

# remove downloads if they already exist
rm -f peer_a/{5..10}k.bin peer_b/{1..4}k.bin peer_b/{8..10}k.bin peer_c/{1..7}k.bin **/*.meta

tmux new-session -d -s nekop2p './indexer.sh'
tmux split-window -h
tmux send './peer_a.sh' ENTER
tmux split-window -h
tmux send './peer_b.sh' ENTER
tmux split-window -h
tmux send './peer_c.sh' ENTER
tmux a -t nekop2p
