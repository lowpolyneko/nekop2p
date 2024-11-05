#!/bin/sh

cd peer_a

# wait for indexer
sleep 1

# premade command sequence
echo -e 'register\n1k.bin\nregister\n2k.bin\nregister\n3k.bin\nregister\n4k.bin\ndownload\n5k.bin\ndownload\n6k.bin\ndownload\n7k.bin\ndownload\n8k.bin\ndownload\n9k.bin\ndownload\n10k.bin\nexit\n' | ../../target/release/nekopeer config.toml &
PEER_PID=$!

sleep 1

# Register owned binaries
kill -2 $PEER_PID
sleep 1
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
sleep 3

# Quit
kill -2 $PEER_PID
wait

ls -lh
