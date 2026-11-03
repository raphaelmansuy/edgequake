"use client";

/**
 * SPEC-181 W-03 — create a workspace in three decisions.
 *
 * The pre-181 wizard asked eight steps (name, models, parsing, workspace, chunking, budget,
 * extraction, review) for a workspace whose answer to six of them is "inherit" — the default
 * ([F-181-03](../../../../specs/181-admin/04-findings.md)). This dialog asks the three that
 * change what the workspace *is*, prints the inherited values so inheritance stays a visible
 * decision, and puts every other knob in `TuneLater` (LAW-181-4, LAW-181-13).
 */

import { ChoiceCard, ScopeDialog } from "@/components/admin/scope-dialog";
import { CreateWorkspaceEvidence } from "@/components/admin/create-workspace-evidence";
import { MeterRow } from "@/components/admin/primitives";
import { TuneLater } from "@/components/admin/tune-later";
import { WorkspaceModelsPanel } from "@/components/admin/workspace-models-panel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAccess } from "@/lib/access/use-access";
import {
  applyCreatedScope,
  classifyCreateFailure,
  settleAfterCreate,
  useCreateWorkspace,
} from "@/hooks/use-create-scope";
import { useInheritedModelDefaults } from "@/hooks/use-inherited-model-defaults";
import { useWizardEligibility } from "@/hooks/use-wizard-eligibility";
import {
  blockedModelSentence,
  emptyModelDraft,
  hasAnyPick,
  inheritedRoles,
  refusedPickedRoles,
  toModelPayload,
  type ModelRole,
  type WorkspaceModelDraft,
} from "@/lib/admin/workspace-model-draft";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { CreateWorkspaceRequest } from "@/types";
import { slugifyName } from "@/lib/onboarding/slug";
import { cn } from "@/lib/utils";
import Link from "next/link";
import { AlertCircle } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import {
  imageEmbeddingPin,
  imagesUnavailable,
  resolveCreateModalities,
  type RetrievalChoice,
} from "@/lib/admin/create-evidence";
import { useImageEmbeddingOffer } from "@/hooks/use-image-embedding-offer";
import type { EmbeddingModalities } from "@/constants/rag-profile";
import { useMemo, useState } from "react";
import { toast } from "sonner";

type Retrieval = RetrievalChoice;

export interface CreateWorkspaceDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tenantId?: string | null;
  onCreated?: (workspaceId: string, slug: string) => void;
}

