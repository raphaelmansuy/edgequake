import {
  Activity,
  AlertCircle,
  CheckCircle,
  Clock,
  XCircle,
  Zap,
} from "lucide-react";

export const PIPELINE_PHASES = [
  {
    key: "pending",
    label: "Pending",
    icon: Clock,
    color: "text-yellow-700 dark:text-yellow-400",
    bgColor: "bg-yellow-50 border-yellow-500 dark:bg-yellow-500/10 dark:border-yellow-500/60",
  },
  {
    key: "processing",
    label: "Processing",
    icon: Zap,
    color: "text-blue-600 dark:text-blue-400",
    bgColor: "bg-blue-50 border-blue-500 dark:bg-blue-500/10 dark:border-blue-500/60",
  },
  {
    key: "completed",
    label: "Completed",
    icon: CheckCircle,
    color: "text-green-500",
    bgColor: "bg-green-50 border-green-500 dark:bg-green-500/10 dark:border-green-500/60",
  },
  {
    key: "failed",
    label: "Failed",
    icon: XCircle,
    color: "text-red-500",
    bgColor: "bg-red-50 border-red-500 dark:bg-red-500/10 dark:border-red-500/60",
  },
] as const;

export const PIPELINE_MESSAGE_LEVEL_CONFIG = {
  info: {
    icon: Activity,
    color: "text-blue-600 dark:text-blue-400",
    bgColor: "bg-blue-50 dark:bg-blue-950",
  },
  warn: {
    icon: AlertCircle,
    color: "text-yellow-700 dark:text-yellow-400",
    bgColor: "bg-yellow-50 dark:bg-yellow-950",
  },
  error: {
    icon: XCircle,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-950",
  },
} as const;
