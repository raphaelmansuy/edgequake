"use client";

import { useConversation, useConversations } from "@/hooks/use-conversations";
import { useScopedEligibleModels } from "@/hooks/use-model-policy";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import { isConversationNotFoundError } from "@/lib/query/conversation-errors";
import {
  isQueryHandoffNewConversation,
  pickResumedConversationId,
} from "@/lib/query/query-handoff";
import { applyProfile } from "@/lib/query/query-profile";
import { sanitizeQueryModelSelection } from "@/lib/query-model-selection";
import { selectKinds, useQueryKindsStore } from "@/stores/use-query-kinds-store";
import { useActiveConversationId, useQueryUIStore } from "@/stores/use-query-ui-store";
import { useSettingsStore } from "@/stores/use-settings-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useEffect, useMemo, useRef } from "react";
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
  const prevTenantRef = useRef<string | null>(null);
  const prevWorkspaceRef = useRef<string | null>(null);

  const savedSettings = useSettingsStore((s) => s.querySettings);
  const caps = useWorkspaceCapabilities();
  const setQuerySettings = useSettingsStore((s) => s.setQuerySettings);
  const setWorkspaceQueryModel = useSettingsStore((s) => s.setWorkspaceQueryModel);
  const selectedTenantId = useTenantStore((s) => s.selectedTenantId);
  const selectedWorkspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const workspacePair = useSettingsStore((s) =>
    selectedWorkspaceId ? s.queryModelsByWorkspace[selectedWorkspaceId] : undefined,
  );
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const docKinds = useQueryKindsStore(selectKinds(workspaceId));
  // SPEC-180: the mode and kind chips this workspace can run. The saved choice is untouched.
  const querySettings = useMemo(
    () => applyProfile(savedSettings, caps, docKinds),
    [savedSettings, caps, docKinds],
  );
  const eligible = useScopedEligibleModels({
    scope: "workspace",
    id: workspaceId,
    usage: "query",
  });

  const setActiveConversation = useQueryUIStore((s) => s.setActiveConversation);
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

    setActiveConversation(null);
    toast(t("query.conversationExpired", "Previous conversation not available"), {
      description: t(
        "query.startingFreshSession",
        "Starting a fresh session.",
      ),
    });
  }, [
    isConversationError,
    conversationError,
    activeConversationId,
    setActiveConversation,
    t,
  ]);

  const shown = workspacePair?.provider || workspacePair?.model
    ? workspacePair
    : { provider: querySettings.provider, model: querySettings.model };

  useEffect(() => {
    const sanitizedSelection = sanitizeQueryModelSelection(
      {
        provider: shown?.provider,
        model: shown?.model,
      },
      (eligible.data?.models ?? []).map((entry) => ({
        provider: entry.provider,
        name: entry.model,
      })),
      undefined,
      eligible.isFetched,
    );

    if (
      sanitizedSelection.provider !== shown?.provider ||
      sanitizedSelection.model !== shown?.model
    ) {
      if (selectedWorkspaceId) setWorkspaceQueryModel(selectedWorkspaceId, sanitizedSelection);
      else setQuerySettings(sanitizedSelection);

      if (
        !hasResetUnavailableModelRef.current &&
        (shown?.provider || shown?.model)
      ) {
        hasResetUnavailableModelRef.current = true;
        toast.warning(t("query.modelReset", "Model selection reset"), {
          description: t(
            "query.modelResetDesc",
            "This model is no longer allowed. Pick another, or use Inherit.",
          ),
        });
      }
    }
  }, [
    eligible.data?.models,
    eligible.isFetched,
    querySettings.model,
    querySettings.provider,
    selectedWorkspaceId,
    setQuerySettings,
    setWorkspaceQueryModel,
    shown?.model,
    shown?.provider,
    t,
    workspacePair,
  ]);

  useEffect(() => {
    if (hasInitializedRef.current) return;
    hasInitializedRef.current = true;

    const nextId = pickResumedConversationId({
      hasInitialized: false,
      activeConversationId,
      latestId: conversationsData?.pages?.[0]?.items?.[0]?.id,
      handoffNew: isQueryHandoffNewConversation(),
    });
    if (nextId === undefined) return;
    setActiveConversation(nextId);
  }, [activeConversationId, conversationsData, setActiveConversation]);

  useEffect(() => {
    if (!isQueryHandoffNewConversation()) return;
    if (activeConversationId) setActiveConversation(null);
  }, [activeConversationId, setActiveConversation]);

  useEffect(() => {
    if (prevTenantRef.current === null && prevWorkspaceRef.current === null) {
      prevTenantRef.current = selectedTenantId;
      prevWorkspaceRef.current = selectedWorkspaceId;
      return;
    }
    if (
      prevTenantRef.current === selectedTenantId &&
      prevWorkspaceRef.current === selectedWorkspaceId
    ) {
      return;
    }
    prevTenantRef.current = selectedTenantId;
    prevWorkspaceRef.current = selectedWorkspaceId;

    if (!activeConversationId) return;
    setActiveConversation(null);
    onTenantContextChange();
    toast(t("query.conversationCleared", "New conversation started"), {
      description: t(
        "query.conversationClearedDesc",
        "Context has changed. Starting a fresh conversation.",
      ),
    });
  }, [
    activeConversationId,
    onTenantContextChange,
    selectedTenantId,
    selectedWorkspaceId,
    setActiveConversation,
    t,
  ]);

  // Stable actions facade for stream session (avoid whole-store subscription)
  const store = useMemo(
    () => ({
      setActiveConversation: useQueryUIStore.getState().setActiveConversation,
    }),
    [],
  );

  return {
    activeConversation,
    activeConversationId,
    isLoadingConversation,
    querySettings,
    setQuerySettings,
    store: store as ReturnType<typeof useQueryUIStore.getState>,
  };
}
