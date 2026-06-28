'use client';

import { EntityTypeSelector } from '@/components/shared/entity-type-selector';
import { Button } from '@/components/ui/button';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
} from '@/components/ui/command';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import {
    EmbeddingModelSelector,
    type EmbeddingSelection,
} from '@/components/workspace/embedding-model-selector';
import {
    LLMModelSelector,
    type LLMSelection,
} from '@/components/workspace/llm-model-selector';
import { WorkspaceCreateModelSection } from '@/components/workspace/workspace-create-model-section';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
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

  // Selector popover state
  const [selectorOpen, setSelectorOpen] = useState(false);

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
  // SPEC-085: Custom entity types for new workspace
  const [workspaceEntityTypes, setWorkspaceEntityTypes] = useState<string[]>([...ENTITY_PRESETS.general.types]);
  const [showEntityTypeConfig, setShowEntityTypeConfig] = useState(false);
  const [useServerModelDefaults, setUseServerModelDefaults] = useState(false);


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
      entity_types?: string[];
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
      setWorkspaceEntityTypes([...ENTITY_PRESETS.general.types]); // SPEC-085: Reset entity types
      setShowEntityTypeConfig(false);
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

  // WHY: CSS truncation handles overflow cleanly; JS slicing produced unpredictable
  // ellipsis positions at different viewport sizes (audit F-WS-03).
  // @implements FEAT0861 - Display tenant+workspace context to prevent confusion
  const displayName = selectedWorkspace && selectedTenant
    ? `${selectedTenant.name} / ${selectedWorkspace.name}`
    : selectedTenant?.name || t('tenant.selectContext', 'Select workspace');

  return (
    <>
      <Popover open={selectorOpen} onOpenChange={setSelectorOpen}>
        <PopoverTrigger asChild>
          <Button
            data-testid="workspace-selector"
            variant="ghost"
            size="sm"
            role="combobox"
            aria-expanded={selectorOpen}
            aria-label={`${t('workspace.select', 'Select workspace')}: ${displayName}`}
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
              <FolderKanban className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
            )}
            <span className="max-w-50 truncate hidden sm:inline">
              {displayName}
            </span>
            <ChevronDown className="h-3 w-3 text-muted-foreground" aria-hidden="true" />
          </Button>
        </PopoverTrigger>

        <PopoverContent align="start" className="w-72 p-0" sideOffset={6}>
          <Command>
            <CommandInput
              placeholder={t('workspace.searchPlaceholder', 'Search workspaces...')}
              className="h-9"
            />
            <CommandList>
              <CommandEmpty>
                {t('workspace.noResults', 'No workspaces found.')}
              </CommandEmpty>

              {/* Organizations / Tenants */}
              <CommandGroup heading={t('tenant.tenant', 'Organizations')}>
                {tenants.map((tenant) => (
                  <CommandItem
                    key={tenant.id}
                    value={`org:${tenant.name}`}
                    onSelect={() => {
                      handleTenantSelect(tenant.id);
                      setSelectorOpen(false);
                    }}
                  >
                    <Building2 className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
                    <span className="flex-1 truncate">{tenant.name}</span>
                    {tenant.id === selectedTenantId && (
                      <Check className="ml-2 h-4 w-4 text-primary" />
                    )}
                  </CommandItem>
                ))}
                <CommandItem
                  value="create-organization"
                  onSelect={() => {
                    setSelectorOpen(false);
                    setShowCreateTenant(true);
                  }}
                >
                  <Plus className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
                  {t('tenant.createNew', 'New Organization')}
                </CommandItem>
              </CommandGroup>

              {/* Workspaces scoped to selected tenant */}
              {selectedTenantId && (
                <>
                  <CommandSeparator />
                  <CommandGroup
                    heading={
                      selectedTenant
                        ? `${t('workspace.workspace', 'Workspaces')} — ${selectedTenant.name}`
                        : t('workspace.workspace', 'Workspaces')
                    }
                  >
                    {workspaces.length === 0 && !isLoadingWorkspaces && (
                      <div className="py-2 px-4 text-xs text-muted-foreground">
                        {t('workspace.empty', 'No workspaces yet')}
                      </div>
                    )}
                    {workspaces.map((workspace) => (
                      <CommandItem
                        key={workspace.id}
                        value={`ws:${workspace.name} ${workspace.slug ?? ''}`}
                        onSelect={() => {
                          handleWorkspaceSelect(workspace.id);
                          setSelectorOpen(false);
                        }}
                      >
                        <FolderKanban className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
                        <div className="flex-1 min-w-0">
                          <div className="truncate text-sm">{workspace.name}</div>
                          {workspace.document_count !== undefined && (
                            <div className="text-[10px] text-muted-foreground">
                              {workspace.document_count} docs
                            </div>
                          )}
                        </div>
                        {workspace.id === selectedWorkspaceId && (
                          <Check className="ml-2 h-4 w-4 text-primary" />
                        )}
                      </CommandItem>
                    ))}
                    <CommandItem
                      value="create-workspace"
                      onSelect={() => {
                        setSelectorOpen(false);
                        handleOpenCreateWorkspace();
                      }}
                    >
                      <Plus className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
                      {t('workspace.createNew', 'New Workspace')}
                    </CommandItem>
                  </CommandGroup>
                </>
              )}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      {/* Create Tenant Dialog */}
      <Dialog open={showCreateTenant} onOpenChange={setShowCreateTenant}>
        <DialogContent className="w-[95vw] sm:max-w-190 max-h-[92vh] overflow-hidden grid-rows-[auto_minmax(0,1fr)_auto]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Building2 className="h-5 w-5 text-primary" />
              {t('tenant.createNew', 'Create New Tenant')}
            </DialogTitle>
            <DialogDescription>
              {t('tenant.createDescription', 'Create a new tenant to organize your workspaces and documents.')}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-3 overflow-y-auto pr-1">
            <div className="rounded-lg border p-3 space-y-3">
              <div className="grid gap-3 sm:grid-cols-2">
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
              </div>
            </div>

            <div className="rounded-lg border p-3 space-y-3">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                {t('tenant.defaultsSection', 'Default Models')}
              </p>
              <div className="grid gap-3 sm:grid-cols-2">
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
                <div className="grid gap-2 sm:col-span-2">
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
        <DialogContent className="w-[95vw] sm:max-w-190 max-h-[92vh] overflow-hidden grid-rows-[auto_minmax(0,1fr)_auto]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <FolderKanban className="h-5 w-5 text-primary" />
              {t('workspace.createNew', 'Create New Workspace')}
            </DialogTitle>
            <DialogDescription>
              {t('workspace.createDescription', 'Create a new workspace within the current tenant.')}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 py-3 overflow-y-auto pr-1">
            <div className="rounded-lg border p-3 space-y-3">
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="grid gap-2">
                  <Label htmlFor="workspace-name">{t('common.name', 'Name')}</Label>
                  <Input
                    id="workspace-name"
                    value={newWorkspaceName}
                    onChange={(e) => {
                      setNewWorkspaceName(e.target.value);
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
                </div>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="workspace-description">{t('common.description', 'Description')}</Label>
                <Input
                  id="workspace-description"
                  value={newWorkspaceDescription}
                  onChange={(e) => setNewWorkspaceDescription(e.target.value)}
                  placeholder={t('workspace.descriptionPlaceholder', 'Optional description')}
                />
                <p className="text-xs text-muted-foreground">
                  {t('workspace.slugDescription', 'Used in URLs: /query?workspace={slug}')}
                </p>
              </div>
            </div>

            <WorkspaceCreateModelSection
              llm={workspaceLLMSelection}
              embedding={embeddingSelection}
              vision={workspaceVisionLLMSelection}
              onLlmChange={setWorkspaceLLMSelection}
              onEmbeddingChange={setEmbeddingSelection}
              onVisionChange={setWorkspaceVisionLLMSelection}
              onUseServerDefaultsChange={setUseServerModelDefaults}
            />

            <div className="rounded-lg border p-3">
              <Collapsible open={showEntityTypeConfig} onOpenChange={setShowEntityTypeConfig}>
                <CollapsibleTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    className="w-full justify-between px-0 h-auto text-left"
                  >
                    <div>
                      <p className="text-sm font-medium">{t('entityTypes.title', 'Entity Types')}</p>
                      <p className="text-xs text-muted-foreground">
                        {workspaceEntityTypes.length} configured
                      </p>
                    </div>
                    <ChevronDown className={cn('h-4 w-4 transition-transform', showEntityTypeConfig && 'rotate-180')} />
                  </Button>
                </CollapsibleTrigger>
                <CollapsibleContent className="pt-3">
                  <p className="text-xs text-muted-foreground mb-2">
                    {t('entityTypes.description', 'Types of entities to extract from documents in this workspace.')}
                  </p>
                  <EntityTypeSelector
                    value={workspaceEntityTypes}
                    onChange={setWorkspaceEntityTypes}
                  />
                </CollapsibleContent>
              </Collapsible>
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
                ...(useServerModelDefaults
                  ? {}
                  : {
                      llm_model: workspaceLLMSelection?.model,
                      llm_provider: workspaceLLMSelection?.provider,
                      embedding_model: embeddingSelection?.model,
                      embedding_provider: embeddingSelection?.provider,
                      embedding_dimension: embeddingSelection?.dimension,
                      vision_llm_model: workspaceVisionLLMSelection?.model,
                      vision_llm_provider: workspaceVisionLLMSelection?.provider,
                    }),
                entity_types: workspaceEntityTypes.length > 0 ? workspaceEntityTypes : undefined,
              })}
              disabled={
                !newWorkspaceName.trim() ||
                createWorkspaceMutation.isPending ||
                (!useServerModelDefaults &&
                  (!workspaceLLMSelection || !embeddingSelection || !workspaceVisionLLMSelection))
              }
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
