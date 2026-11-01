"use client";

/**
 * SPEC-181 — the chrome both create dialogs share.
 *
 * Reuses the accessibility contract already proven in `onboarding/wizard-shell.tsx` (focus the
 * first field, `aria-live` for the blocked reason, Escape asks to discard when dirty, a portaled
 * model menu does not dismiss the dialog) **without** the stepper, the step counter or the back
 * button. That is the DRY split: `WizardShell` stays for the first-run wizard, which genuinely
 * has ordered steps; this is the three-decision case.
 */

import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { MeterRow } from "@/components/admin/primitives";
import { isPortaledChoiceMenu } from "@/lib/onboarding/portaled-menu";
import { cn } from "@/lib/utils";
import { Info, Loader2 } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";

export interface ScopeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  lead?: string;
  /** Plan budget shown under the title; the reason the dialog is short. */
  quota?: { label: string; used: number; limit: number };
  children: ReactNode;
  /** Why the primary action is inert; rendered in the footer, where the eye already is. */
  blockedReason?: string | null;
  /**
   * Whether the primary action may be pressed.
   *
   * Kept separate from `blockedReason` so a dialog can allow an *attempt* that then explains
   * itself, instead of shipping a permanently inert button beside a hint the operator never
   * asked for. Pass `false` only for a refusal the operator cannot fix in the dialog.
   */
  canFinish: boolean;
  busy?: boolean;
  finishLabel: string;
  cancelLabel?: string;
  isDirty?: boolean;
  /** A confirmation card has nothing to cancel; the only exit is the primary action. */
  hideCancel?: boolean;
  onFinish: () => void;
  testId: string;
}

