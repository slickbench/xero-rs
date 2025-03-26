#!/bin/bash
export RUST_LOG=warn
echo "Running tests in a loop until rate limit is hit..." | tee -a log.txt
while true; do
  echo "---------------------------------------------" | tee -a log.txt
  echo "Starting test run at $(date)" | tee -a log.txt
  output=$(cargo test -- --nocapture 2>&1)
  echo "$output" | tee -a log.txt
  if echo "$output" | grep -q "Rate limit exceeded"; then
    echo "RATE LIMIT DETECTED! Stopping." | tee -a log.txt
    echo "$output" | grep "Rate limit exceeded" | tee -a log.txt
    break
  fi
  echo "No rate limit warning yet. Continuing..." | tee -a log.txt
  echo "---------------------------------------------" | tee -a log.txt
  sleep 1
done
