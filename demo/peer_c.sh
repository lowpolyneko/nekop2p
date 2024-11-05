#!/bin/sh

cd peer_c

# wait for indexer
sleep 1

# premade command sequence
echo -e 'register\n8k.bin\nregister\n9k.bin\nregister\n10k.bin\ndownload\n1k.bin\ndownload\n2k.bin\ndownload\n3k.bin\ndownload\n4k.bin\ndownload\n5k.bin\ndownload\n6k.bin\ndownload\n7k.bin\nexit\n' | ../../target/release/nekopeer config.toml &
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
