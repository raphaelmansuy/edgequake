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
  const createTenantMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) => createTenant(data),
    onSuccess: (newTenant) => {
      toast.success(t('tenant.createSuccess', 'Tenant created successfully'));
      queryClient.invalidateQueries({ queryKey: ['tenants'] });
      selectTenant(newTenant.id);
      setShowCreateTenant(false);
      setNewTenantName('');
      setNewTenantDescription('');
    },
    onError: (error) => {
      toast.error(t('tenant.createFailed', 'Failed to create tenant'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  // Create workspace mutation
  const createWorkspaceMutation = useMutation({
    mutationFn: (data: { name: string; description?: string }) =>
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
    },
    onError: (error) => {
      toast.error(t('workspace.createFailed', 'Failed to create workspace'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const handleTenantSelect = useCallback((tenantId: string) => {
    selectTenant(tenantId);
  }, [selectTenant]);

  const handleWorkspaceSelect = useCallback((workspaceId: string) => {
    selectWorkspace(workspaceId);
  }, [selectWorkspace]);

  const selectedTenant = tenants.find((t) => t.id === selectedTenantId);
  const selectedWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);
  const isLoading = isLoadingTenants || isLoadingWorkspaces;

  const displayName = selectedWorkspace?.name || selectedTenant?.name || t('tenant.selectContext', 'Select workspace');
  const truncatedName = displayName.length > 16 ? displayName.slice(0, 16) + '...' : displayName;

  return (
    <>
      <TooltipProvider delayDuration={300}>
        <Tooltip>
          <TooltipTrigger asChild>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button 
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
                  <span className="max-w-[120px] truncate hidden sm:inline">
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
                            <span className="flex-1 truncate">{workspace.name}</span>
                            {workspace.document_count !== undefined && (
                              <span className="text-[10px] text-muted-foreground ml-1">
                                {workspace.document_count} docs
                              </span>
                            )}
                            {workspace.id === selectedWorkspaceId && (
                              <Check className="ml-2 h-4 w-4 text-primary" />
                            )}
                          </DropdownMenuItem>
                        ))
                      )}
                      <DropdownMenuItem onClick={() => setShowCreateWorkspace(true)} className="py-2">
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
        <DialogContent className="sm:max-w-[425px]">
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
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateTenant(false)}>
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() => createTenantMutation.mutate({ name: newTenantName, description: newTenantDescription || undefined })}
              disabled={!newTenantName.trim() || createTenantMutation.isPending}
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
        <DialogContent className="sm:max-w-[425px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <FolderKanban className="h-5 w-5 text-primary" />
              {t('workspace.createNew', 'Create New Workspace')}
            </DialogTitle>
            <DialogDescription>
              {t('workspace.createDescription', 'Create a new workspace within the current tenant.')}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="workspace-name">{t('common.name', 'Name')}</Label>
              <Input
                id="workspace-name"
                value={newWorkspaceName}
                onChange={(e) => setNewWorkspaceName(e.target.value)}
                placeholder={t('workspace.namePlaceholder', 'My Project')}
              />
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
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateWorkspace(false)}>
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              onClick={() => createWorkspaceMutation.mutate({ name: newWorkspaceName, description: newWorkspaceDescription || undefined })}
              disabled={!newWorkspaceName.trim() || createWorkspaceMutation.isPending}
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
