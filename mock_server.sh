#!/bin/bash
echo "[Server] Starting mock server..."
echo "[Server] Done loading."

while read line; do
  echo "[Server] Received: $line"
  if [ "$line" == "stop" ]; then
    echo "[Server] Stopping..."
    sleep 2
    exit 0
  fi
  if [ "$line" == "/stop" ]; then
    echo "[Server] Stopping..."
    sleep 2
    exit 0
  fi
done
