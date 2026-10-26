/** Plain-language labels for Access. Pure functions, no React. */

import type { ClearanceTag, ClassificationWord, SecurityRoleView } from "./types";
import { draftFromRules, type Ceiling, type ProfileDraft } from "./rules";

const SEPARATOR = " · ";

const CEILING_LABEL: Record<Ceiling, string> = {
  public: "Public",
  internal: "Internal",
  confidential: "Confidential",
  restricted: "Restricted",
};

export function ceilingLabel(ceiling: Ceiling): string {
  return CEILING_LABEL[ceiling];
}

/** Name lookup for vocabulary ids, built once per render. */
export function tagNames(tags: ClearanceTag[]): Map<string, string> {
  return new Map(tags.map((tag) => [tag.id, tag.value]));
}

/** Departments and projects only. Classification words are not vocabulary. */
export function compartmentTags(tags: ClearanceTag[]): ClearanceTag[] {
  return tags.filter((tag) => tag.kind === "compartment");
}

export function tagOptions(tags: ClearanceTag[]): { id: string; label: string }[] {
  return compartmentTags(tags).map((tag) => ({ id: tag.id, label: tag.value }));
}

function namesFor(ids: string[], names: Map<string, string>): string[] {
  return ids.map((id) => names.get(id)).filter((name): name is string => Boolean(name));
}

/** "Confidential · HR, R&D · except Atlas". Used on profile rows and in the editor. */
export function profileSummary(draft: ProfileDraft, names: Map<string, string>): string {
  const parts: string[] = [ceilingLabel(draft.ceiling)];
  const departments = namesFor(draft.departmentIds, names);
  if (departments.length > 0) parts.push(departments.join(", "));
  const except = namesFor(draft.exceptIds, names);
  if (except.length > 0) parts.push(`except ${except.join(", ")}`);
  return parts.join(SEPARATOR);
}

/** Words for a document's classification. Stewards see "Not reviewed"; others see "Only you". */
export function classificationWord(word: ClassificationWord | string, steward: boolean): string {
  if (word === "unverified") return steward ? "Needs review" : "You and authorized security stewards, until reviewed";
  return word.charAt(0).toUpperCase() + word.slice(1);
}

/** "Internal · HR, Legal" for a document's label. */
export function documentLabelSummary(
  classification: string,
  compartmentNames: string[],
  steward: boolean,
): string {
  const parts = [classificationWord(classification, steward)];
  if (compartmentNames.length > 0) parts.push(compartmentNames.join(", "));
  return parts.join(SEPARATOR);
}

/** The line under a profile name, e.g. "Confidential · HR, R&D · except Atlas". */
export function profileLine(role: SecurityRoleView, tags: ClearanceTag[]): string {
  if (role.rules.length === 0) return "No access yet";
  return profileSummary(draftFromRules(role.rules), tagNames(tags));
}

/** "3 documents", "1 document". A null count means more than 20. */
export function documentCountLabel(count: number | null): string {
  if (count == null) return "More than 20 documents";
  return count === 1 ? "1 document" : `${count} documents`;
}
