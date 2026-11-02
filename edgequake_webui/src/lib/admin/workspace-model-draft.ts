/**
 * SPEC-181 W-03 — the Models decision in the Create workspace dialog.
 *
 * Pure helpers only: what the panel prints, and what the POST body carries. They live outside
 * the component so the promise "choosing inherit sends nothing" (LAW-123-8) is a unit test
 * instead of a screenshot, and so the summary line has one implementation for three roles.
 */

import type { CreateWorkspaceRequest } from "@/types";

export type ModelRole = "llm" | "embedding" | "vision";

/** `inherit` follows the organization ladder; `choose` pins at least one role here. */
export type ModelSource = "inherit" | "choose";

export const MODEL_ROLES: readonly ModelRole[] = ["llm", "embedding", "vision"];

/** Short labels. Kept identical to what the dialog printed before, so the change is additive. */
export const ROLE_LABEL: Record<ModelRole, string> = {
  llm: "Extract",
  embedding: "Embed",
  vision: "Vision",
};

/** One sentence of consequence per role. A pick here is invisible until it is explained. */
export const ROLE_BLURB: Record<ModelRole, string> = {
  llm: "Writes entities and relations while documents are ingested.",
  embedding: "Turns chunks into vectors. Its size fixes the vector column.",
  vision: "Reads images while a PDF is converted. Follows Extract until you pick one.",
};

/**
 * Anything a picker hands back. Every field is optional so a half-made selection is
 * representable, and so both `LLMSelection` and `EmbeddingSelection` fit without a cast.
 */
export interface ModelPickLike {
  provider?: string;
  model?: string;
  dimension?: number;
  displayName?: string;
  supportsVision?: boolean;
  embedsImages?: boolean;
}

export interface WorkspaceModelDraft {
  source: ModelSource;
  llm?: ModelPickLike;
  embedding?: ModelPickLike;
  vision?: ModelPickLike;
}

/** A pristine dialog inherits everything — the shape the 181 promise is built on. */
export function emptyModelDraft(): WorkspaceModelDraft {
  return { source: "inherit" };
}

/** A pair is a choice only when both halves are present; a provider alone is not one. */
export function isCompletePick(pick?: ModelPickLike): boolean {
  return Boolean(pick?.provider?.trim() && pick?.model?.trim());
}

/** How many roles the operator pinned. Printed beside the section label. */
export function pickedCount(draft: WorkspaceModelDraft): number {
  return MODEL_ROLES.filter((role) => isCompletePick(draft[role])).length;
}

export function hasAnyPick(draft: WorkspaceModelDraft): boolean {
  return pickedCount(draft) > 0;
}

/**
 * The create body's model fields.
 *
 * Absent means inherited: the workspace keeps resolving against the organization default, so a
 * later change of that default still reaches it. Materialising the resolved pair here would
 * freeze the workspace to today's value (LAW-123-8).
 */
export function toModelPayload(
  draft: WorkspaceModelDraft,
): Partial<CreateWorkspaceRequest> {
  if (draft.source !== "choose") return {};
  const body: Partial<CreateWorkspaceRequest> = {};
  const { llm, embedding, vision } = draft;
  if (isCompletePick(llm)) {
    body.llm_provider = llm?.provider;
    body.llm_model = llm?.model;
  }
  if (isCompletePick(embedding)) {
    // The server copies the dimension from the model card. The client never guesses it.
    body.embedding_provider = embedding?.provider;
    body.embedding_model = embedding?.model;
  }
  if (isCompletePick(vision)) {
    body.vision_llm_provider = vision?.provider;
    body.vision_llm_model = vision?.model;
  }
  return body;
}

/** The inherited pair for one role, as the ladder resolved it. */
export interface RoleInherited {
  provider?: string;
  model?: string;
  dimension?: number;
}

export type InheritedRoles = Record<ModelRole, RoleInherited>;

/** `inherited` follows the organization, `workspace` is pinned here, `follows` has no pair of its own. */
export type RoleChip = "inherited" | "workspace" | "follows";

export interface RoleSummary {
  /** What to read first: a display name, a model id, or "Follows Extract". */
  title: string;
  /**
   * The one-line form SPEC-181 §09 specifies for the inherit card:
   * `gpt-5-nano · openai`, `text-embedding-3-small · openai · 1536`, `not set`.
   */
  label: string;
  chip: RoleChip;
  /** One extra fact: `1536 dims`, `Vision capable`. */
  detail?: string;
  /** The mono `provider/model`, or null when there is nothing to print. */
  id: string | null;
  /** Neither a pick nor an inherited default: the row has nothing to say yet. */
  unset: boolean;
}

const samePair = (a: RoleInherited, b: RoleInherited): boolean =>
  Boolean(a.provider && a.model) &&
  a.provider?.trim().toLowerCase() === b.provider?.trim().toLowerCase() &&
  a.model?.trim().toLowerCase() === b.model?.trim().toLowerCase();

