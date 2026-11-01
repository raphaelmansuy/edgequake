"use client";

/**
 * SPEC-181 — "Tune later".
 *
 * Every setting the 8-step wizard asked for lives here, collapsed, each row printing the value
 * it would inherit ([LAW-181-13](../../../../specs/181-admin/01-first-principles.md)). Rows are
 * controlled by a flat `values` map so the parent submits exactly what the operator changed and
 * omits the rest — an omitted field stays inherited on the server (LAW-123-8).
 *
 * This is the DRY split from the wizard: the *fields* are the same ideas, the *order* is gone.
 */

import { InheritLine } from "@/components/admin/primitives";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { ChevronRight } from "lucide-react";
import { useId } from "react";

export interface TuneLaterValues {
  [key: string]: string;
}

export interface TuneLaterInherited {
  extraction_language?: string | null;
}

export function TuneLater({
  open,
  onOpenChange,
  count,
  values,
  onChange,
  inherited,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** How many rows the operator has changed (shown as a badge). */
  count: number;
  values: TuneLaterValues;
  onChange: (key: string, value: string) => void;
  inherited?: TuneLaterInherited | null;
}) {
  const panelId = useId();

  return (
    <div className="rounded-lg border" data-testid="tune-later">
      <button
        type="button"
        onClick={() => onOpenChange(!open)}
        aria-expanded={open}
        aria-controls={panelId}
        data-testid="tune-later-toggle"
        className={cn(
          "flex w-full items-center gap-2 rounded-lg px-3 py-2.5 text-left transition-colors",
          "hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        )}
      >
        <ChevronRight
          aria-hidden
          className={cn("h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform", open && "rotate-90")}
        />
        <span className="text-[13px] font-medium text-foreground">Tune later</span>
        <span className="min-w-0 flex-1 truncate text-[12px] text-muted-foreground">
          Parsing · Chunking · Extract budget · Entity types · Language · Decision engine
        </span>
        <span className="shrink-0 text-[11.5px] tabular-nums text-muted-foreground">
          {count > 0 ? `${count} changed` : "all inherited"}
        </span>
      </button>

      {open ? (
        <div id={panelId} className="space-y-3 border-t px-3 py-3" data-testid="tune-later-panel">
          <Row label="PDF parser" hint="How PDFs become text.">
            <Select
              value={values.pdf_parser_backend ?? "inherit"}
              onValueChange={(v) => onChange("pdf_parser_backend", v === "inherit" ? "" : v)}
            >
              <SelectTrigger size="sm" className="w-[9.5rem]" data-testid="tune-pdf-parser">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="inherit">Inherit</SelectItem>
                <SelectItem value="vision">Vision</SelectItem>
                <SelectItem value="edgeparse">EdgeParse</SelectItem>
                <SelectItem value="edgeparse-ocr">EdgeParse OCR</SelectItem>
                <SelectItem value="auto">Auto</SelectItem>
              </SelectContent>
            </Select>
          </Row>

          <Row label="Language" hint="Language of the extracted knowledge.">
            <Select
              value={values.extraction_language ?? "inherit"}
              onValueChange={(v) => onChange("extraction_language", v === "inherit" ? "" : v)}
            >
              <SelectTrigger size="sm" className="w-[9.5rem]" data-testid="tune-language">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="inherit">
                  Inherit{inherited?.extraction_language ? ` (${inherited.extraction_language})` : ""}
                </SelectItem>
                <SelectItem value="English">English</SelectItem>
                <SelectItem value="French">French</SelectItem>
                <SelectItem value="German">German</SelectItem>
                <SelectItem value="Spanish">Spanish</SelectItem>
                <SelectItem value="Chinese">Chinese</SelectItem>
                <SelectItem value="Japanese">Japanese</SelectItem>
              </SelectContent>
            </Select>
          </Row>

          <Row label="Chunking" hint="How documents split before extraction.">
            <div className="flex items-center gap-1.5">
              <Input
                type="number"
                min={128}
                max={8192}
                placeholder="1200"
                value={values.chunk_token_size ?? ""}
                onChange={(e) => onChange("chunk_token_size", e.target.value)}
                className="h-7 w-[4.5rem] text-[12px] tabular-nums"
                data-testid="tune-chunk-size"
                aria-label="Chunk size"
              />
              <span className="text-[11.5px] text-muted-foreground">/</span>
              <Input
                type="number"
                min={0}
                max={1024}
                placeholder="100"
                value={values.chunk_overlap_token_size ?? ""}
                onChange={(e) => onChange("chunk_overlap_token_size", e.target.value)}
                className="h-7 w-[4rem] text-[12px] tabular-nums"
                data-testid="tune-chunk-overlap"
                aria-label="Chunk overlap"
              />
            </div>
          </Row>

          <Row label="Extract budget" hint="Entity and relation caps per response.">
            <div className="flex items-center gap-1.5">
              <Input
                type="number"
                min={1}
                max={200}
                placeholder="40"
                value={values.extract_max_entities ?? ""}
                onChange={(e) => onChange("extract_max_entities", e.target.value)}
                className="h-7 w-[4rem] text-[12px] tabular-nums"
                data-testid="tune-max-entities"
                aria-label="Max entities"
              />
              <span className="text-[11.5px] text-muted-foreground">/</span>
              <Input
                type="number"
                min={1}
                max={500}
                placeholder="100"
                value={values.extract_max_records ?? ""}
                onChange={(e) => onChange("extract_max_records", e.target.value)}
                className="h-7 w-[4.5rem] text-[12px] tabular-nums"
                data-testid="tune-max-records"
                aria-label="Max records"
              />
            </div>
          </Row>

          <Row label="Extraction mode" hint="LLM extraction, or the decision engine.">
            <Select
              value={values.extraction_mode ?? "inherit"}
              onValueChange={(v) => onChange("extraction_mode", v === "inherit" ? "" : v)}
            >
              <SelectTrigger size="sm" className="w-[9.5rem]" data-testid="tune-extraction-mode">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="inherit">Inherit</SelectItem>
                <SelectItem value="llm">LLM</SelectItem>
                <SelectItem value="decision">Decision</SelectItem>
              </SelectContent>
            </Select>
          </Row>

          <div className="flex items-center justify-between gap-3 border-t pt-2">
            <p className="text-[11.5px] text-muted-foreground">
              Anything you leave alone keeps its inherited value.
            </p>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 text-[12px]"
              onClick={() => {
                for (const key of Object.keys(values)) onChange(key, "");
              }}
              data-testid="tune-reset"
            >
              Reset
            </Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid grid-cols-1 items-center gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:gap-3">
      <div className="min-w-0">
        <Label className="text-[12.5px] font-medium">{label}</Label>
        <p className="mt-0.5 text-[11.5px] leading-snug text-muted-foreground">{hint}</p>
      </div>
      <div className="flex items-center justify-end">{children}</div>
    </div>
  );
}

/** Re-exported so a caller that only needs the inheritance line does not import two modules. */
export { InheritLine };
