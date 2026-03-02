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
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import {
    EmbeddingModelSelector,
    type EmbeddingSelection,
} from '@/components/workspace/embedding-model-selector';
import {
    LLMModelSelector,
    type LLMSelection,
} from '@/components/workspace/llm-model-selector';
import {
    createTenant,
    createWorkspace,
    getTenants,
    getWorkspaces,
} from '@/lib/api/edgequake';
import { cn } from '@/lib/utils';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
    Building2,
    Check,
    ChevronDown,
    FolderKanban,
    Loader2,
    Plus,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface HeaderTenantSelectorProps {
  className?: string;
}

/**
 * Compact tenant/workspace selector designed for header bar placement.
 * Shows current context with a slick dropdown for switching.
 * Includes full create tenant/workspace functionality.
 */
export function HeaderTenantSelector({ className }: HeaderTenantSelectorProps) {
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
    isInitialized,
    setInitialized,
  } = useTenantStore();

  // Dialog states
  const [showCreateTenant, setShowCreateTenant] = useState(false);
  const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
  const [newTenantName, setNewTenantName] = useState('');
  const [newTenantDescription, setNewTenantDescription] = useState('');
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [newWorkspaceDescription, setNewWorkspaceDescription] = useState('');
  const [newWorkspaceSlug, setNewWorkspaceSlug] = useState('');
  // SPEC-032: Workspace LLM configuration
  const [workspaceLLMSelection, setWorkspaceLLMSelection] = useState<LLMSelection | undefined>(undefined);
  // SPEC-032: Workspace embedding configuration
  const [embeddingSelection, setEmbeddingSelection] = useState<EmbeddingSelection | undefined>(undefined);
  // SPEC-041: Workspace Vision LLM for PDF-to-Markdown extraction
  const [workspaceVisionLLMSelection, setWorkspaceVisionLLMSelection] = useState<LLMSelection | undefined>(undefined);
  // SPEC-032: Tenant default LLM configuration
  const [tenantDefaultLLM, setTenantDefaultLLM] = useState<LLMSelection | undefined>(undefined);
  // SPEC-032: Tenant default embedding configuration
  const [tenantDefaultEmbedding, setTenantDefaultEmbedding] = useState<EmbeddingSelection | undefined>(undefined);
  // SPEC-041: Tenant default Vision LLM configuration
  const [tenantDefaultVisionLLM, setTenantDefaultVisionLLM] = useState<LLMSelection | undefined>(undefined);


  // Generate URL-safe slug from name
  const generateSlug = useCallback((name: string): string => {
    return name
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, '')
      .replace(/\s+/g, '-')
      .replace(/-+/g, '-')
      .substring(0, 50)
      .replace(/^-|-$/g, '');
  }, []);

  // Initialize from storage on mount
  useEffect(() => {
    initializeFromStorage();
  }, [initializeFromStorage]);

  // Fetch tenants
  const { data: tenantsData, isLoading: isLoadingTenants } = useQuery({
    queryKey: ['tenants'],
    queryFn: getTenants,
    staleTime: 60000,
  });

  // Update store when tenants are fetched - ENHANCED WITH AUTO-SELECTION
  useEffect(() => {
    if (tenantsData) {
      setTenants(tenantsData);
      
      // Auto-select logic: prioritize existing selection, then first available
      if (!selectedTenantId && tenantsData.length > 0) {
        selectTenant(tenantsData[0].id);
      }
      
      // Mark as initialized once we have tenant data
      if (!isInitialized) {
        setInitialized(true);
      }
    }
  }, [tenantsData, setTenants, selectedTenantId, selectTenant, isInitialized, setInitialized]);

  // Fetch workspaces for selected tenant
  const { data: workspacesData, isLoading: isLoadingWorkspaces } = useQuery({
    queryKey: ['workspaces', selectedTenantId],
    queryFn: () => selectedTenantId ? getWorkspaces(selectedTenantId) : Promise.resolve([]),
    enabled: !!selectedTenantId,
    staleTime: 60000,
  });

  // Update store when workspaces are fetched - ENHANCED WITH AUTO-SELECTION
  useEffect(() => {
    if (workspacesData) {
      setWorkspaces(workspacesData);
      
      // Auto-select first workspace if none selected
      if (!selectedWorkspaceId && workspacesData.length > 0) {
        selectWorkspace(workspacesData[0].id);
        
        // Show success toast for first-time auto-selection
        if (isInitialized && !localStorage.getItem('edgequake-workspace-initialized')) {
          toast.success(t('workspace.autoSelected', `Workspace "${workspacesData[0].name}" selected`), {
            description: t('workspace.autoSelectedDesc', 'You can change this anytime from the selector above'),
          });
          localStorage.setItem('edgequake-workspace-initialized', 'true');
        }
      }
    }
  }, [workspacesData, setWorkspaces, selectedWorkspaceId, selectWorkspace, isInitialized, t]);

  // Create tenant mutation
  // SPEC-032/SPEC-041: Updated to include LLM, embedding, and vision configuration
  const createTenantMutation = useMutation({
    mutationFn: (data: { 
      name: string; 
      description?: string;
      default_llm_model?: string;
      default_llm_provider?: string;
      default_embedding_model?: string;
      default_embedding_provider?: string;
      default_vision_llm_model?: string;
      default_vision_llm_provider?: string;
    }) => createTenant(data),
    onSuccess: (newTenant) => {
      toast.success(t('tenant.createSuccess', 'Tenant created successfully'));
      queryClient.invalidateQueries({ queryKey: ['tenants'] });
      selectTenant(newTenant.id);
      setShowCreateTenant(false);
      setNewTenantName('');
      setNewTenantDescription('');
      setTenantDefaultLLM(undefined);
      setTenantDefaultEmbedding(undefined);
      setTenantDefaultVisionLLM(undefined);
      // Pre-fill workspace form with new tenant defaults, then open the dialog
      if (newTenant.default_llm_model) {
        setWorkspaceLLMSelection({
          model: newTenant.default_llm_model,
          provider: newTenant.default_llm_provider || '',
          fullId: newTenant.default_llm_provider
            ? `${newTenant.default_llm_provider}/${newTenant.default_llm_model}`
            : newTenant.default_llm_model,
        });
      }
      if (newTenant.default_embedding_model) {
        setEmbeddingSelection({
          model: newTenant.default_embedding_model,
          provider: newTenant.default_embedding_provider || '',
          dimension: newTenant.default_embedding_dimension ?? 1536,
        });
      }
      if (newTenant.default_vision_llm_model) {
        setWorkspaceVisionLLMSelection({
          model: newTenant.default_vision_llm_model,
          provider: newTenant.default_vision_llm_provider || '',
          fullId: newTenant.default_vision_llm_provider
            ? `${newTenant.default_vision_llm_provider}/${newTenant.default_vision_llm_model}`
            : newTenant.default_vision_llm_model,
        });
      }
      setShowCreateWorkspace(true);
    },
    onError: (error) => {
      toast.error(t('tenant.createFailed', 'Failed to create tenant'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  // Create workspace mutation
  // SPEC-032/SPEC-041: Updated to include LLM, embedding, and vision configuration
  const createWorkspaceMutation = useMutation({
    mutationFn: (data: {
      name: string;
      description?: string;
      slug?: string;
      llm_model?: string;
      llm_provider?: string;
      embedding_model?: string;
      embedding_provider?: string;
      embedding_dimension?: number;
      vision_llm_model?: string;
      vision_llm_provider?: string;
    }) =>
      selectedTenantId
        ? createWorkspace(selectedTenantId, data)
        : Promise.reject(new Error('No tenant selected')),
    onSuccess: (newWorkspace) => {
      toast.success(t('workspace.createSuccess', 'Workspace created successfully'));
      queryClient.invalidateQueries({ queryKey: ['workspaces', selectedTenantId] });
      selectWorkspace(newWorkspace.id);
      setShowCreateWorkspace(false);
      setNewWorkspaceName('');
      setNewWorkspaceDescription('');
      setNewWorkspaceSlug('');
      setWorkspaceLLMSelection(undefined); // Reset LLM selection
      setEmbeddingSelection(undefined); // Reset embedding selection
      setWorkspaceVisionLLMSelection(undefined); // Reset vision LLM selection
    },
    onError: (error) => {
      toast.error(t('workspace.createFailed', 'Failed to create workspace'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const handleTenantSelect = useCallback((tenantId: string) => {
    if (tenantId === selectedTenantId) return;
    selectTenant(tenantId);
    const tenant = tenants.find((te) => te.id === tenantId);
    if (tenant) {
      toast.info(t('tenant.switched', `Switched to tenant "{{name}}"`, { name: tenant.name }), {
        id: 'tenant-switch',
        duration: 2000,
      });
    }
  }, [selectTenant, selectedTenantId, tenants, t]);

  const handleWorkspaceSelect = useCallback((workspaceId: string) => {
    if (workspaceId === selectedWorkspaceId) return;
    selectWorkspace(workspaceId);
    const workspace = workspaces.find((w) => w.id === workspaceId);
    if (workspace) {
      toast.info(t('workspace.switched', `Switched to workspace "{{name}}"`, { name: workspace.name }), {
        id: 'workspace-switch',
        duration: 2000,
      });
    }
  }, [selectWorkspace, selectedWorkspaceId, workspaces, t]);

  /**
   * Pre-fill workspace creation form from a tenant's default model settings,
   * then open the dialog. Accepts an optional tenant override for the case
   * where the store hasn't been updated yet (e.g. immediately after tenant creation).
   */
  const handleOpenCreateWorkspace = useCallback((tenantOverride?: typeof tenants[0]) => {
    const tenant = tenantOverride ?? tenants.find((te) => te.id === selectedTenantId);
    if (tenant) {
      if (tenant.default_llm_model) {
        setWorkspaceLLMSelection({
          model: tenant.default_llm_model,
          provider: tenant.default_llm_provider || '',
          fullId: tenant.default_llm_provider
            ? `${tenant.default_llm_provider}/${tenant.default_llm_model}`
            : tenant.default_llm_model,
        });
      }
      if (tenant.default_embedding_model) {
        setEmbeddingSelection({
          model: tenant.default_embedding_model,
          provider: tenant.default_embedding_provider || '',
          dimension: tenant.default_embedding_dimension ?? 1536,
        });
      }
      if (tenant.default_vision_llm_model) {
        setWorkspaceVisionLLMSelection({
          model: tenant.default_vision_llm_model,
          provider: tenant.default_vision_llm_provider || '',
          fullId: tenant.default_vision_llm_provider
            ? `${tenant.default_vision_llm_provider}/${tenant.default_vision_llm_model}`
            : tenant.default_vision_llm_model,
        });
      }
    }
    setShowCreateWorkspace(true);
  }, [selectedTenantId, tenants]);

  const selectedTenant = tenants.find((t) => t.id === selectedTenantId);
  const selectedWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);
  const isLoading = isLoadingTenants || isLoadingWorkspaces;

  // WHY: Display full workspace name for better identification
  // Previous: 16 chars was too aggressive, users couldn't identify workspaces
  // Now: 30 chars with wider max-width (200px) for better visibility
  // Tooltip still shows full name on hover for very long names
  // @implements FEAT0861 - Display tenant+workspace context to prevent confusion
  // @implements BR0506 - Workspace name must be identifiable
  // WHY: Multiple tenants can have identically-named workspaces.
  // Show "Tenant / Workspace" format to make context unambiguous.
  const displayName = (() => {
    if (selectedWorkspace && selectedTenant) {
      const tenantPart = selectedTenant.name.length > 15 
        ? selectedTenant.name.slice(0, 15) + '...' 
        : selectedTenant.name;
      const workspacePart = selectedWorkspace.name.length > 20 
        ? selectedWorkspace.name.slice(0, 20) + '...' 
        : selectedWorkspace.name;
      return `${tenantPart} / ${workspacePart}`;
    }
    return selectedTenant?.name || t('tenant.selectContext', 'Select workspace');
  })();
  const truncatedName = displayName.length > 40 ? displayName.slice(0, 40) + '...' : displayName;

  return (
    <>
      <TooltipProvider delayDuration={300}>
        <Tooltip delayDuration={300}>
          <TooltipTrigger asChild>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button 
                  data-testid="workspace-selector"
                  variant="ghost" 
                  size="sm"
                  className={cn(
                    "h-8 gap-1.5 px-2.5 font-medium text-sm",
                    "bg-muted/50 hover:bg-muted border border-border/50",
                    "transition-all duration-150",
                    className
                  )}
                >
                  {isLoading ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <FolderKanban className="h-3.5 w-3.5 text-muted-foreground" />
                  )}
                  <span className="max-w-[200px] truncate hidden sm:inline">
                    {truncatedName}
                  </span>
                  <ChevronDown className="h-3 w-3 text-muted-foreground" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-72">
                {/* Current Context */}
                {selectedTenant && selectedWorkspace && (
                  <>
                    <DropdownMenuLabel className="pb-2">
                      <div className="flex items-center gap-2">
                        <div className="h-8 w-8 rounded-lg bg-primary/10 flex items-center justify-center">
                          <FolderKanban className="h-4 w-4 text-primary" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-semibold truncate">{selectedWorkspace.name}</p>
                          <p className="text-xs text-muted-foreground truncate">{selectedTenant.name}</p>
                        </div>
                      </div>
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                  </>
                )}
                
                {/* Tenant Selection */}
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="text-xs text-muted-foreground font-semibold uppercase tracking-wide">
                    {t('tenant.tenant', 'Tenant')}
                  </DropdownMenuLabel>
                  {tenants.length === 0 ? (
                    <DropdownMenuItem disabled className="text-xs text-muted-foreground">
                      {isLoadingTenants ? 'Loading...' : 'No tenants found'}
                    </DropdownMenuItem>
                  ) : (
                    tenants.map((tenant) => (
                      <DropdownMenuItem
                        key={tenant.id}
                        onClick={() => handleTenantSelect(tenant.id)}
                        className="py-2"
                      >
                        <Building2 className="mr-2 h-4 w-4 text-muted-foreground" />
                        <span className="flex-1 truncate">{tenant.name}</span>
                        {tenant.id === selectedTenantId && (
                          <Check className="ml-2 h-4 w-4 text-primary" />
                        )}
                      </DropdownMenuItem>
                    ))
                  )}
                  <DropdownMenuItem onClick={() => setShowCreateTenant(true)} className="py-2">
                    <Plus className="mr-2 h-4 w-4 text-muted-foreground" />
                    <span>{t('tenant.createNew', 'Create New Tenant')}</span>
                  </DropdownMenuItem>
                </DropdownMenuGroup>

                {/* Workspace Selection */}
                {selectedTenantId && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuGroup>
                      <DropdownMenuLabel className="text-xs text-muted-foreground font-semibold uppercase tracking-wide">
                        {t('workspace.workspace', 'Workspace')}
                      </DropdownMenuLabel>
                      {workspaces.length === 0 ? (
                        <DropdownMenuItem disabled className="text-xs text-muted-foreground">
                          {isLoadingWorkspaces ? 'Loading...' : 'No workspaces found'}
                        </DropdownMenuItem>
                      ) : (
                        workspaces.map((workspace) => (
                          <DropdownMenuItem
                            key={workspace.id}
                            onClick={() => handleWorkspaceSelect(workspace.id)}
                            className="py-2"
                          >
                            <FolderKanban className="mr-2 h-4 w-4 text-muted-foreground" />
                            <div className="flex-1 min-w-0">
                              <div className="truncate font-medium">{workspace.name}</div>
                              {selectedTenant && (
                                <div className="text-[10px] text-muted-foreground truncate mt-0.5">
                                  {selectedTenant.name}
                                  {workspace.document_count !== undefined && ` • ${workspace.document_count} docs`}
                                </div>
                              )}
                            </div>
                            {workspace.id === selectedWorkspaceId && (
                              <Check className="ml-2 h-4 w-4 text-primary" />
                            )}
                          </DropdownMenuItem>
                        ))
                      )}
                      <DropdownMenuItem onClick={() => handleOpenCreateWorkspace()} className="py-2">
                        <Plus className="mr-2 h-4 w-4 text-muted-foreground" />
                        <span>{t('workspace.createNew', 'Create New Workspace')}</span>
                      </DropdownMenuItem>
                    </DropdownMenuGroup>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </TooltipTrigger>
          <TooltipContent side="bottom" sideOffset={8}>
            {selectedTenant && selectedWorkspace ? (
              <div className="text-xs">
                <p className="font-medium">{selectedWorkspace.name}</p>
                <p className="text-muted-foreground">{selectedTenant.name}</p>
              </div>
            ) : (
              <p>{t('tenant.selectContext', 'Select workspace')}</p>
            )}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      {/* Create Tenant Dialog */}
      <Dialog open={showCreateTenant} onOpenChange={setShowCreateTenant}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Building2 className="h-5 w-5 text-primary" />
              {t('tenant.createNew', 'Create New Tenant')}
            </DialogTitle>
            <DialogDescription>
              {t('tenant.createDescription', 'Create a new tenant to organize your workspaces and documents.')}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="tenant-name">{t('common.name', 'Name')}</Label>
              <Input
                id="tenant-name"
                value={newTenantName}
                onChange={(e) => setNewTenantName(e.target.value)}
                placeholder={t('tenant.namePlaceholder', 'My Organization')}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="tenant-description">{t('common.description', 'Description')}</Label>
              <Input
                id="tenant-description"
                value={newTenantDescription}
                onChange={(e) => setNewTenantDescription(e.target.value)}
                placeholder={t('tenant.descriptionPlaceholder', 'Optional description')}
              />
            </div>
            {/* SPEC-032: Default LLM model selection for tenant */}
            <div className="grid gap-2">
              <Label>
                {t('tenant.defaultLLM', 'Default LLM Model')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <LLMModelSelector
                value={tenantDefaultLLM}
                onChange={setTenantDefaultLLM}
              />
              <p className="text-xs text-muted-foreground">
                {t('tenant.defaultLLMHint', 'Default LLM for new workspaces. Can be overridden per workspace.')}
              </p>
            </div>
            {/* SPEC-032: Default embedding model selection for tenant */}
            <div className="grid gap-2">
              <Label>
                {t('tenant.defaultEmbedding', 'Default Embedding Model')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <EmbeddingModelSelector
                value={tenantDefaultEmbedding}
                onChange={setTenantDefaultEmbedding}
              />
              <p className="text-xs text-muted-foreground">
                {t('tenant.defaultEmbeddingHint', 'Default embedding for new workspaces. Can be overridden per workspace.')}
              </p>
            </div>
            {/* SPEC-041: Default Vision LLM selection for tenant */}
            <div className="grid gap-2">
              <Label>
                {t('tenant.defaultVisionLLM', 'Default Vision LLM')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <LLMModelSelector
                value={tenantDefaultVisionLLM}
                onChange={setTenantDefaultVisionLLM}
                filterVision
                showUsageHint={false}
              />
              <p className="text-xs text-muted-foreground">
                {t('tenant.defaultVisionLLMHint', 'Default vision model for PDF extraction. Can be overridden per workspace.')}
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateTenant(false)}>
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() => createTenantMutation.mutate({ 
                name: newTenantName, 
                description: newTenantDescription || undefined,
                // SPEC-032: Include default LLM configuration
                default_llm_model: tenantDefaultLLM?.model,
                default_llm_provider: tenantDefaultLLM?.provider,
                // SPEC-032: Include default embedding configuration
                default_embedding_model: tenantDefaultEmbedding?.model,
                default_embedding_provider: tenantDefaultEmbedding?.provider,
                // SPEC-041: Include default vision LLM configuration
                default_vision_llm_model: tenantDefaultVisionLLM?.model,
                default_vision_llm_provider: tenantDefaultVisionLLM?.provider,
              })}
              disabled={!newTenantName.trim() || !tenantDefaultLLM || !tenantDefaultEmbedding || !tenantDefaultVisionLLM || createTenantMutation.isPending}
            >
              {createTenantMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('common.creating', 'Creating...')}
                </>
              ) : (
                t('common.create', 'Create')
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Workspace Dialog */}
      <Dialog open={showCreateWorkspace} onOpenChange={setShowCreateWorkspace}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <FolderKanban className="h-5 w-5 text-primary" />
              {t('workspace.createNew', 'Create New Workspace')}
            </DialogTitle>
            <DialogDescription>
              {t('workspace.createDescription', 'Create a new workspace within the current tenant.')}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4 max-h-[60vh] overflow-y-auto">
            <div className="grid gap-2">
              <Label htmlFor="workspace-name">{t('common.name', 'Name')}</Label>
              <Input
                id="workspace-name"
                value={newWorkspaceName}
                onChange={(e) => {
                  setNewWorkspaceName(e.target.value);
                  // Auto-generate slug from name if user hasn't manually edited it
                  if (!newWorkspaceSlug || newWorkspaceSlug === generateSlug(newWorkspaceName)) {
                    setNewWorkspaceSlug(generateSlug(e.target.value));
                  }
                }}
                placeholder={t('workspace.namePlaceholder', 'My Project')}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="workspace-slug">
                {t('workspace.slug', 'URL Slug')}
                <span className="text-muted-foreground text-xs ml-2">
                  {t('workspace.slugHint', '(optional, auto-generated)')}
                </span>
              </Label>
              <Input
                id="workspace-slug"
                value={newWorkspaceSlug}
                onChange={(e) => setNewWorkspaceSlug(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, '-'))}
                placeholder="my-project"
                pattern="[a-z0-9-]+"
              />
              <p className="text-xs text-muted-foreground">
                {t('workspace.slugDescription', 'Used in URLs: /query?workspace={slug}')}
              </p>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="workspace-description">{t('common.description', 'Description')}</Label>
              <Input
                id="workspace-description"
                value={newWorkspaceDescription}
                onChange={(e) => setNewWorkspaceDescription(e.target.value)}
                placeholder={t('workspace.descriptionPlaceholder', 'Optional description')}
              />
            </div>
            {/* SPEC-032: LLM model selection for workspace */}
            <div className="grid gap-2">
              <Label>
                {t('workspace.llmModel', 'LLM Model')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <LLMModelSelector
                value={workspaceLLMSelection}
                onChange={setWorkspaceLLMSelection}
              />
              <p className="text-xs text-muted-foreground">
                {t('workspace.llmDescription', 'LLM for document ingestion and knowledge graph generation.')}
              </p>
            </div>
            {/* SPEC-032: Embedding model selection */}
            <div className="grid gap-2">
              <Label htmlFor="workspace-embedding">
                {t('workspace.embeddingModel', 'Embedding Model')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <EmbeddingModelSelector
                value={embeddingSelection}
                onChange={setEmbeddingSelection}
              />
              <p className="text-xs text-muted-foreground">
                {t('workspace.embeddingDescription', 'Embedding model determines how documents are indexed. Cannot be changed after creation.')}
              </p>
            </div>
            {/* SPEC-041: Vision LLM selection for workspace */}
            <div className="grid gap-2">
              <Label>
                {t('workspace.visionLLM', 'Vision LLM')}
                <span className="text-destructive ml-0.5">*</span>
              </Label>
              <LLMModelSelector
                value={workspaceVisionLLMSelection}
                onChange={setWorkspaceVisionLLMSelection}
                filterVision
                showUsageHint={false}
              />
              <p className="text-xs text-muted-foreground">
                {t('workspace.visionDescription', 'Vision model for PDF-to-Markdown image extraction. Overrides tenant default.')}
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateWorkspace(false)}>
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() => createWorkspaceMutation.mutate({ 
                name: newWorkspaceName, 
                description: newWorkspaceDescription || undefined,
                slug: newWorkspaceSlug.trim() || undefined,
                // SPEC-032: Include LLM configuration if selected
                llm_model: workspaceLLMSelection?.model,
                llm_provider: workspaceLLMSelection?.provider,
                // SPEC-032: Include embedding configuration if selected
                embedding_model: embeddingSelection?.model,
                embedding_provider: embeddingSelection?.provider,
                embedding_dimension: embeddingSelection?.dimension,
                // SPEC-041: Include vision LLM configuration if selected
                vision_llm_model: workspaceVisionLLMSelection?.model,
                vision_llm_provider: workspaceVisionLLMSelection?.provider,
              })}
              disabled={!newWorkspaceName.trim() || !workspaceLLMSelection || !embeddingSelection || !workspaceVisionLLMSelection || createWorkspaceMutation.isPending}
            >
              {createWorkspaceMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('common.creating', 'Creating...')}
                </>
              ) : (
                t('common.create', 'Create')
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default HeaderTenantSelector;