/**
 * The collapsed line for one role.
 *
 * Vision has one honest special case: when it resolves to the Extract pair, printing the same
 * id twice reads as a duplicate row. Saying "Follows Extract" states the relationship instead.
 */
export function roleSummary(
  role: ModelRole,
  inherited: InheritedRoles,
  pick: ModelPickLike | undefined,
): RoleSummary {
  const own = inherited[role];
  const followed = role === "vision" && samePair(own, inherited.llm);

  if (isCompletePick(pick)) {
    const model = pick?.model as string;
    return {
      title: pick?.displayName || model,
      label: `${model} · ${pick?.provider}`,
      chip: "workspace",
      detail: pick ? detailFor(role, pick) : undefined,
      id: `${pick?.provider}/${model}`,
      unset: false,
    };
  }

  if (followed) {
    return {
      title: "Follows Extract",
      label: "follows Extract",
      chip: "follows",
      detail: undefined,
      id: null,
      unset: false,
    };
  }

  if (!own.provider || !own.model) {
    return {
      title: "Not set",
      label: "not set",
      chip: "inherited",
      id: null,
      unset: true,
    };
  }

  const dimension = role === "embedding" && own.dimension && own.dimension > 0 ? own.dimension : 0;
  return {
    title: own.model,
    // `model · provider` and, for an embedder, `· 1536` — exactly the §09 line.
    label: `${own.model} · ${own.provider}${dimension ? ` · ${dimension}` : ""}`,
    chip: "inherited",
    detail: dimension ? `${dimension} dims` : undefined,
    id: `${own.provider}/${own.model}`,
    unset: false,
  };
}

function detailFor(role: ModelRole, pick: ModelPickLike): string | undefined {
  if (role === "embedding") {
    return pick.dimension && pick.dimension > 0 ? `${pick.dimension} dims` : undefined;
  }
  if (role === "vision") return "Vision capable";
  return pick.supportsVision ? "Vision capable" : undefined;
}

/** Build the ladder's pairs once, so all three summaries read the same object. */
export function inheritedRoles(input: {
  defaultLlmProvider?: string;
  defaultLlmModel?: string;
  defaultEmbeddingProvider?: string;
  defaultEmbeddingModel?: string;
  defaultEmbeddingDimension?: number;
  defaultVisionProvider?: string;
  defaultVisionModel?: string;
}): InheritedRoles {
  return {
    llm: { provider: input.defaultLlmProvider, model: input.defaultLlmModel },
    embedding: {
      provider: input.defaultEmbeddingProvider,
      model: input.defaultEmbeddingModel,
      dimension: input.defaultEmbeddingDimension,
    },
    vision: { provider: input.defaultVisionProvider, model: input.defaultVisionModel },
  };
}

/**
 * The verdict words this panel can print. A subset of the wizard's `RoleStatus`: `ok`,
 * `unset` and `loading` say nothing, because a row that is fine should stay quiet.
 */
export type RoleVerdict = "ok" | "not_eligible" | "unset" | "dimension_missing" | "loading";

/** One sentence, directly under the row it is about. */
export function verdictSentence(status: RoleVerdict | undefined): string | null {
  if (status === "not_eligible") return "Not offered to this organization.";
  if (status === "dimension_missing") return "No registered vector size for this model.";
  return null;
}

/** The roles whose row is showing a refusal. Used for the row border and the footer reason. */
export function refusedRoles(
  verdicts: Partial<Record<ModelRole, RoleVerdict>> | undefined,
): ModelRole[] {
  if (!verdicts) return [];
  return MODEL_ROLES.filter(
    (role) => verdicts[role] === "not_eligible" || verdicts[role] === "dimension_missing",
  );
}

/**
 * Refusals that can actually stop a create.
 *
 * `POST /workspaces` judges only the pairs the body names (`ensure_workspace_choice`), and an
 * absent role inherits without being judged. So a refusal on a role the operator did not pin
 * must not disable Create — it would refuse a workspace the server would happily accept.
 */
export function refusedPickedRoles(
  verdicts: Partial<Record<ModelRole, RoleVerdict>> | undefined,
  draft: WorkspaceModelDraft,
): ModelRole[] {
  if (draft.source !== "choose") return [];
  return refusedRoles(verdicts).filter((role) => isCompletePick(draft[role]));
}

/**
 * Why Create is inert. It names the role, because "a model cannot be used" makes the reader
 * search three rows for the one that is red.
 */
export function blockedModelSentence(roles: readonly ModelRole[]): string | null {
  if (roles.length === 0) return null;
  const names = roles.map((role) => ROLE_LABEL[role]).join(", ");
  return `Choose another model for ${names} to continue.`;
}
