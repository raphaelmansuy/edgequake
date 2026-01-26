#!/bin/bash
cd edgequake_webui
echo "Starting tsc with explainFiles..." > tsc_monitor.log
node --max-old-space-size=8192 node_modules/typescript/bin/tsc --noEmit --project tsconfig.json --explainFiles > tsc_output.txt 2>&1 &
TSC_PID=$!
echo "TSC PID: $TSC_PID" >> tsc_monitor.log

for i in {1..60}; do
  if ps -p $TSC_PID > /dev/null; then
    CPU=$(ps -p $TSC_PID -o %cpu | tail -1)
    MEM=$(ps -p $TSC_PID -o %mem | tail -1)
    echo "Time: ${i}s, CPU: $CPU%, MEM: $MEM%" >> tsc_monitor.log
  else
    echo "TSC finished or crashed." >> tsc_monitor.log
    break
  fi
  sleep 1
done

if ps -p $TSC_PID > /dev/null; then
  echo "TSC still running after 60s, killing it." >> tsc_monitor.log
  kill -9 $TSC_PID
fi
