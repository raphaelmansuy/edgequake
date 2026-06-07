"use client";

import { useConversation, useConversations } from "@/hooks/use-conversations";
import { useProvidersHealth } from "@/hooks/use-models";
import { useLlmModels as useProviderLlmModels } from "@/hooks/use-providers";
import { isConversationNotFoundError } from "@/lib/query/conversation-errors";
import { sanitizeQueryModelSelection } from "@/lib/query-model-selection";
import { useActiveConversationId, useQueryUIStore } from "@/stores/use-query-ui-store";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

interface UseQueryConversationLifecycleOptions {
  onTenantContextChange: () => void;
}

/** Conversation load, recovery, model sanitization, and tenant isolation (SPEC-017 UI-P3-006). */
export function useQueryConversationLifecycle({
  onTenantContextChange,
}: UseQueryConversationLifecycleOptions) {
  const { t } = useTranslation();
  const hasInitializedRef = useRef(false);
  const hasResetUnavailableModelRef = useRef(false);

  const { querySettings, setQuerySettings } = useSettingsStore();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  const { data: llmCatalog } = useProviderLlmModels();
  const { data: providerHealth } = useProvidersHealth({
    enabled: true,
    refetchInterval: 60 * 1000,
  });

  const store = useQueryUIStore();
  const activeConversationId = useActiveConversationId();

  const {
    data: activeConversation,
    isLoading: isLoadingConversation,
    error: conversationError,
    isError: isConversationError,
  } = useConversation(activeConversationId);

  const { data: conversationsData } = useConversations({
    sort: "updated_at",
  });

  useEffect(() => {
    if (!isConversationError || !activeConversationId) return;
    if (!isConversationNotFoundError(conversationError)) return;

    store.setActiveConversation(null);
    toast(t("query.conversationExpired", "Previous conversation not available"), {
      description: t(
        "query.startingFreshSession",
        "Starting a fresh session.",
      ),
    });
  }, [isConversationError, conversationError, activeConversationId, store, t]);

  useEffect(() => {
    const sanitizedSelection = sanitizeQueryModelSelection(
      {
        provider: querySettings.provider,
        model: querySettings.model,
      },
      llmCatalog?.models,
      providerHealth,
    );

    if (
      sanitizedSelection.provider !== querySettings.provider ||
      sanitizedSelection.model !== querySettings.model
    ) {
      setQuerySettings(sanitizedSelection);

      if (
        !hasResetUnavailableModelRef.current &&
        (querySettings.provider || querySettings.model)
      ) {
        hasResetUnavailableModelRef.current = true;
        toast.warning(t("query.modelReset", "Model selection reset"), {
          description: t(
            "query.modelResetDesc",
            "Your previous model is unavailable in this environment, so the server default will be used.",
          ),
        });
      }
    }
  }, [
    llmCatalog?.models,
    providerHealth,
    querySettings.model,
    querySettings.provider,
    setQuerySettings,
    t,
  ]);

  useEffect(() => {
    if (hasInitializedRef.current) return;
    hasInitializedRef.current = true;

    const firstPage = conversationsData?.pages?.[0];
    if (!activeConversationId && firstPage?.items && firstPage.items.length > 0) {
      store.setActiveConversation(firstPage.items[0].id);
    }
  }, [activeConversationId, conversationsData, store]);

  useEffect(() => {
    const serverMessageCount = activeConversation?.messages?.length ?? 0;
    if (activeConversationId && serverMessageCount > 0) {
      store.setActiveConversation(null);
      onTenantContextChange();
      toast(t("query.conversationCleared", "New conversation started"), {
        description: t(
          "query.conversationClearedDesc",
          "Context has changed. Starting a fresh conversation.",
        ),
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedTenantId, selectedWorkspaceId]);

  return {
    activeConversation,
    activeConversationId,
    isLoadingConversation,
    querySettings,
    setQuerySettings,
    store,
  };
}
