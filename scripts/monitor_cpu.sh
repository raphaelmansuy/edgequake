#!/bin/bash
echo "Monitoring CPU usage for 5 minutes..."
for i in {1..60}
do
  echo "--- $(date) ---"
  ps -eo pcpu,pid,user,args | sort -k 1 -r | head -n 5
  sleep 5
done
