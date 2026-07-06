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
    case "nvidia":
      return "text-lime-600";
    case "cohere":
      return "text-rose-600";
    case "huggingface":
      return "text-yellow-600";
    case "jina":
      return "text-cyan-600";
    case "vscode-copilot":
      return "text-violet-600";
    case "bedrock":
      return "text-orange-700";
    case "vertexai":
      return "text-blue-700";
    case "mistral":
      return "text-emerald-600";
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
    case "cohere":
    case "nvidia":
      return <Sparkles className={iconClass} />;
    case "huggingface":
    case "jina":
      return <Globe className={iconClass} />;
    case "vscode-copilot":
    case "bedrock":
      return <Cloud className={iconClass} />;
    case "mistral":
    case "vertexai":
      return <Zap className={iconClass} />;
    default:
      return <Brain className={iconClass} />;
  }
}
