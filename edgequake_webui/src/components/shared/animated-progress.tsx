/**
 * Animated Progress Component
 * 
 * Smooth animated progress bar with variants.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 */

'use client';

import { cn } from '@/lib/utils';
import { useEffect, useRef, useState } from 'react';

interface AnimatedProgressProps {
  /** Progress value (0-100) */
  value: number;
  /** Maximum value (default: 100) */
  max?: number;
  /** Show percentage value */
  showValue?: boolean;
  /** Color variant */
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  /** Enable smooth animation */
  animated?: boolean;
  /** Show indeterminate animation when value is unknown */
  indeterminate?: boolean;
  /** Show striped pattern */
  striped?: boolean;
  /** Custom class name */
  className?: string;
}

const variantStyles = {
  default: 'bg-primary',
  success: 'bg-green-500',
  warning: 'bg-yellow-500',
  error: 'bg-red-500',
  info: 'bg-blue-500',
};

const sizeStyles = {
  sm: 'h-1',
  md: 'h-2',
  lg: 'h-3',
};

const bgSizeStyles = {
  sm: 'h-1',
  md: 'h-2',
  lg: 'h-3',
};

/**
 * Animated progress bar component.
 * 
 * Features:
 * - Smooth value transitions
 * - Multiple color variants
 * - Indeterminate mode for unknown progress
 * - Striped pattern option
 */
export function AnimatedProgress({
  value,
  max = 100,
  showValue = false,
  variant = 'default',
  size = 'md',
  animated = true,
  indeterminate = false,
  striped = false,
  className,
}: AnimatedProgressProps) {
  const [displayValue, setDisplayValue] = useState(value);
  const prevValueRef = useRef(value);

  // Smooth animation using requestAnimationFrame
  // Note: This is a valid pattern for animation - setState is called in animation frame
  useEffect(() => {
    if (!animated || indeterminate) {
      // Intentional: Direct sync for non-animated mode
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setDisplayValue(value);
      return;
    }

    const start = prevValueRef.current;
    const end = value;
    const duration = 300; // ms
    const startTime = performance.now();

    const animate = (currentTime: number) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      // Ease out cubic
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = start + (end - start) * eased;
      
      setDisplayValue(current);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        prevValueRef.current = end;
      }
    };

    requestAnimationFrame(animate);
  }, [value, animated, indeterminate]);

  const percentage = Math.min(Math.max((displayValue / max) * 100, 0), 100);

  return (
    <div className={cn('w-full', className)}>
      {/* Progress bar container */}
      <div
        className={cn(
          'w-full rounded-full bg-secondary overflow-hidden',
          bgSizeStyles[size]
        )}
        role="progressbar"
        aria-valuenow={indeterminate ? undefined : Math.round(value)}
        aria-valuemin={0}
        aria-valuemax={max}
      >
        {/* Progress fill */}
        <div
          className={cn(
            'h-full rounded-full transition-all',
            variantStyles[variant],
            sizeStyles[size],
            indeterminate && 'animate-indeterminate',
            striped && 'bg-stripes'
          )}
          style={{
            width: indeterminate ? '30%' : `${percentage}%`,
            transition: animated ? 'width 0.1s ease-out' : 'none',
          }}
        />
      </div>

      {/* Optional value display */}
      {showValue && !indeterminate && (
        <div className="flex justify-end mt-1">
          <span className="text-xs text-muted-foreground">
            {Math.round(value)}%
          </span>
        </div>
      )}

      <style jsx>{`
        @keyframes indeterminate {
          0% {
            transform: translateX(-100%);
          }
          100% {
            transform: translateX(400%);
          }
        }
        
        .animate-indeterminate {
          animation: indeterminate 1.5s ease-in-out infinite;
        }

        .bg-stripes {
          background-image: linear-gradient(
            45deg,
            rgba(255, 255, 255, 0.15) 25%,
            transparent 25%,
            transparent 50%,
            rgba(255, 255, 255, 0.15) 50%,
            rgba(255, 255, 255, 0.15) 75%,
            transparent 75%,
            transparent
          );
          background-size: 1rem 1rem;
          animation: stripes 1s linear infinite;
        }

        @keyframes stripes {
          from {
            background-position: 1rem 0;
          }
          to {
            background-position: 0 0;
          }
        }
      `}</style>
    </div>
  );
}

/**
 * Circular progress indicator.
 */
interface CircularProgressProps {
  value: number;
  max?: number;
  size?: number;
  strokeWidth?: number;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  showValue?: boolean;
  className?: string;
}

const circularVariantStyles = {
  default: 'stroke-primary',
  success: 'stroke-green-500',
  warning: 'stroke-yellow-500',
  error: 'stroke-red-500',
  info: 'stroke-blue-500',
};

export function CircularProgress({
  value,
  max = 100,
  size = 40,
  strokeWidth = 4,
  variant = 'default',
  showValue = false,
  className,
}: CircularProgressProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const percentage = Math.min(Math.max((value / max) * 100, 0), 100);
  const offset = circumference - (percentage / 100) * circumference;

  return (
    <div className={cn('relative inline-flex', className)}>
      <svg
        width={size}
        height={size}
        className="transform -rotate-90"
      >
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          className="stroke-secondary"
          strokeWidth={strokeWidth}
        />
        {/* Progress circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          className={cn(circularVariantStyles[variant], 'transition-all duration-300')}
          strokeWidth={strokeWidth}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
        />
      </svg>
      
      {showValue && (
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-xs font-medium">
            {Math.round(percentage)}%
          </span>
        </div>
      )}
    </div>
  );
}

export default AnimatedProgress;