export function CreateWorkspaceDialog({
  open,
  onOpenChange,
  tenantId,
  onCreated,
}: CreateWorkspaceDialogProps) {
  const storeTenantId = useTenantStore((s) => s.selectedTenantId);
  const target = tenantId ?? storeTenantId;
  const { access } = useAccess();
  const inherited = useInheritedModelDefaults(target);
  const create = useCreateWorkspace();
  const qc = useQueryClient();

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [retrieval, setRetrieval] = useState<Retrieval>("graph");
  // SPEC-183: evidence is fixed at create like the retrieval type, so it is asked here too.
  const [evidence, setEvidence] = useState<EmbeddingModalities>("text");
  const [models, setModels] = useState<WorkspaceModelDraft>(emptyModelDraft);
  const [openRole, setOpenRole] = useState<ModelRole | null>(null);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [advanced, setAdvanced] = useState<Record<string, string>>({});
  const [error, setError] = useState<{ kind: string; message: string } | null>(null);
  /**
   * A pristine dialog must not scold. The hint appears only after the operator has *tried* to
   * create without a name — not on blur, because opening "Tune later" blurs the field and would
   * fire a nag nobody earned (found in round-03 screenshots).
   */
  const [attempted, setAttempted] = useState(false);

  const slugPreview = useMemo(
    () => slugifyName(slug || name),
    [slug, name],
  );
  const ladder = useMemo(
    () =>
      inheritedRoles({
        defaultLlmProvider: inherited.defaultLlmProvider,
        defaultLlmModel: inherited.defaultLlmModel,
        defaultEmbeddingProvider: inherited.defaultEmbeddingProvider,
        defaultEmbeddingModel: inherited.defaultEmbeddingModel,
        defaultEmbeddingDimension: inherited.defaultEmbeddingDimension,
        defaultVisionProvider: inherited.defaultVisionProvider,
        defaultVisionModel: inherited.defaultVisionModel,
      }),
    [
      inherited.defaultLlmProvider,
      inherited.defaultLlmModel,
      inherited.defaultEmbeddingProvider,
      inherited.defaultEmbeddingModel,
      inherited.defaultEmbeddingDimension,
      inherited.defaultVisionProvider,
      inherited.defaultVisionModel,
    ],
  );

  /**
   * Judged by the same hook the wizards use, against the offer for the tenant this workspace
   * will belong to. Inheritance is judged too: an organization default that is no longer offered
   * has to be visible here, not discovered as a 422 after pressing Create.
   */
  const eligibility = useWizardEligibility({
    scope: target ? "tenant" : "deployment",
    id: target,
    useDefaults: models.source === "inherit",
    inherited: {
      defaultLlmProvider: inherited.defaultLlmProvider,
      defaultLlmModel: inherited.defaultLlmModel,
      defaultEmbeddingProvider: inherited.defaultEmbeddingProvider,
      defaultEmbeddingModel: inherited.defaultEmbeddingModel,
      defaultVisionProvider: inherited.defaultVisionProvider,
      defaultVisionModel: inherited.defaultVisionModel,
    },
    llm: models.llm as { provider?: string; model?: string } | undefined,
    embedding: models.embedding as
      | { provider?: string; model?: string; dimension?: number }
      | undefined,
    vision: models.vision as { provider?: string; model?: string } | undefined,
  });
  const modelBlocked = refusedPickedRoles(eligibility.verdicts, models);
  /**
   * SPEC-183: what the create will actually send. A graph workspace is clamped to `text` so the
   * dialog can never ask the server for a pair it refuses (`modalities_require_normal`).
   */
  const modalities = resolveCreateModalities(retrieval, evidence);
  const offer = useImageEmbeddingOffer({ scope: target ? "tenant" : "deployment", id: target });
  /** The pair an image workspace would embed with: the pinned one, else the inherited one. */
  const embeddingPair = models.embedding?.provider
    ? { provider: models.embedding.provider, model: models.embedding.model }
    : {
        provider: inherited.defaultEmbeddingProvider,
        model: inherited.defaultEmbeddingModel,
      };
  const pairReadsImages = offer.pairReadsImages(embeddingPair.provider, embeddingPair.model);
  /**
   * Text and images is blocked when the offer has no image model, or when the pair that would be
   * stored cannot read images. Both are refusals the server would return on Create, stated here
   * instead — the operator is looking at the control that fixes it.
   */
  const evidenceBlocked = imagesUnavailable({
    retrieval,
    modalities,
    imageModelOffered: offer.imageModelOffered,
  });
  const imagePairBlocked =
    modalities === "text,image" && offer.imageModelOffered && !pairReadsImages;
  const trimmed = name.trim();
  const quota = access.quota.workspaces;
  const quotaBlocked = access.reason.workspace === "workspace_quota_reached";
  const orgName = access.organization?.name ?? "your organization";
  /**
   * Two different questions, deliberately:
   * - `ready` is what the footer shows, so a refusal is stated beside the button it disables.
   * - `canFinish` also needs a name, but a missing name must stay an *attempt* that then explains
   *   itself, not an inert button beside a hint nobody asked for (LAW-181-14).
   */
  const evidenceRefused = evidenceBlocked || imagePairBlocked;
  const ready = !quotaBlocked && modelBlocked.length === 0 && !evidenceRefused;
  const canFinish =
    trimmed.length > 0 && !create.isPending && modelBlocked.length === 0 && !evidenceRefused;

  const quotaReason = `Your plan allows ${quota.limit} workspaces. Archive one you no longer need, or ask your platform admin to raise the limit.`;
  const blockedReason = quotaBlocked
    ? quotaReason
    : modelBlocked.length > 0
      ? blockedModelSentence(modelBlocked)
      : imagePairBlocked
        ? "The embedding model reads text only. Pick an image-capable model under Models, or choose Text evidence."
        : evidenceBlocked
          ? "No image-capable embedding model is offered here. Choose Text evidence, or ask a deployment admin to offer one."
          : null;

  /**
   * SPEC-183 — choosing images also chooses the embedder that can serve them.
   *
   * The inherited model is usually text-only, so without this the operator is left beside a
   * disabled Create button and has to discover the Embed row themselves. The card then prints
   * the picked model, so the change is stated instead of happening invisibly.
   */
  const pickEvidence = (next: EmbeddingModalities) => {
    setEvidence(next);
    const pin = imageEmbeddingPin({
      retrieval,
      modalities: resolveCreateModalities(retrieval, next),
      current: embeddingPair,
      offered: offer.offered,
      pairReadsImages,
    });
    if (!pin) return;
    // `source: choose` is what makes the pin travel in the POST; without it the body omits it
    // and the workspace would inherit the text-only default anyway.
    setModels({ ...models, source: "choose", embedding: { ...pin, dimension: 1536 } });
  };

  const reset = () => {
    setName("");
    setSlug("");
    setRetrieval("graph");
    setEvidence("text");
    setModels(emptyModelDraft());
    setOpenRole(null);
    setAdvanced({});
    setAdvancedOpen(false);
    setError(null);
    setAttempted(false);
  };

  const close = (next: boolean) => {
    if (!next) reset();
    onOpenChange(next);
  };

  const finish = () => {
    if (!canFinish) {
      setAttempted(true);
      return;
    }
    if (!target) return;
    setError(null);
    create.mutate(
      {
        tenantId: target,
        name: trimmed,
        slug: slugPreview || undefined,
        // SPEC-180: the profile is immutable after create, so it is a top-level decision.
        rag_profile: retrieval,
        // SPEC-183: so is the evidence. Without this the workspace is always `text` and an
        // image can never be uploaded, because the column does not change.
        embedding_modalities: modalities,
        // Only send what the operator actually chose; an absent field stays inherited (LAW-123-8).
        ...toIngestPayload(advanced),
        ...toModelPayload(models),
      },
      {
        onSuccess: ({ tenant, workspace }) => {
          applyCreatedScope(tenant, workspace);
          settleAfterCreate(qc, tenant.id, workspace?.id ?? null);
          toast.success(`${trimmed} is ready`, {
            description: "Upload documents or invite people from the confirmation card.",
          });
          onCreated?.(workspace?.id ?? "", workspace?.slug ?? slugPreview);
          close(false);
        },
        onError: (e) => setError(classifyCreateFailure(e)),
      },
    );
  };

  const advancedCount = Object.keys(advanced).filter((k) => advanced[k] !== "").length;

  return (
    <ScopeDialog
      open={open}
      onOpenChange={close}
      title="Create a workspace"
      lead="Workspaces keep documents and knowledge apart."
      quota={{ label: "Workspaces", used: quota.used, limit: quota.limit }}
      canFinish={ready}
      blockedReason={blockedReason}
      busy={create.isPending}
      finishLabel="Create workspace"
      isDirty={
        trimmed.length > 0 ||
        advancedCount > 0 ||
        hasAnyPick(models) ||
        models.source === "choose" ||
        modalities !== "text"
      }
      onFinish={finish}
      onEscape={() => {
        // Collapse the open model row first; a second Escape then asks to discard.
        if (!openRole) return false;
        setOpenRole(null);
        return true;
      }}
      testId="create-workspace-dialog"
    >
      <div className="space-y-5">
        {/* 1 — Name */}
        <div className="space-y-1.5">
          <Label htmlFor="ws-name" className="text-[13px] font-medium">
            Name
          </Label>
          <Input
            id="ws-name"
            data-autofocus
            data-wizard-enter
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Legal"
            autoComplete="off"
            maxLength={120}
            aria-invalid={error?.kind === "slug"}
            data-testid="create-workspace-name"
          />
          {slugPreview ? (
            <p className="font-mono text-[11.5px] text-muted-foreground">/w/{slugPreview}</p>
          ) : null}
        </div>

        {/* 2 — Retrieval type. Permanent, so it is a decision, not a drawer row. */}
        <div className="space-y-1.5">
          <div className="flex items-baseline justify-between gap-2">
            <Label className="text-[13px] font-medium">Retrieval type</Label>
            <span className="text-[11.5px] font-medium text-amber-600 dark:text-amber-400">
              This is permanent
            </span>
          </div>
          <div role="radiogroup" aria-label="Retrieval type" className="space-y-2">
            <ChoiceCard
              checked={retrieval === "graph"}
              onSelect={() => setRetrieval("graph")}
              title="Graph"
              body="Entities and relationships. Slower ingest, better at questions that connect facts."
              testId="create-workspace-retrieval-graph"
            />
            <ChoiceCard
              checked={retrieval === "normal"}
              onSelect={() => setRetrieval("normal")}
              title="Normal"
              body="Text and keywords only. No knowledge graph. Faster ingest, enough for lookup."
              testId="create-workspace-retrieval-normal"
            />
          </div>
        </div>

        {/*
          SPEC-183 — Evidence. Another create-time decision, so it is asked here beside the
          retrieval type: a workspace made from the UI was always `text`, and `text` is
          immutable, which is why images could not be uploaded and no control could fix it.
        */}
        <CreateWorkspaceEvidence
          retrieval={retrieval}
          value={evidence}
          onChange={pickEvidence}
          imageModelOffered={offer.imageModelOffered}
          imageModelLabel={offer.imageModelLabel}
          pairCannotReadImages={modalities === "text,image" && !pairReadsImages}
          isLoading={!offer.loaded}
        />

        {/* 3 — Models. Inheritance stays the default and stays printed; a change is now possible. */}
        <WorkspaceModelsPanel
          draft={models}
          onChange={setModels}
          tenantId={target ?? null}
          orgName={orgName}
          isLoading={inherited.isLoading}
          ladder={ladder}
          verdicts={eligibility.verdicts}
          openRole={openRole}
          onOpenRoleChange={setOpenRole}
          canManageProviders={access.organization?.capabilities.has("organization.settings.manage") ?? false}
          imageEmbeddingOnly={modalities === "text,image"}
        />

        {/* Tune later — everything the 8-step wizard asked for, collapsed. */}
        <TuneLater
          open={advancedOpen}
          onOpenChange={setAdvancedOpen}
          count={advancedCount}
          values={advanced}
          onChange={(key, value) => setAdvanced((prev) => ({ ...prev, [key]: value }))}
          inherited={
            inherited.isLoading
              ? null
              : { extraction_language: inherited.extractionLanguage }
          }
        />

        {error ? (
          <div
            role="alert"
            data-testid="create-workspace-error"
            className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[12.5px]"
          >
            <AlertCircle aria-hidden className="mt-0.5 h-3.5 w-3.5 shrink-0 text-destructive" />
            <span>
              {error.kind === "quota"
                ? "Your plan's workspace limit was reached. Archive a workspace and try again."
                : error.kind === "slug"
                  ? "A workspace with that address already exists here. Change the name."
                  : error.kind === "models"
                    ? "That model cannot be used for this workspace. Check the model policy."
                    : "Could not create the workspace. Your input is kept, so you can retry."}
            </span>
          </div>
        ) : null}

        {/* The quota strip repeats here at the limit so the fix sits beside the form. */}
        {quotaBlocked ? (
          <div className="rounded-md border border-amber-500/30 bg-amber-500/10 p-3">
            <MeterRow label="Workspaces" used={quota.used} limit={quota.limit} />
            <Button variant="link" size="sm" className="mt-1 h-auto p-0 text-[12.5px]" asChild>
              <Link href="/settings/organization/workspaces?tab=archived">Archive a workspace →</Link>
            </Button>
          </div>
        ) : null}
      </div>
    </ScopeDialog>
  );
}

