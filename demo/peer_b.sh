#!/bin/sh

cd peer_b

# wait for indexer
sleep 1

# premade command sequence
echo -e 'register\n5k.bin\nregister\n6k.bin\nregister\n7k.bin\ndownload\n1k.bin\ndownload\n2k.bin\ndownload\n3k.bin\ndownload\n4k.bin\ndownload\n8k.bin\ndownload\n9k.bin\ndownload\n10k.bin\nexit\n' | ../../target/release/nekopeer config.toml &
PEER_PID=$!

sleep 1

# Register owned binaries
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1

# Download binaries
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 1
kill -2 $PEER_PID
sleep 3

# Quit
kill -2 $PEER_PID
wait

ls -lh
