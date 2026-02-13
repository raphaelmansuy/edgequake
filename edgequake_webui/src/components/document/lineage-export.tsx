/**
 * @fileoverview Lineage export buttons for downloading lineage data.
 *
 * OODA-24: Implements deliverable #4 "Export Capability:
 * Download complete lineage as JSON/CSV" from the mission spec.
 *
 * @implements FEAT0540 - Lineage data export
 * @see UC2144 - User exports document lineage for offline analysis
 */
"use client";

import { Button } from "@/components/ui/button";
import { exportDocumentLineage } from "@/lib/api/edgequake";
import { Download } from "lucide-react";
import { useCallback, useState } from "react";

interface LineageExportProps {
  documentId: string;
}

/**
 * Dropdown-free export buttons for downloading lineage as JSON or CSV.
 *
 * WHY: Two separate buttons are simpler and more accessible than a dropdown.
 * Users can see both options at once without additional clicks.
 */
export function LineageExport({ documentId }: LineageExportProps) {
  const [downloading, setDownloading] = useState<"json" | "csv" | null>(null);

  const handleExport = useCallback(
    async (format: "json" | "csv") => {
      setDownloading(format);
      try {
        await exportDocumentLineage(documentId, format);
      } catch (error) {
        console.error(`Failed to export lineage as ${format}:`, error);
      } finally {
        // WHY: Small delay so the user sees the loading state
        setTimeout(() => setDownloading(null), 500);
      }
    },
    [documentId],
  );

  return (
    <div className="flex gap-2">
      <Button
        variant="outline"
        size="sm"
        onClick={() => handleExport("json")}
        disabled={downloading !== null}
        className="flex-1"
      >
        <Download className="h-3 w-3 mr-1" />
        {downloading === "json" ? "Exporting…" : "JSON"}
      </Button>
      <Button
        variant="outline"
        size="sm"
        onClick={() => handleExport("csv")}
        disabled={downloading !== null}
        className="flex-1"
      >
        <Download className="h-3 w-3 mr-1" />
        {downloading === "csv" ? "Exporting…" : "CSV"}
      </Button>
    </div>
  );
}
