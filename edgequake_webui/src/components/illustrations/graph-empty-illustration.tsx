"use client";

import { cn } from "@/lib/utils";
import { memo } from "react";

interface GraphEmptyIllustrationProps {
  className?: string;
  animate?: boolean;
}

/**
 * An animated SVG illustration for empty graph states.
 * Shows a stylized knowledge graph with floating nodes and connections.
 */
export const GraphEmptyIllustration = memo(function GraphEmptyIllustration({
  className,
  animate = true,
}: GraphEmptyIllustrationProps) {
  return (
    <svg
      viewBox="0 0 200 160"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={cn("w-full h-full", className)}
      aria-hidden="true"
    >
      {/* Background grid pattern */}
      <defs>
        <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
          <path
            d="M 20 0 L 0 0 0 20"
            fill="none"
            stroke="currentColor"
            strokeOpacity="0.05"
            strokeWidth="0.5"
          />
        </pattern>
        
        {/* Gradient for nodes */}
        <linearGradient id="nodeGradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="var(--primary)" stopOpacity="0.8" />
          <stop offset="100%" stopColor="var(--primary)" stopOpacity="0.4" />
        </linearGradient>
        
        {/* Glow filter for active nodes */}
        <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="2" result="coloredBlur" />
          <feMerge>
            <feMergeNode in="coloredBlur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* Grid background */}
      <rect width="200" height="160" fill="url(#grid)" />

      {/* Connection lines (edges) - drawn first so they appear behind nodes */}
      <g className="text-muted-foreground">
        {/* Line from center to top-left */}
        <line
          x1="100"
          y1="80"
          x2="50"
          y2="40"
          stroke="currentColor"
          strokeOpacity="0.2"
          strokeWidth="1.5"
          strokeDasharray={animate ? "4 2" : "none"}
          className={animate ? "animate-dash" : ""}
        />
        {/* Line from center to top-right */}
        <line
          x1="100"
          y1="80"
          x2="150"
          y2="35"
          stroke="currentColor"
          strokeOpacity="0.2"
          strokeWidth="1.5"
          strokeDasharray={animate ? "4 2" : "none"}
          className={animate ? "animate-dash" : ""}
          style={{ animationDelay: "0.5s" }}
        />
        {/* Line from center to bottom-left */}
        <line
          x1="100"
          y1="80"
          x2="40"
          y2="110"
          stroke="currentColor"
          strokeOpacity="0.2"
          strokeWidth="1.5"
          strokeDasharray={animate ? "4 2" : "none"}
          className={animate ? "animate-dash" : ""}
          style={{ animationDelay: "1s" }}
        />
        {/* Line from center to bottom-right */}
        <line
          x1="100"
          y1="80"
          x2="160"
          y2="115"
          stroke="currentColor"
          strokeOpacity="0.2"
          strokeWidth="1.5"
          strokeDasharray={animate ? "4 2" : "none"}
          className={animate ? "animate-dash" : ""}
          style={{ animationDelay: "1.5s" }}
        />
        {/* Secondary connections */}
        <line
          x1="50"
          y1="40"
          x2="30"
          y2="70"
          stroke="currentColor"
          strokeOpacity="0.1"
          strokeWidth="1"
        />
        <line
          x1="150"
          y1="35"
          x2="175"
          y2="65"
          stroke="currentColor"
          strokeOpacity="0.1"
          strokeWidth="1"
        />
        <line
          x1="40"
          y1="110"
          x2="70"
          y2="135"
          stroke="currentColor"
          strokeOpacity="0.1"
          strokeWidth="1"
        />
        <line
          x1="160"
          y1="115"
          x2="140"
          y2="140"
          stroke="currentColor"
          strokeOpacity="0.1"
          strokeWidth="1"
        />
      </g>

      {/* Nodes */}
      <g>
        {/* Center node - largest, represents main entity */}
        <circle
          cx="100"
          cy="80"
          r="16"
          fill="url(#nodeGradient)"
          filter={animate ? "url(#glow)" : undefined}
          className={animate ? "animate-pulse-subtle" : ""}
        />
        <circle
          cx="100"
          cy="80"
          r="8"
          fill="var(--background)"
          fillOpacity="0.3"
        />

        {/* Primary nodes - medium size */}
        <circle
          cx="50"
          cy="40"
          r="10"
          fill="#3b82f6"
          fillOpacity="0.7"
          className={animate ? "animate-float" : ""}
          style={{ animationDelay: "0s" }}
        />
        <circle
          cx="150"
          cy="35"
          r="10"
          fill="#8b5cf6"
          fillOpacity="0.7"
          className={animate ? "animate-float" : ""}
          style={{ animationDelay: "0.5s" }}
        />
        <circle
          cx="40"
          cy="110"
          r="10"
          fill="#22c55e"
          fillOpacity="0.7"
          className={animate ? "animate-float" : ""}
          style={{ animationDelay: "1s" }}
        />
        <circle
          cx="160"
          cy="115"
          r="10"
          fill="#f97316"
          fillOpacity="0.7"
          className={animate ? "animate-float" : ""}
          style={{ animationDelay: "1.5s" }}
        />

        {/* Secondary nodes - small */}
        <circle cx="30" cy="70" r="5" fill="#94a3b8" fillOpacity="0.5" />
        <circle cx="175" cy="65" r="5" fill="#94a3b8" fillOpacity="0.5" />
        <circle cx="70" cy="135" r="5" fill="#94a3b8" fillOpacity="0.5" />
        <circle cx="140" cy="140" r="5" fill="#94a3b8" fillOpacity="0.5" />
        
        {/* Decorative tiny nodes */}
        <circle cx="25" cy="45" r="3" fill="#94a3b8" fillOpacity="0.3" />
        <circle cx="180" cy="95" r="3" fill="#94a3b8" fillOpacity="0.3" />
        <circle cx="85" cy="145" r="3" fill="#94a3b8" fillOpacity="0.3" />
        <circle cx="120" cy="25" r="3" fill="#94a3b8" fillOpacity="0.3" />
      </g>

      {/* Question mark or plus symbol in center */}
      <g className="text-primary-foreground">
        <path
          d="M97 77 L103 77 M100 74 L100 80"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          opacity="0.8"
        />
      </g>
    </svg>
  );
});

export default GraphEmptyIllustration;