/**
 * Coerce the drawer's string values into the typed create payload.
 *
 * The drawer is a flat `Record<string, string>` because it is a form; the request is typed. This
 * is the one conversion point, so the dialog stays type-safe instead of casting the whole body
 * (`as never`) and losing every other field's checking with it.
 */
function toIngestPayload(
  values: Record<string, string>,
): Partial<CreateWorkspaceRequest> {
  const num = (v: string | undefined) => {
    const n = Number.parseInt(v ?? "", 10);
    return Number.isFinite(n) ? n : undefined;
  };
  const str = (v: string | undefined) => (v && v.length > 0 ? v : undefined);
  return {
    // The drawer holds plain strings; narrow to the union the request actually accepts.
    pdf_parser_backend: parsePdfBackend(values.pdf_parser_backend),
    extraction_language: str(values.extraction_language),
    chunk_token_size: num(values.chunk_token_size),
    chunk_overlap_token_size: num(values.chunk_overlap_token_size),
    extract_max_entities: num(values.extract_max_entities),
    extract_max_records: num(values.extract_max_records),
  };
}

function parsePdfBackend(
  v: string | undefined,
): CreateWorkspaceRequest["pdf_parser_backend"] {
  switch (v) {
    case "vision":
    case "edgeparse":
    case "edgeparse-ocr":
    case "auto":
    case "none":
      return v;
    default:
      return undefined;
  }
}
