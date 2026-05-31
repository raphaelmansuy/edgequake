/**
 * Shared LLM provider icon — single source of truth (SPEC-017 UI-DRY-007).
 */
import { cn } from "@/lib/utils";
import {
  Brain,
  Cloud,
  Cpu,
  FlaskConical,
  Globe,
  Sparkles,
  Zap,
} from "lucide-react";

/** Tailwind color class for a provider badge/icon. */
export function getProviderIconColorClass(
  providerId: string | undefined,
): string {
  switch (providerId?.toLowerCase()) {
    case "openai":
      return "text-green-600";
    case "ollama":
      return "text-blue-600";
    case "lmstudio":
      return "text-purple-600";
    case "anthropic":
      return "text-orange-600";
    case "gemini":
      return "text-blue-500";
    case "xai":
      return "text-slate-700 dark:text-slate-300";
    case "openrouter":
      return "text-indigo-600";
    case "azure":
      return "text-sky-600";
    case "minimax":
      return "text-teal-600";
    case "mock":
      return "text-gray-500";
    default:
      return "text-muted-foreground";
  }
}

export interface ProviderIconProps {
  providerId: string | undefined;
  className?: string;
}

/** Renders the Lucide icon for an LLM/embedding provider. */
export function ProviderIcon({ providerId, className }: ProviderIconProps) {
  const iconClass = cn("h-4 w-4", getProviderIconColorClass(providerId), className);
  switch (providerId?.toLowerCase()) {
    case "openai":
    case "azure":
      return <Cloud className={iconClass} />;
    case "ollama":
      return <Cpu className={iconClass} />;
    case "lmstudio":
      return <Brain className={iconClass} />;
    case "gemini":
      return <Zap className={iconClass} />;
    case "openrouter":
      return <Globe className={iconClass} />;
    case "mock":
      return <FlaskConical className={iconClass} />;
    case "anthropic":
    case "minimax":
    case "xai":
      return <Sparkles className={iconClass} />;
    default:
      return <Brain className={iconClass} />;
  }
}
