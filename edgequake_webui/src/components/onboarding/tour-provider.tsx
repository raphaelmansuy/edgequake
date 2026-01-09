/**
 * @fileoverview Guided tour provider for onboarding new users
 *
 * @implements FEAT1050 - Interactive onboarding tour system
 * @implements FEAT1051 - Step-by-step feature introduction
 *
 * @see UC1301 - New user completes onboarding tour
 * @see UC1302 - User navigates tour steps
 *
 * @enforces BR1050 - Tour state persistence in localStorage
 * @enforces BR1051 - Element highlighting with overlay
 */
'use client';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { ChevronLeft, ChevronRight, HelpCircle, X } from 'lucide-react';
import * as React from 'react';
import { createContext, useCallback, useContext, useEffect, useState } from 'react';
import { createPortal } from 'react-dom';

// ============================================================================
// Types
// ============================================================================

export interface TourStep {
  /** Unique identifier for the step */
  id: string;
  /** CSS selector for the target element to highlight */
  target: string;
  /** Title of the step */
  title: string;
  /** Description/content of the step */
  content: React.ReactNode;
  /** Placement of the popover relative to target */
  placement?: 'top' | 'bottom' | 'left' | 'right';
  /** Optional action to perform when step is shown */
  onShow?: () => void;
  /** Optional action to perform when step is hidden */
  onHide?: () => void;
}

interface TourContextValue {
  /** Whether the tour is currently active */
  isActive: boolean;
  /** Current step index */
  currentStep: number;
  /** Total number of steps */
  totalSteps: number;
  /** Start the tour */
  startTour: () => void;
  /** End the tour */
  endTour: () => void;
  /** Go to next step */
  nextStep: () => void;
  /** Go to previous step */
  prevStep: () => void;
  /** Go to a specific step */
  goToStep: (step: number) => void;
  /** Current step data */
  currentStepData: TourStep | null;
}

// ============================================================================
// Context
// ============================================================================

const TourContext = createContext<TourContextValue | null>(null);

export function useTour() {
  const context = useContext(TourContext);
  if (!context) {
    throw new Error('useTour must be used within a TourProvider');
  }
  return context;
}

// ============================================================================
// Provider
// ============================================================================

interface TourProviderProps {
  children: React.ReactNode;
  steps: TourStep[];
  /** Storage key for persisting completion state */
  storageKey?: string;
  /** Whether to auto-start on first visit */
  autoStart?: boolean;
  /** Callback when tour completes */
  onComplete?: () => void;
}

export function TourProvider({
  children,
  steps,
  storageKey = 'edgequake.tour.completed',
  autoStart = false,
  onComplete,
}: TourProviderProps) {
  const [isActive, setIsActive] = useState(false);
  const [currentStep, setCurrentStep] = useState(0);

  // Check if tour has been completed before
  useEffect(() => {
    if (!autoStart) return;
    
    try {
      const completed = localStorage.getItem(storageKey);
      if (!completed && steps.length > 0) {
        // Auto-start after a short delay
        const timer = setTimeout(() => setIsActive(true), 1000);
        return () => clearTimeout(timer);
      }
    } catch {
      // Ignore localStorage errors
    }
  }, [autoStart, storageKey, steps.length]);

  const startTour = useCallback(() => {
    setCurrentStep(0);
    setIsActive(true);
  }, []);

  const endTour = useCallback(() => {
    setIsActive(false);
    setCurrentStep(0);
    
    // Mark as completed
    try {
      localStorage.setItem(storageKey, 'true');
    } catch {
      // Ignore localStorage errors
    }
    
    onComplete?.();
  }, [storageKey, onComplete]);

  const nextStep = useCallback(() => {
    if (currentStep < steps.length - 1) {
      steps[currentStep]?.onHide?.();
      setCurrentStep((s) => s + 1);
    } else {
      endTour();
    }
  }, [currentStep, steps, endTour]);

  const prevStep = useCallback(() => {
    if (currentStep > 0) {
      steps[currentStep]?.onHide?.();
      setCurrentStep((s) => s - 1);
    }
  }, [currentStep, steps]);

  const goToStep = useCallback((step: number) => {
    if (step >= 0 && step < steps.length) {
      steps[currentStep]?.onHide?.();
      setCurrentStep(step);
    }
  }, [currentStep, steps]);

  const currentStepData = steps[currentStep] || null;

  // Trigger onShow when step changes
  useEffect(() => {
    if (isActive && currentStepData) {
      currentStepData.onShow?.();
    }
  }, [isActive, currentStep, currentStepData]);

  const value: TourContextValue = {
    isActive,
    currentStep,
    totalSteps: steps.length,
    startTour,
    endTour,
    nextStep,
    prevStep,
    goToStep,
    currentStepData,
  };

  return (
    <TourContext.Provider value={value}>
      {children}
      {isActive && <TourOverlay />}
    </TourContext.Provider>
  );
}

// ============================================================================
// Overlay Component
// ============================================================================

