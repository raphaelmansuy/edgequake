"use client";

/**
 * SPEC-183 — the Evidence decision on the Create workspace dialog.
 *
 * `embedding_modalities` is fixed at create (SPEC-180 LAW-180-5). The dialog asked name,
 * retrieval and models, so every workspace it made was `text`, and images could never be
 * uploaded — permanently, because the column does not change. The question has to be asked
 * here or it cannot be asked at all.
 *
 * Presentational only: the rules live in `lib/admin/create-evidence.ts`.
 */

import { OptionCard } from "@/components/onboarding/option-card";
import { Label } from "@/components/ui/label";
import {
  NO_IMAGE_MODEL_REASON,
  evidenceChoices,
  type RetrievalChoice,
} from "@/lib/admin/create-evidence";
import type { EmbeddingModalities } from "@/constants/rag-profile";
import { AlertTriangle, ImageOff } from "lucide-react";

export interface CreateWorkspaceEvidenceProps {
  retrieval: RetrievalChoice;
  value: EmbeddingModalities;
  onChange: (value: EmbeddingModalities) => void;
  /** False when the offer has no embedding model with `supports_vision`. */
  imageModelOffered: boolean;
  /** The pair an image workspace will actually use, so the sentence can name it. */
  imageModelLabel?: string | null;
  /** The pinned embedding model cannot read images (the server would answer 422). */
  pairCannotReadImages: boolean;
  /** The offer is still loading; say so instead of claiming nothing is offered. */
  isLoading?: boolean;
}

export function CreateWorkspaceEvidence({
  retrieval,
  value,
  onChange,
  imageModelOffered,
  imageModelLabel,
  pairCannotReadImages,
  isLoading = false,
}: CreateWorkspaceEvidenceProps) {
  const choices = evidenceChoices(retrieval);
  /**
   * A graph workspace can only be `text`, so that is the card that must read as chosen. Showing
   * the remembered image choice as *unselected* would leave no radio checked at all, which reads
   * as "nothing decided" when the workspace has in fact been decided.
   */
  const shownValue: EmbeddingModalities = retrieval === "graph" ? "text" : value;
  const imagesChosen = retrieval === "normal" && shownValue === "text,image";

  return (
    <div className="space-y-1.5">
      <div className="flex items-baseline justify-between gap-2">
        <Label className="text-[13px] font-medium">Evidence</Label>
        <span className="text-[11.5px] text-muted-foreground">fixed at create</span>
      </div>

      <div
        role="radiogroup"
        aria-label="Evidence"
        className="grid gap-2 sm:grid-cols-2"
        data-testid="create-workspace-evidence"
      >
        {choices.map((choice) => (
          <OptionCard
            key={choice.value}
            selected={shownValue === choice.value && !choice.disabled}
            onSelect={() => onChange(choice.value)}
            disabled={choice.disabled}
            title={choice.value === "text" ? "Text" : "Text and images"}
            data-testid={
              choice.value === "text"
                ? "create-workspace-evidence-text"
                : "create-workspace-evidence-text-image"
            }
          >
            <p className="text-[11.5px] leading-relaxed text-muted-foreground">
              {choice.value === "text"
                ? "Chunks are embedded as text. Documents, PDFs and notes."
                : "Pictures are embedded too, so they can be found by description or by another picture."}
            </p>
            {choice.reason ? (
              <p className="text-[11.5px] leading-snug text-muted-foreground">{choice.reason}</p>
            ) : null}
          </OptionCard>
        ))}
      </div>

      {imagesChosen && isLoading ? (
        <p className="text-[11.5px] text-muted-foreground" data-testid="create-workspace-evidence-checking">
          Checking which embedding models this organization offers…
        </p>
      ) : null}

      {imagesChosen && !isLoading && !imageModelOffered ? (
        <p
          role="status"
          data-testid="create-workspace-evidence-no-model"
          className="flex items-start gap-1.5 text-[11.5px] leading-snug text-amber-600 dark:text-amber-400"
        >
          <ImageOff aria-hidden className="mt-px h-3.5 w-3.5 shrink-0" />
          {NO_IMAGE_MODEL_REASON}
        </p>
      ) : null}

      {imagesChosen && imageModelOffered && pairCannotReadImages ? (
        <p
          role="status"
          data-testid="create-workspace-evidence-pin-model"
          className="flex items-start gap-1.5 text-[11.5px] leading-snug text-amber-600 dark:text-amber-400"
        >
          <AlertTriangle aria-hidden className="mt-px h-3.5 w-3.5 shrink-0" />
          The inherited embedding model reads text only. Pick an image-capable embedding model under
          Models.
        </p>
      ) : null}

      {imagesChosen && imageModelOffered && !pairCannotReadImages && imageModelLabel ? (
        <p
          className="text-[11.5px] leading-snug text-muted-foreground"
          data-testid="create-workspace-evidence-model"
        >
          Images will be embedded with <span className="font-mono">{imageModelLabel}</span>.
        </p>
      ) : null}

    </div>
  );
}
