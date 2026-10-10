'use client';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { Loader2 } from 'lucide-react';
import { useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface WizardShellProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  stepIndex: number;
  stepCount: number;
  stepTitle: string;
  stepDescription?: string;
  children: React.ReactNode;
  canGoNext: boolean;
  isLastStep: boolean;
  isSubmitting?: boolean;
  onBack: () => void;
  onNext: () => void;
  onCancel?: () => void;
  nextLabel?: string;
  finishLabel?: string;
  className?: string;
  testId?: string;
  /** When false, ignore outside/Escape dismiss (first-run). Default true. */
  dismissible?: boolean;
  /** Hide Cancel button (first-run). Default false. */
  hideCancel?: boolean;
  /** Forwarded to DialogContent. Default true. */
  showCloseButton?: boolean;
  /** When true, Cancel/close asks for confirmation. */
  isDirty?: boolean;
}

/**
 * SPEC-101 — Shared multi-step dialog chrome (LAW-101-1, LAW-101-8/10).
 */
export function WizardShell({
  open,
  onOpenChange,
  title,
  description,
  stepIndex,
  stepCount,
  stepTitle,
  stepDescription,
  children,
  canGoNext,
  isLastStep,
  isSubmitting = false,
  onBack,
  onNext,
  onCancel,
  nextLabel,
  finishLabel,
  className,
  testId = 'wizard-shell',
  dismissible = true,
  hideCancel = false,
  showCloseButton = true,
  isDirty = false,
}: WizardShellProps) {
  const { t } = useTranslation();
  const stepTitleId = useId();
  const stepHeadingRef = useRef<HTMLHeadingElement>(null);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const pct = stepCount > 0 ? Math.round(((stepIndex + 1) / stepCount) * 100) : 0;
  const stepOfLabel = t('onboarding.stepOf', 'Step {{current}} of {{total}}', {
    current: stepIndex + 1,
    total: stepCount,
  });
  const valueText = `${stepOfLabel}: ${stepTitle}`;

  useEffect(() => {
    if (open) {
      stepHeadingRef.current?.focus();
    }
  }, [open, stepIndex]);

  const requestClose = () => {
    if (!dismissible) return;
    if (isDirty && !isSubmitting) {
      setConfirmDiscard(true);
      return;
    }
    onCancel?.();
    onOpenChange(false);
  };

  const confirmClose = () => {
    setConfirmDiscard(false);
    onCancel?.();
    onOpenChange(false);
  };

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (!next) {
            requestClose();
            return;
          }
          onOpenChange(next);
        }}
      >
        <DialogContent
          className={cn(
            // SPEC-114 layout: DialogContent defaults to `grid` — force a column flex
            // shell so sticky chrome + scroll body + footer share max height correctly.
            '!flex !flex-col gap-0 overflow-hidden p-0',
            'h-[min(70vh,40rem)] max-h-[min(92vh,56rem)] w-[calc(100%-1.5rem)] sm:!max-w-5xl lg:!max-w-7xl',
            className,
          )}
          data-testid={testId}
          showCloseButton={showCloseButton && dismissible}
          onPointerDownOutside={(e) => {
            if (!dismissible || isDirty) e.preventDefault();
            if (isDirty && dismissible) {
              e.preventDefault();
              setConfirmDiscard(true);
            }
          }}
          onEscapeKeyDown={(e) => {
            if (!dismissible) {
              e.preventDefault();
              return;
            }
            if (isDirty) {
              e.preventDefault();
              setConfirmDiscard(true);
            }
          }}
        >
          {/* Sticky chrome — stays visible while step body scrolls */}
          <div className="shrink-0 space-y-3 border-b px-6 pt-6 pb-3">
            <DialogHeader>
              {/* Do not pass a custom `id` — Radix TitleWarning looks up context.titleId. */}
              <DialogTitle>{title}</DialogTitle>
              {description ? (
                <DialogDescription>{description}</DialogDescription>
              ) : (
                <DialogDescription className="sr-only">
                  {t('onboarding.wizardDialogHint', 'Multi-step creation wizard')}
                </DialogDescription>
              )}
            </DialogHeader>

            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span data-testid="wizard-step-counter">{stepOfLabel}</span>
                <span className="font-medium text-foreground" data-testid="wizard-step-label">
                  {stepTitle}
                </span>
              </div>
              <div
                className="sr-only"
                aria-live="polite"
                aria-atomic="true"
                data-testid="wizard-step-live"
              >
                {valueText}
              </div>
              <Progress
                value={pct}
                aria-valuenow={stepIndex + 1}
                aria-valuemin={1}
                aria-valuemax={stepCount}
                aria-valuetext={valueText}
                data-testid="wizard-progress"
              />
              {/* Focus target for step changes — visually use counter label, not a duplicate h3. */}
              <h3
                ref={stepHeadingRef}
                id={stepTitleId}
                tabIndex={-1}
                className="sr-only"
                data-testid="wizard-step-title"
              >
                {stepTitle}
              </h3>
              {stepDescription ? (
                <p className="text-sm text-muted-foreground" data-testid="wizard-step-description">
                  {stepDescription}
                </p>
              ) : null}
            </div>
          </div>

          <div
            className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-6 py-4"
            data-testid="wizard-step-scroll"
          >
            <div
              data-testid="wizard-step-body"
              role="group"
              aria-labelledby={stepTitleId}
            >
              {children}
            </div>
          </div>

          <DialogFooter className="relative z-10 gap-2 sm:gap-2 border-t bg-background px-6 py-3 shrink-0 max-sm:flex-col shadow-[0_-4px_12px_-8px_rgba(0,0,0,0.12)]">
            {!hideCancel ? (
              <Button
                type="button"
                variant="ghost"
                onClick={requestClose}
                disabled={isSubmitting}
                data-testid="wizard-cancel"
                className="max-sm:w-full"
              >
                {t('common.cancel', 'Cancel')}
              </Button>
            ) : null}
            <div className="flex gap-2 max-sm:w-full sm:contents">
              <Button
                type="button"
                variant="outline"
                onClick={onBack}
                disabled={stepIndex === 0 || isSubmitting}
                data-testid="wizard-back"
                className="max-sm:flex-1"
              >
                {t('common.back', 'Back')}
              </Button>
              <Button
                type="button"
                onClick={onNext}
                disabled={!canGoNext || isSubmitting}
                data-testid={isLastStep ? 'wizard-finish' : 'wizard-next'}
                className="max-sm:flex-1"
              >
                {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                {isLastStep
                  ? finishLabel ?? t('common.create', 'Create')
                  : nextLabel ?? t('common.next', 'Next')}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmDiscard} onOpenChange={setConfirmDiscard}>
        <AlertDialogContent data-testid="wizard-discard-confirm">
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t('onboarding.discardTitle', 'Discard changes?')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                'onboarding.discardDescription',
                'You have unsaved wizard progress. Closing will discard it.',
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel data-testid="wizard-discard-keep">
              {t('onboarding.keepEditing', 'Keep editing')}
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmClose}
              data-testid="wizard-discard-confirm-action"
            >
              {t('onboarding.discardConfirm', 'Discard')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