function TourOverlay() {
  const { currentStepData, currentStep, totalSteps, nextStep, prevStep, endTour } = useTour();
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const mounted = typeof window !== 'undefined';

  // Find and track target element
  useEffect(() => {
    if (!currentStepData) return;

    const findTarget = () => {
      const target = document.querySelector(currentStepData.target);
      if (target) {
        const rect = target.getBoundingClientRect();
        setTargetRect(rect);
        
        // Scroll target into view if needed
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
      } else {
        setTargetRect(null);
      }
    };

    // Initial find
    findTarget();

    // Watch for layout changes
    const observer = new ResizeObserver(findTarget);
    observer.observe(document.body);

    // Also update on scroll
    window.addEventListener('scroll', findTarget, true);

    return () => {
      observer.disconnect();
      window.removeEventListener('scroll', findTarget, true);
    };
  }, [currentStepData]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        endTour();
      } else if (e.key === 'ArrowRight' || e.key === 'Enter') {
        nextStep();
      } else if (e.key === 'ArrowLeft') {
        prevStep();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [endTour, nextStep, prevStep]);

  if (!mounted || !currentStepData) return null;

  const placement = currentStepData.placement || 'bottom';
  
  // Calculate popover position
  const getPopoverStyle = (): React.CSSProperties => {
    if (!targetRect) {
      // Center if no target found
      return {
        position: 'fixed',
        top: '50%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
      };
    }

    const padding = 12;
    const popoverWidth = 320;
    const popoverHeight = 180;

    switch (placement) {
      case 'top':
        return {
          position: 'fixed',
          top: targetRect.top - popoverHeight - padding,
          left: targetRect.left + targetRect.width / 2 - popoverWidth / 2,
        };
      case 'bottom':
        return {
          position: 'fixed',
          top: targetRect.bottom + padding,
          left: targetRect.left + targetRect.width / 2 - popoverWidth / 2,
        };
      case 'left':
        return {
          position: 'fixed',
          top: targetRect.top + targetRect.height / 2 - popoverHeight / 2,
          left: targetRect.left - popoverWidth - padding,
        };
      case 'right':
        return {
          position: 'fixed',
          top: targetRect.top + targetRect.height / 2 - popoverHeight / 2,
          left: targetRect.right + padding,
        };
    }
  };

  const overlay = (
    <div className="fixed inset-0 z-9999" role="dialog" aria-modal="true">
      {/* Backdrop with cutout for target */}
      <svg
        className="absolute inset-0 w-full h-full"
        style={{ pointerEvents: 'none' }}
      >
        <defs>
          <mask id="tour-mask">
            <rect x="0" y="0" width="100%" height="100%" fill="white" />
            {targetRect && (
              <rect
                x={targetRect.left - 4}
                y={targetRect.top - 4}
                width={targetRect.width + 8}
                height={targetRect.height + 8}
                rx="8"
                fill="black"
              />
            )}
          </mask>
        </defs>
        <rect
          x="0"
          y="0"
          width="100%"
          height="100%"
          fill="rgba(0, 0, 0, 0.5)"
          mask="url(#tour-mask)"
          style={{ pointerEvents: 'auto' }}
          onClick={endTour}
        />
      </svg>

      {/* Highlight ring around target */}
      {targetRect && (
        <div
          className="absolute border-2 border-primary rounded-lg animate-pulse pointer-events-none"
          style={{
            top: targetRect.top - 4,
            left: targetRect.left - 4,
            width: targetRect.width + 8,
            height: targetRect.height + 8,
          }}
        />
      )}

      {/* Popover */}
      <div
        className={cn(
          'w-80 bg-popover text-popover-foreground rounded-lg shadow-xl border',
          'animate-in fade-in-0 zoom-in-95 duration-200'
        )}
        style={getPopoverStyle()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h3 className="font-semibold text-sm">{currentStepData.title}</h3>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 -mr-2"
            onClick={endTour}
            aria-label="Close tour"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        {/* Content */}
        <div className="p-4">
          <div className="text-sm text-muted-foreground">
            {currentStepData.content}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t bg-muted/30">
          {/* Progress dots */}
          <div className="flex items-center gap-1">
            {Array.from({ length: totalSteps }).map((_, i) => (
              <div
                key={i}
                className={cn(
                  'w-2 h-2 rounded-full transition-colors',
                  i === currentStep ? 'bg-primary' : 'bg-muted-foreground/30'
                )}
              />
            ))}
          </div>

          {/* Navigation */}
          <div className="flex items-center gap-2">
            {currentStep > 0 && (
              <Button variant="ghost" size="sm" onClick={prevStep}>
                <ChevronLeft className="h-4 w-4 mr-1" />
                Back
              </Button>
            )}
            <Button size="sm" onClick={nextStep}>
              {currentStep === totalSteps - 1 ? 'Finish' : 'Next'}
              {currentStep < totalSteps - 1 && <ChevronRight className="h-4 w-4 ml-1" />}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );

  return createPortal(overlay, document.body);
}

// ============================================================================
// Trigger Button
// ============================================================================

interface TourTriggerProps {
  className?: string;
  variant?: 'default' | 'ghost' | 'outline';
}

export function TourTrigger({ className, variant = 'ghost' }: TourTriggerProps) {
  const { startTour, isActive } = useTour();

  if (isActive) return null;

  return (
    <Button
      variant={variant}
      size="icon"
      className={cn('h-8 w-8', className)}
      onClick={startTour}
      aria-label="Start guided tour"
    >
      <HelpCircle className="h-4 w-4" />
    </Button>
  );
}

// ============================================================================
// Reset Tour Button (for settings)
// ============================================================================

interface ResetTourButtonProps {
  storageKey?: string;
  className?: string;
}

export function ResetTourButton({ storageKey = 'edgequake.tour.completed', className }: ResetTourButtonProps) {
  const handleReset = () => {
    try {
      localStorage.removeItem(storageKey);
    } catch {
      // Ignore
    }
  };

  return (
    <Button variant="outline" size="sm" className={className} onClick={handleReset}>
      Reset Tour
    </Button>
  );
}

export default TourProvider;
