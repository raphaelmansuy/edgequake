# LENS — Platform

Parent: [04 Gap matrix](../04-gap-matrix.md) · Facts: [02](../02-platform-facts.md)

## What the host allows

An App is a source directory on Ubuntu 22.04. Databricks installs Python and, if present, Node, then runs `command`. The process must listen on `DATABRICKS_APP_PORT`. Files over 10 MB fail the deploy.

## What that does to EdgeQuake

The server is a Rust binary. The Web UI is a second process. v1 ships the API only. A launcher of a few kilobytes is the App. It copies `DATABRICKS_APP_PORT` into `PORT`, fetches the binary from a Volume, and execs it on `0.0.0.0`.

Compute sizes stop at 12 vCPU and 48 GB. PDF extraction and graph ingest are the workloads that care. Medium (2 vCPU, 6 GB) is the default and is the wrong size for ingest. Start on Large or XLarge and measure. This study did not run ingest on an App.

## Gaps

G10, G11, G12 in [04](../04-gap-matrix.md).