export function ScopeDialog({
  open,
  onOpenChange,
  title,
  lead,
  quota,
  children,
  blockedReason,
  canFinish,
  busy = false,
  finishLabel,
  cancelLabel = "Cancel",
  isDirty = false,
  hideCancel = false,
  onFinish,
  testId,
}: ScopeDialogProps) {
  const bodyRef = useRef<HTMLDivElement>(null);
  const [confirmDiscard, setConfirmDiscard] = useState(false);

  /** The first field marked `data-autofocus` takes focus; otherwise the dialog title does. */
  const focusFirst = () => {
    const field = bodyRef.current?.querySelector<HTMLElement>("[data-autofocus]");
    field?.focus();
    if (field instanceof HTMLInputElement) field.select();
  };

  useEffect(() => {
    if (open) requestAnimationFrame(focusFirst);
  }, [open]);

  const requestClose = () => {
    if (isDirty && !busy) {
      setConfirmDiscard(true);
      return;
    }
    onOpenChange(false);
  };

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (!next) requestClose();
          else onOpenChange(true);
        }}
      >
        <DialogContent
          data-testid={testId}
          className={cn(
            "!flex !flex-col gap-0 overflow-hidden p-0",
            "h-[min(90vh,42rem)] w-[calc(100%-1.5rem)] sm:!max-w-lg",
          )}
          onOpenAutoFocus={(e) => {
            e.preventDefault();
            focusFirst();
          }}
          onPointerDownOutside={(e) => {
            // A portaled model menu is outside the dialog but is the *selection*, not a dismissal.
            if (isPortaledChoiceMenu(e.detail.originalEvent.target)) {
              e.preventDefault();
              return;
            }
            if (isDirty) {
              e.preventDefault();
              setConfirmDiscard(true);
            }
          }}
          onEscapeKeyDown={(e) => {
            if (isDirty) {
              e.preventDefault();
              setConfirmDiscard(true);
            }
          }}
        >
          <div className="shrink-0 space-y-3 border-b px-5 pt-5 pb-4">
            <DialogHeader className="gap-0.5 pr-8 text-left">
              <DialogTitle className="text-base font-semibold tracking-[-0.01em]">{title}</DialogTitle>
              {lead ? (
                <DialogDescription className="text-[13px] leading-relaxed text-muted-foreground">
                  {lead}
                </DialogDescription>
              ) : null}
            </DialogHeader>
            {quota ? (
              <MeterRow
                label={quota.label}
                used={quota.used}
                limit={quota.limit}
                testId={`${testId}-quota`}
              />
            ) : null}
          </div>

          <div
            ref={bodyRef}
            className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-5 py-4"
            onKeyDown={(e) => {
              // Opt-in: only a field marked `data-wizard-enter` submits on Enter.
              const target = e.target as HTMLElement;
              if (e.key !== "Enter" || e.shiftKey || !target.hasAttribute("data-wizard-enter")) {
                return;
              }
              e.preventDefault();
              if (canFinish && !busy) onFinish();
            }}
          >
            {children}
          </div>

          <div className="relative z-10 flex shrink-0 flex-col gap-2 border-t bg-background px-5 py-3 sm:flex-row sm:items-center sm:gap-3">
            <div
              role="status"
              aria-live="polite"
              className="flex-1 text-[12.5px] text-muted-foreground empty:hidden"
              data-testid={`${testId}-blocked-reason`}
            >
              {!canFinish && blockedReason ? (
                <span className="flex items-start gap-1.5">
                  <Info aria-hidden className="mt-px h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
                  <span>{blockedReason}</span>
                </span>
              ) : null}
            </div>
            <div className="flex gap-2 max-sm:w-full">
              {!hideCancel ? (
                <Button
                  type="button"
                  variant="ghost"
                  onClick={requestClose}
                  disabled={busy}
                  data-testid={`${testId}-cancel`}
                  className="max-sm:flex-1"
                >
                  {cancelLabel}
                </Button>
              ) : null}
              <Button
                type="button"
                onClick={onFinish}
                disabled={!canFinish || busy}
                data-testid={`${testId}-finish`}
                className="max-sm:flex-1"
              >
                {busy ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                {finishLabel}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmDiscard} onOpenChange={setConfirmDiscard}>
        <AlertDialogContent data-testid={`${testId}-discard`}>
          <AlertDialogHeader>
            <AlertDialogTitle>Discard this draft?</AlertDialogTitle>
            <AlertDialogDescription>
              Your progress is kept until you close. Closing now discards it.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel data-testid={`${testId}-discard-keep`}>Keep editing</AlertDialogCancel>
            <AlertDialogAction
              data-testid={`${testId}-discard-confirm`}
              onClick={() => {
                setConfirmDiscard(false);
                onOpenChange(false);
              }}
            >
              Discard
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

/**
 * A two-line radio card. Used for the retrieval type, where each option carries a consequence
 * sentence that a segmented control cannot express.
 */
export function ChoiceCard({
  checked,
  onSelect,
  title,
  body,
  disabled,
  hint,
  testId,
}: {
  checked: boolean;
  onSelect: () => void;
  title: string;
  body: string;
  disabled?: boolean;
  hint?: string;
  testId?: string;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      disabled={disabled}
      onClick={onSelect}
      data-testid={testId}
      className={cn(
        "flex w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        checked ? "border-primary bg-primary/5 ring-1 ring-primary" : "border-border hover:bg-muted/40",
        disabled && "cursor-not-allowed opacity-60",
      )}
    >
      <span
        aria-hidden
        className={cn(
          "mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full border",
          checked ? "border-primary" : "border-input",
        )}
      >
        {checked ? <span className="h-2 w-2 rounded-full bg-primary" /> : null}
      </span>
      <span className="min-w-0">
        <span className="block text-[13.5px] font-medium text-foreground">{title}</span>
        <span className="mt-0.5 block text-[12.5px] leading-relaxed text-muted-foreground">{body}</span>
        {hint ? <span className="mt-1 block text-[11.5px] text-muted-foreground">{hint}</span> : null}
      </span>
    </button>
  );
}
