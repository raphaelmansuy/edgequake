"use client";

import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import type { ProviderResponse } from "@/lib/api/models";
import { CheckCircle, Server, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspaceProviderHealthCardProps {
  providerHealth?: ProviderResponse[];
  isLoadingHealth: boolean;
}

export function WorkspaceProviderHealthCard({
  providerHealth,
  isLoadingHealth,
}: WorkspaceProviderHealthCardProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Server className="h-5 w-5 text-slate-600" />
          {t("workspace.providerHealth", "Provider Status")}
        </CardTitle>
        <CardDescription>
          {t(
            "workspace.providerHealthDesc",
            "Real-time availability of configured LLM and embedding providers.",
          )}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isLoadingHealth ? (
          <div className="flex gap-2">
            {[...Array(3)].map((_, i) => (
              <Skeleton key={i} className="h-8 w-24" />
            ))}
          </div>
        ) : providerHealth && providerHealth.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {providerHealth
              .filter((p) => p.enabled)
              .map((provider) => {
                const isAvailable =
                  provider.health?.available ?? provider.enabled;
                return (
                  <Badge
                    key={provider.name}
                    variant={isAvailable ? "default" : "secondary"}
                    className={`flex items-center gap-1.5 px-3 py-1.5 ${
                      isAvailable
                        ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300 border-green-200 dark:border-green-800"
                        : "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-300 border-red-200 dark:border-red-800"
                    }`}
                  >
                    {isAvailable ? (
                      <CheckCircle className="h-3 w-3" />
                    ) : (
                      <XCircle className="h-3 w-3" />
                    )}
                    <span className="capitalize">
                      {provider.display_name || provider.name}
                    </span>
                    {provider.models && provider.models.length > 0 && (
                      <span className="text-xs opacity-70">
                        ({provider.models.length})
                      </span>
                    )}
                  </Badge>
                );
              })}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            {t("workspace.noProvidersConfigured", "No providers configured")}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
