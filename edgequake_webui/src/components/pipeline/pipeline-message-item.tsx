"use client";

import { formatDistanceToNow } from "date-fns";
import { useMemo } from "react";
import {
  PIPELINE_MESSAGE_LEVEL_CONFIG,
} from "@/lib/pipeline/pipeline-phases";
import { replaceUuidsInMessage } from "@/lib/pipeline/pipeline-formatters";
import type { PipelineMessage } from "@/types";

export interface PipelineMessageItemProps {
  message: PipelineMessage;
  documentMap: Map<string, string>;
}

export function PipelineMessageItem({
  message,
  documentMap,
}: PipelineMessageItemProps) {
  const config =
    PIPELINE_MESSAGE_LEVEL_CONFIG[
      message.level as keyof typeof PIPELINE_MESSAGE_LEVEL_CONFIG
    ] || PIPELINE_MESSAGE_LEVEL_CONFIG.info;
  const Icon = config.icon;

  const formattedMessage = useMemo(
    () => replaceUuidsInMessage(message.message, documentMap),
    [message.message, documentMap],
  );

  return (
    <div
      className={`flex items-start gap-2 py-1.5 px-2 rounded text-xs ${config.bgColor}`}
    >
      <Icon className={`h-3 w-3 mt-0.5 shrink-0 ${config.color}`} />
      <div className="flex-1 min-w-0">
        <p className="break-words">{formattedMessage}</p>
        <p className="text-[10px] text-muted-foreground mt-0.5">
          {formatDistanceToNow(new Date(message.timestamp), { addSuffix: true })}
        </p>
      </div>
    </div>
  );
}
