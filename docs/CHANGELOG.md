---
title: 'Changelog (docs)'
description: "Product changelog for EdgeQuake releases."
---

# Changelog

Product and documentation release history lives in the **root** changelog. Do not keep a second list here.

→ **[CHANGELOG.md](../CHANGELOG.md)** (current product line: **v0.32.2**)

Each release section follows the same path, from an `[Unreleased]` entry to a published image.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    A["Notes under Unreleased"] --> B["make version-bump VERSION=X.Y.Z"]
    B --> C["Commit release: bump to vX.Y.Z"]
    C --> D["git tag vX.Y.Z and push"]
    D --> E["release-docker.yml builds images"]
    E --> F["Images published to GHCR"]
```

Pushing the version tag starts the Docker workflow, which publishes the images to GHCR.

When a docs-only change ships without a product bump, note it under `[Unreleased]` or the next version section in the root changelog.
