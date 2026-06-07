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
    color: "text-yellow-500",
    bgColor: "bg-yellow-50 border-yellow-500",
  },
  {
    key: "processing",
    label: "Processing",
    icon: Zap,
    color: "text-blue-500",
    bgColor: "bg-blue-50 border-blue-500",
  },
  {
    key: "completed",
    label: "Completed",
    icon: CheckCircle,
    color: "text-green-500",
    bgColor: "bg-green-50 border-green-500",
  },
  {
    key: "failed",
    label: "Failed",
    icon: XCircle,
    color: "text-red-500",
    bgColor: "bg-red-50 border-red-500",
  },
] as const;

export const PIPELINE_MESSAGE_LEVEL_CONFIG = {
  info: {
    icon: Activity,
    color: "text-blue-500",
    bgColor: "bg-blue-50 dark:bg-blue-950",
  },
  warn: {
    icon: AlertCircle,
    color: "text-yellow-500",
    bgColor: "bg-yellow-50 dark:bg-yellow-950",
  },
  error: {
    icon: XCircle,
    color: "text-red-500",
    bgColor: "bg-red-50 dark:bg-red-950",
  },
} as const;
