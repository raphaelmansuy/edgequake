"use client";

/**
 * SPEC-181 W-05 — the confirmation after a create.
 *
 * A dialog that closes into a toast is a dead end: the person has a new scope and no idea what
 * to do with it. This card names what was created (owner, workspace, plan) and offers the two
 * actions that make a scope useful.
 */

import { Button } from "@/components/ui/button";
import { Check, FileUp, UserPlus } from "lucide-react";
import Link from "next/link";

export function CreateOrganizationConfirmation({
  tenantName,
  ownerLabel,
  workspaceSlug,
  plan,
  limit,
  seats,
  onInvite,
  onUpload,
  testId = "create-organization-confirmation",
}: {
  tenantName: string;
  ownerLabel: string;
  workspaceSlug: string | null;
  plan: string;
  limit: number;
  seats: number;
  onInvite: () => void;
  onUpload: () => void;
  testId?: string;
}) {
  return (
    <div className="space-y-5" data-testid={testId}>
      <div className="flex flex-col items-center gap-2 pt-1 text-center">
        <span className="flex h-10 w-10 items-center justify-center rounded-full bg-success/10">
          <Check aria-hidden className="h-5 w-5 text-success" />
        </span>
        <p className="text-[15px] font-semibold text-foreground">{tenantName} is ready</p>
      </div>

      <dl className="divide-y rounded-lg border">
        <Row label="Owner" value={ownerLabel} />
        <Row
          label="Default workspace"
          value={
            workspaceSlug ? (
              <Link href={`/w/${workspaceSlug}`} className="underline underline-offset-2">
                /w/{workspaceSlug}
              </Link>
            ) : (
              "—"
            )
          }
        />
        <Row
          label="Plan"
          value={`${plan} · ${limit} workspaces, ${seats === 0 ? "unlimited people" : `${seats} people`}`}
        />
      </dl>

      <div className="flex flex-col gap-2 sm:flex-row">
        <Button type="button" onClick={onInvite} className="sm:flex-1" data-testid={`${testId}-invite`}>
          <UserPlus aria-hidden className="mr-2 h-4 w-4" />
          Invite people
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={onUpload}
          className="sm:flex-1"
          data-testid={`${testId}-upload`}
        >
          <FileUp aria-hidden className="mr-2 h-4 w-4" />
          Upload documents
        </Button>
      </div>

      <p className="text-center text-[11.5px] text-muted-foreground">
        Switch to it later from the organization menu in the header.
      </p>
    </div>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-3 px-3 py-2">
      <dt className="text-[12.5px] text-muted-foreground">{label}</dt>
      <dd className="min-w-0 truncate text-right text-[12.5px] font-medium text-foreground">{value}</dd>
    </div>
  );
}
