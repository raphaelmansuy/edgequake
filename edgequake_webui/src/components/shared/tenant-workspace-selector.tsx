'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { EmbeddingModelSelector, type EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import {
    createTenant,
    createWorkspace,
    getTenants,
    getWorkspaces,
} from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Tenant, Workspace } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
    Building2,
    Check,
    FolderKanban,
    Loader2,
    Plus,
    RefreshCw
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface TenantWorkspaceSelectorProps {
  /**
   * Whether to show in compact mode (icon only)
   */
  compact?: boolean;
  /**
   * Callback when tenant changes
   */
  onTenantChange?: (tenant: Tenant) => void;
  /**
   * Callback when workspace changes
   */
  onWorkspaceChange?: (workspace: Workspace) => void;
}

export function TenantWorkspaceSelector({
  compact = false,
  onTenantChange,
  onWorkspaceChange,
}: TenantWorkspaceSelectorProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // Store state
  const {
    tenants,
    workspaces,
    selectedTenantId,
    selectedWorkspaceId,
    setTenants,
    setWorkspaces,
    selectTenant,
    selectWorkspace,
    initializeFromStorage,
  } = useTenantStore();

  // Dialog states
  const [showCreateTenant, setShowCreateTenant] = useState(false);
  const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
  const [newTenantName, setNewTenantName] = useState('');
  const [newTenantDescription, setNewTenantDescription] = useState('');
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [newWorkspaceDescription, setNewWorkspaceDescription] = useState('');
  // Workspace model selection (for workspace creation)
  const [selectedLLM, setSelectedLLM] = useState<LLMSelection | undefined>(undefined);
  const [selectedEmbedding, setSelectedEmbedding] = useState<EmbeddingSelection | undefined>(undefined);
  // Tenant default model selection (SPEC-032: for tenant creation)
  const [tenantDefaultLLM, setTenantDefaultLLM] = useState<LLMSelection | undefined>(undefined);
  const [tenantDefaultEmbedding, setTenantDefaultEmbedding] = useState<EmbeddingSelection | undefined>(undefined);
  // Tenant default vision LLM (SPEC-041: vision model selection)
  const [tenantDefaultVision, setTenantDefaultVision] = useState<LLMSelection | undefined>(undefined);

  // Initialize from storage on mount
  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  // Fetch tenants
  const {
    data: tenantsData,
    isLoading: isLoadingTenants,
    refetch: refetchTenants,
  } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000, // Cache for 1 minute
  });

  // Update store when tenants are fetched
  useEffect(() => {
    if (tenantsData) {
      setTenants(tenantsData);
      // Auto-select first tenant if none selected
      if (!selectedTenantId && tenantsData.length > 0) {
        selectTenant(tenantsData[0].id);
      }
    }
  }, [tenantsData, setTenants, selectedTenantId, selectTenant]);

  // Fetch workspaces for selected tenant
  const {
    data: workspacesData,
    isLoading: isLoadingWorkspaces,
    refetch: refetchWorkspaces,
  } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () =>
      selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  // Update store when workspaces are fetched
  useEffect(() => {
    if (workspacesData) {
      setWorkspaces(workspacesData);
      // Auto-select first workspace if none selected
      if (!selectedWorkspaceId && workspacesData.length > 0) {
        selectWorkspace(workspacesData[0].id);
      }
    }
  }, [workspacesData, setWorkspaces, selectedWorkspaceId, selectWorkspace]);

  // Create tenant mutation (SPEC-032: with default model configuration)
  const createTenantMutation = useMutation({
    mutationFn: (data: { 
      name: string; 
      description?: string;
      default_llm_model?: string;
      default_llm_provider?: string;
      default_embedding_model?: string;
      default_embedding_provider?: string;
      default_embedding_dimension?: number;
      // SPEC-041: Vision LLM defaults
      default_vision_llm_model?: string;
      default_vision_llm_provider?: string;
    }) =>
      createTenant(data),
    onSuccess: (newTenant) => {
      toast.success(t('tenant.createSuccess', 'Tenant created successfully'));
      // WHY: Immediately add to Zustand store so the dropdown reflects the new
      // tenant without waiting for the async query refetch. The query invalidation
      // that follows will sync fresh server data, but selectTenant() needs the
      // tenant in the list now to show the correct display name in the Select.
      const currentTenants = useTenantStore.getState().tenants;
      setTenants([...currentTenants, newTenant]);
      queryClient.invalidateQueries({ queryKey: ['tenants'] });
      selectTenant(newTenant.id);
      setShowCreateTenant(false);
      setNewTenantName('');
      setNewTenantDescription('');
      setTenantDefaultLLM(undefined);
      setTenantDefaultEmbedding(undefined);
      setTenantDefaultVision(undefined);
    },
    onError: (error) => {
      toast.error(
        t('tenant.createFailed', 'Failed to create tenant'),
        {
          description:
            error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  // Create workspace mutation
  const createWorkspaceMutation = useMutation({
    mutationFn: (data: { 
      name: string; 
      description?: string;
      llm_model?: string;
      llm_provider?: string;
      embedding_model?: string;
      embedding_provider?: string;
      embedding_dimension?: number;
    }) =>
      selectedTenantId
        ? createWorkspace(selectedTenantId, data)
        : Promise.reject(new Error('No tenant selected')),
    onSuccess: (newWorkspace) => {
      toast.success(
        t('workspace.createSuccess', 'Workspace created successfully')
      );
      // WHY: Immediately add to Zustand store so the dropdown reflects the new
      // workspace without waiting for the async query refetch. The query
      // invalidation that follows will sync fresh server data, but
      // selectWorkspace() needs the workspace in the list now so the Select
      // shows the correct display name instead of "Select workspace..."
      const currentWorkspaces = useTenantStore.getState().workspaces;
      setWorkspaces([...currentWorkspaces, newWorkspace]);
      queryClient.invalidateQueries({
        queryKey: ['workspaces', selectedTenantId],
      });
      selectWorkspace(newWorkspace.id);
      setShowCreateWorkspace(false);
      setNewWorkspaceName('');
      setNewWorkspaceDescription('');
      setSelectedLLM(undefined);
      setSelectedEmbedding(undefined);
    },
    onError: (error) => {
      toast.error(
        t('workspace.createFailed', 'Failed to create workspace'),
        {
          description:
            error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  const handleTenantSelect = useCallback(
    (tenantId: string) => {
      if (tenantId === selectedTenantId) return;
      selectTenant(tenantId);
      const tenant = tenants.find((t) => t.id === tenantId);
      if (tenant) {
        toast.info(t('tenant.switched', `Switched to tenant "{{name}}"`, { name: tenant.name }), {
          id: 'tenant-switch',
          duration: 2000,
        });
        onTenantChange?.(tenant);
      }
    },
    [selectTenant, tenants, selectedTenantId, onTenantChange, t]
  );

  const handleWorkspaceSelect = useCallback(
    (workspaceId: string) => {
      if (workspaceId === selectedWorkspaceId) return;
      selectWorkspace(workspaceId);
      // Invalidate workspace stats query to force refetch with new workspace
      queryClient.invalidateQueries({ queryKey: ['workspaceStats'] });
      const workspace = workspaces.find((w) => w.id === workspaceId);
      if (workspace) {
        toast.info(t('workspace.switched', `Switched to workspace "{{name}}"`, { name: workspace.name }), {
          id: 'workspace-switch',
          duration: 2000,
        });
        onWorkspaceChange?.(workspace);
      }
    },
    [selectWorkspace, workspaces, onWorkspaceChange, queryClient]
  );

  const selectedTenant = tenants.find((t) => t.id === selectedTenantId);
  const selectedWorkspace = workspaces.find(
    (w) => w.id === selectedWorkspaceId
  );

  const isLoading = isLoadingTenants || isLoadingWorkspaces;

  // Compact mode - just show icon with tooltip
  if (compact) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="relative">
                  {isLoading ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Building2 className="h-4 w-4" />
                  )}
                  {selectedTenant && selectedWorkspace && (
                    <span className="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-green-500" />
                  )}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-64">
                <DropdownMenuLabel>
                  {t('tenant.selectContext', 'Select Context')}
                </DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="text-xs text-muted-foreground">
                    {t('tenant.tenant', 'Tenant')}
                  </DropdownMenuLabel>
                  {tenants.map((tenant) => (
                    <DropdownMenuItem
                      key={tenant.id}
                      onClick={() => handleTenantSelect(tenant.id)}
                    >
                      <Building2 className="mr-2 h-4 w-4" />
                      <span className="flex-1 truncate">{tenant.name}</span>
                      {tenant.id === selectedTenantId && (
                        <Check className="ml-2 h-4 w-4" />
                      )}
                    </DropdownMenuItem>
                  ))}
                  <DropdownMenuItem onClick={() => setShowCreateTenant(true)}>
                    <Plus className="mr-2 h-4 w-4" />
                    {t('tenant.createNew', 'Create New Tenant')}
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                {selectedTenantId && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuGroup>
                      <DropdownMenuLabel className="text-xs text-muted-foreground">
                        {t('workspace.workspace', 'Workspace')}
                      </DropdownMenuLabel>
                      {workspaces.map((workspace) => (
                        <DropdownMenuItem
                          key={workspace.id}
                          onClick={() => handleWorkspaceSelect(workspace.id)}
                        >
                          <FolderKanban className="mr-2 h-4 w-4" />
                          <span className="flex-1 truncate">
                            {workspace.name}
                          </span>
                          <span className="text-xs text-muted-foreground ml-2">
                            {workspace.document_count ?? 0} docs
                          </span>
                          {workspace.id === selectedWorkspaceId && (
                            <Check className="ml-2 h-4 w-4" />
                          )}
                        </DropdownMenuItem>
                      ))}
                      <DropdownMenuItem
                        onClick={() => setShowCreateWorkspace(true)}
                      >
                        <Plus className="mr-2 h-4 w-4" />
                        {t('workspace.createNew', 'Create New Workspace')}
                      </DropdownMenuItem>
                    </DropdownMenuGroup>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </TooltipTrigger>
          <TooltipContent>
            {selectedTenant && selectedWorkspace ? (
              <div className="text-xs">
                <div className="font-medium">{selectedTenant.name}</div>
                <div className="text-muted-foreground">
                  {selectedWorkspace.name}
                </div>
              </div>
            ) : (
              t('tenant.selectContext', 'Select Context')
            )}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Full mode - show selectors stacked vertically for sidebar
  return (
    <>
      <div data-testid="workspace-selector" className="flex flex-col gap-3 p-3 bg-muted/50 rounded-lg border border-border/50 overflow-hidden">
        {/* Tenant Selector */}
        <div className="flex flex-col gap-1.5 min-w-0">
          <Label className="text-xs font-semibold text-muted-foreground">
            {t('tenant.tenant', 'Tenant')}
          </Label>
          <div className="flex gap-1.5 items-center min-w-0">
            {isLoadingTenants ? (
              <Skeleton className="h-8 flex-1 min-w-0" />
            ) : (
              <Select
                value={selectedTenantId || ''}
                onValueChange={handleTenantSelect}
              >
                <SelectTrigger data-testid="tenant-select" className="h-8 text-xs flex-1 min-w-50 max-w-full">
                  <SelectValue
                    placeholder={t('tenant.selectTenant', 'Select tenant...')}
                  />
                </SelectTrigger>
                <SelectContent className="max-w-55">
                  {tenants.map((tenant) => (
                    <SelectItem key={tenant.id} value={tenant.id}>
                      <div className="flex items-center gap-2 min-w-0">
                        <Building2 className="h-3 w-3 text-muted-foreground shrink-0" />
                        <span className="truncate">{tenant.name}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => setShowCreateTenant(true)}
              title={t('tenant.createNew', 'Create New Tenant')}
            >
              <Plus className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => refetchTenants()}
              title={t('common.refresh', 'Refresh')}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Workspace Selector - Always show, even if tenant not selected */}
        <div className="flex flex-col gap-1.5 min-w-0">
          <Label className="text-xs font-semibold text-muted-foreground">
            {t('workspace.workspace', 'Workspace')}
          </Label>
          <div className="flex gap-1.5 items-center min-w-0">
            {isLoadingWorkspaces ? (
              <Skeleton className="h-8 flex-1 min-w-0" />
            ) : (
              <Select
                value={selectedWorkspaceId || ''}
                onValueChange={handleWorkspaceSelect}
                disabled={!selectedTenantId}
              >
                <SelectTrigger data-testid="workspace-select" className="h-8 text-xs flex-1 min-w-50 max-w-full">
                  <SelectValue
                    placeholder={
                      selectedTenantId
                        ? t('workspace.selectWorkspace', 'Select workspace...')
                        : t('workspace.selectTenantFirst', 'Select tenant first')
                    }
                  />
                </SelectTrigger>
                <SelectContent className="max-w-55">
                  {workspaces.map((workspace) => (
                    <SelectItem key={workspace.id} value={workspace.id}>
                      <div className="flex items-center gap-2 min-w-0">
                        <FolderKanban className="h-3 w-3 text-muted-foreground shrink-0" />
                        <span className="truncate">{workspace.name}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => setShowCreateWorkspace(true)}
              disabled={!selectedTenantId}
              title={t('workspace.createNew', 'Create New Workspace')}
            >
              <Plus className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 shrink-0"
              onClick={() => refetchWorkspaces()}
              disabled={!selectedTenantId}
              title={t('common.refresh', 'Refresh')}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Create Tenant Dialog (SPEC-032: with default model configuration) */}
      <Dialog open={showCreateTenant} onOpenChange={setShowCreateTenant}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {t('tenant.createNew', 'Create New Tenant')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'tenant.createDescription',
                'Create a new tenant to organize your workspaces and data.'
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="tenant-name">
                {t('tenant.name', 'Tenant Name')}
              </Label>
              <Input
                id="tenant-name"
                value={newTenantName}
                onChange={(e) => setNewTenantName(e.target.value)}
                placeholder={t('tenant.namePlaceholder', 'My Organization')}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="tenant-description">
                {t('tenant.description', 'Description')} ({t('common.optional', 'Optional')})
              </Label>
              <Input
                id="tenant-description"
                value={newTenantDescription}
                onChange={(e) => setNewTenantDescription(e.target.value)}
                placeholder={t(
                  'tenant.descriptionPlaceholder',
                  'A brief description...'
                )}
              />
            </div>

            {/* Default LLM Model Selection - SPEC-032 */}
            <div className="space-y-2">
              <Label>
                {t('tenant.defaultLlmModel', 'Default LLM Model')} ({t('common.optional', 'Optional')})
              </Label>
              <LLMModelSelector
                value={tenantDefaultLLM}
                onChange={setTenantDefaultLLM}
                showUsageHint
              />
              <p className="text-xs text-muted-foreground">
                {t('tenant.defaultLlmHint', 'New workspaces will inherit this default')}
              </p>
            </div>

            {/* Default Embedding Model Selection - SPEC-032 */}
            <div className="space-y-2">
              <Label>
                {t('tenant.defaultEmbeddingModel', 'Default Embedding Model')} ({t('common.optional', 'Optional')})
              </Label>
              <EmbeddingModelSelector
                value={tenantDefaultEmbedding}
                onChange={setTenantDefaultEmbedding}
              />
              <p className="text-xs text-muted-foreground">
                {t('tenant.defaultEmbeddingHint', 'New workspaces will inherit this default')}
              </p>
            </div>

            {/* Default Vision LLM Model Selection - SPEC-041 */}
            <div className="space-y-2">
              <Label>
                {t('tenant.defaultVisionLlmModel', 'Default Vision LLM Model')} ({t('common.optional', 'Optional')})
              </Label>
              <LLMModelSelector
                value={tenantDefaultVision}
                onChange={setTenantDefaultVision}
                showUsageHint
              />
              <p className="text-xs text-muted-foreground">
                {t('tenant.defaultVisionLlmHint', 'Used for PDF vision extraction. Workspaces inherit this if not overridden.')}
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCreateTenant(false)}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() =>
                createTenantMutation.mutate({
                  name: newTenantName,
                  description: newTenantDescription || undefined,
                  default_llm_model: tenantDefaultLLM?.model,
                  default_llm_provider: tenantDefaultLLM?.provider,
                  default_embedding_model: tenantDefaultEmbedding?.model,
                  default_embedding_provider: tenantDefaultEmbedding?.provider,
                  default_embedding_dimension: tenantDefaultEmbedding?.dimension,
                  // SPEC-041: Vision LLM defaults
                  default_vision_llm_model: tenantDefaultVision?.model,
                  default_vision_llm_provider: tenantDefaultVision?.provider,
                })
              }
              disabled={!newTenantName.trim() || createTenantMutation.isPending}
            >
              {createTenantMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('common.create', 'Create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Workspace Dialog */}
      <Dialog open={showCreateWorkspace} onOpenChange={setShowCreateWorkspace}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {t('workspace.createNew', 'Create New Workspace')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'workspace.createDescription',
                'Create a new workspace within the current tenant to organize your documents.'
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="workspace-name">
                {t('workspace.name', 'Workspace Name')}
              </Label>
              <Input
                id="workspace-name"
                value={newWorkspaceName}
                onChange={(e) => setNewWorkspaceName(e.target.value)}
                placeholder={t('workspace.namePlaceholder', 'Project Alpha')}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="workspace-description">
                {t('workspace.description', 'Description')} ({t('common.optional', 'Optional')})
              </Label>
              <Input
                id="workspace-description"
                value={newWorkspaceDescription}
                onChange={(e) => setNewWorkspaceDescription(e.target.value)}
                placeholder={t(
                  'workspace.descriptionPlaceholder',
                  'A brief description...'
                )}
              />
            </div>

            {/* LLM Model Selection - SPEC-032 */}
            <div className="space-y-2">
              <Label>
                {t('workspace.llmModel', 'LLM Model')} ({t('common.optional', 'Optional')})
              </Label>
              <LLMModelSelector
                value={selectedLLM}
                onChange={setSelectedLLM}
                showUsageHint
              />
            </div>

            {/* Embedding Model Selection - SPEC-032 */}
            <div className="space-y-2">
              <Label>
                {t('workspace.embeddingModel', 'Embedding Model')} ({t('common.optional', 'Optional')})
              </Label>
              <EmbeddingModelSelector
                value={selectedEmbedding}
                onChange={setSelectedEmbedding}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCreateWorkspace(false)}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() =>
                createWorkspaceMutation.mutate({
                  name: newWorkspaceName,
                  description: newWorkspaceDescription || undefined,
                  llm_model: selectedLLM?.model,
                  llm_provider: selectedLLM?.provider,
                  embedding_model: selectedEmbedding?.model,
                  embedding_provider: selectedEmbedding?.provider,
                  embedding_dimension: selectedEmbedding?.dimension,
                })
              }
              disabled={
                !newWorkspaceName.trim() || createWorkspaceMutation.isPending
              }
            >
              {createWorkspaceMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('common.create', 'Create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default TenantWorkspaceSelector;
