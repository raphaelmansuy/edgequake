'use client';

import { ProviderStatusCard } from '@/components/settings/provider-status-card';
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
    AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { RebuildEmbeddingsButton } from '@/components/workspace/rebuild-embeddings-button';
import { useQueryStore } from '@/stores/use-query-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { Database, Download, Globe, Monitor, Moon, Palette, Sun, Upload } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useRef } from 'react';
import { toast } from 'sonner';

export default function SettingsPage() {
  const { theme, setTheme } = useTheme();
  const { 
    language, 
    graphSettings, 
    querySettings,
    ingestionSettings,
    setLanguage, 
    setGraphSettings,
    setQuerySettings,
    setIngestionSettings,
    resetSettings,
    exportSettings,
    importSettings,
  } = useSettingsStore();
  const { clearHistory } = useQueryStore();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleClearHistory = () => {
    clearHistory();
    toast.success('Query history cleared');
  };

  const handleResetSettings = () => {
    resetSettings();
    toast.success('Settings reset to defaults');
  };

  const handleExportSettings = () => {
    const json = exportSettings();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `edgequake-settings-${new Date().toISOString().split('T')[0]}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success('Settings exported successfully');
  };

  const handleImportSettings = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const result = importSettings(e.target?.result as string);
      if (result.success) {
        toast.success('Settings imported successfully');
      } else {
        toast.error(`Import failed: ${result.error}`);
      }
    };
    reader.onerror = () => {
      toast.error('Failed to read file');
    };
    reader.readAsText(file);
    
    // Reset file input so the same file can be imported again
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleThemeChange = (newTheme: string) => {
    setTheme(newTheme);
    toast.success(`Theme changed to ${newTheme}`);
  };

  const handleLanguageChange = (newLanguage: 'en' | 'zh' | 'ja' | 'ko') => {
    setLanguage(newLanguage);
    const languageNames = { en: 'English', zh: '中文', ja: '日本語', ko: '한국어' };
    toast.success(`Language changed to ${languageNames[newLanguage]}`);
  };

  const handleGraphSettingsChange = <K extends keyof typeof graphSettings>(
    key: K,
    value: typeof graphSettings[K]
  ) => {
    setGraphSettings({ [key]: value });
    toast.success('Graph settings updated');
  };

  const handleQuerySettingsChange = <K extends keyof typeof querySettings>(
    key: K,
    value: typeof querySettings[K]
  ) => {
    setQuerySettings({ [key]: value });
    toast.success('Query settings updated');
  };

  const handleIngestionSettingsChange = <K extends keyof typeof ingestionSettings>(
    key: K,
    value: typeof ingestionSettings[K]
  ) => {
    setIngestionSettings({ [key]: value });
    toast.success('Ingestion settings updated');
  };

  return (
    <ScrollArea className="h-full">
      <div className="p-6 md:p-8 max-w-4xl mx-auto space-y-8">
        {/* Header */}
        <header className="space-y-2">
          <h1 className="text-3xl font-bold tracking-tight">Settings</h1>
          <p className="text-base text-muted-foreground">
            Customize your EdgeQuake experience
          </p>
        </header>

      {/* Appearance */}
      <Card>
        <CardHeader className="pb-4">
          <CardTitle className="flex items-center gap-2">
            <Palette className="h-5 w-5 text-primary" />
            Appearance
          </CardTitle>
          <CardDescription>
            Customize the look and feel of the application
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Theme */}
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-1">
              <label className="text-sm font-medium">Theme</label>
              <p className="text-sm text-muted-foreground">
                Select your preferred color scheme
              </p>
            </div>
            <Select value={theme} onValueChange={handleThemeChange}>
              <SelectTrigger className="w-[150px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="light">
                  <div className="flex items-center gap-2">
                    <Sun className="h-4 w-4" />
                    Light
                  </div>
                </SelectItem>
                <SelectItem value="dark">
                  <div className="flex items-center gap-2">
                    <Moon className="h-4 w-4" />
                    Dark
                  </div>
                </SelectItem>
                <SelectItem value="system">
                  <div className="flex items-center gap-2">
                    <Monitor className="h-4 w-4" />
                    System
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Separator />

          {/* Language */}
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-1">
              <label className="text-sm font-medium">Language</label>
              <p className="text-sm text-muted-foreground">
                Select your preferred language
              </p>
            </div>
            <Select value={language} onValueChange={handleLanguageChange}>
              <SelectTrigger className="w-[150px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="en">English</SelectItem>
                <SelectItem value="zh">中文</SelectItem>
                <SelectItem value="ja">日本語</SelectItem>
                <SelectItem value="ko">한국어</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      {/* Provider Status */}
      <ProviderStatusCard />

      {/* Workspace Maintenance (SPEC-032) */}
      <RebuildEmbeddingsButton variant="card" />

      {/* Graph Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Graph Visualization
          </CardTitle>
          <CardDescription>
            Configure how the knowledge graph is displayed
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Show Labels */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Show Node Labels</label>
              <p className="text-xs text-muted-foreground">
                Display labels on graph nodes
              </p>
            </div>
            <Switch
              checked={graphSettings.showLabels}
              onCheckedChange={(showLabels) => handleGraphSettingsChange('showLabels', showLabels)}
            />
          </div>

          <Separator />

          {/* Show Edge Labels */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Show Edge Labels</label>
              <p className="text-xs text-muted-foreground">
                Display relationship types on edges
              </p>
            </div>
            <Switch
              checked={graphSettings.showEdgeLabels}
              onCheckedChange={(showEdgeLabels) => handleGraphSettingsChange('showEdgeLabels', showEdgeLabels)}
            />
          </div>

          <Separator />

          {/* Node Size */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Node Size</label>
              <p className="text-xs text-muted-foreground">
                Size of nodes in the graph
              </p>
            </div>
            <Select
              value={graphSettings.nodeSize}
              onValueChange={(nodeSize: 'small' | 'medium' | 'large') => handleGraphSettingsChange('nodeSize', nodeSize)}
            >
              <SelectTrigger className="w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="small">Small</SelectItem>
                <SelectItem value="medium">Medium</SelectItem>
                <SelectItem value="large">Large</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Separator />

          {/* Layout */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Default Layout</label>
              <p className="text-xs text-muted-foreground">
                Initial graph layout algorithm
              </p>
            </div>
            <Select
              value={graphSettings.layout}
              onValueChange={(layout: 'force' | 'circular' | 'random' | 'circlepack' | 'noverlaps' | 'force-directed' | 'hierarchical') => handleGraphSettingsChange('layout', layout)}
            >
              <SelectTrigger className="w-[150px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="force">ForceAtlas2 (FA2)</SelectItem>
                <SelectItem value="force-directed">Force-Directed</SelectItem>
                <SelectItem value="circular">Circular</SelectItem>
                <SelectItem value="circlepack">Circle Pack</SelectItem>
                <SelectItem value="random">Random</SelectItem>
                <SelectItem value="noverlaps">Noverlaps</SelectItem>
                <SelectItem value="hierarchical">Hierarchical</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      {/* Query Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Database className="h-5 w-5" />
            Query Defaults
          </CardTitle>
          <CardDescription>
            Default settings for knowledge graph queries
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Default Mode */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Default Query Mode</label>
              <p className="text-xs text-muted-foreground">
                Default retrieval mode for queries
              </p>
            </div>
            <Select
              value={querySettings.mode}
              onValueChange={(mode: 'local' | 'global' | 'hybrid' | 'naive') => 
                handleQuerySettingsChange('mode', mode)
              }
            >
              <SelectTrigger className="w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="local">Local</SelectItem>
                <SelectItem value="global">Global</SelectItem>
                <SelectItem value="hybrid">Hybrid</SelectItem>
                <SelectItem value="naive">Naive</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Separator />

          {/* Streaming */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Enable Streaming</label>
              <p className="text-xs text-muted-foreground">
                Show responses as they are generated
              </p>
            </div>
            <Switch
              checked={querySettings.stream}
              onCheckedChange={(stream) => handleQuerySettingsChange('stream', stream)}
            />
          </div>

          <Separator />

          {/* Reranking - SOTA Feature */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Enable Reranking</label>
              <p className="text-xs text-muted-foreground">
                Improve retrieval precision with semantic reranking
              </p>
            </div>
            <Switch
              checked={querySettings.enableRerank}
              onCheckedChange={(enableRerank) => handleQuerySettingsChange('enableRerank', enableRerank)}
            />
          </div>

          {/* Rerank Top K */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Rerank Top K</label>
              <p className="text-xs text-muted-foreground">
                Number of top results after reranking
              </p>
            </div>
            <Select
              value={String(querySettings.rerankTopK)}
              onValueChange={(value) => handleQuerySettingsChange('rerankTopK', Number(value))}
            >
              <SelectTrigger className="w-[80px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="5">5</SelectItem>
                <SelectItem value="10">10</SelectItem>
                <SelectItem value="15">15</SelectItem>
                <SelectItem value="20">20</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      {/* Ingestion Settings - SOTA Features */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Database className="h-5 w-5" />
            Ingestion Settings
          </CardTitle>
          <CardDescription>
            Advanced settings for document ingestion quality
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Gleaning */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Enable Gleaning</label>
              <p className="text-xs text-muted-foreground">
                Multiple extraction passes for higher quality entities
              </p>
            </div>
            <Switch
              checked={ingestionSettings.enableGleaning}
              onCheckedChange={(enableGleaning) => handleIngestionSettingsChange('enableGleaning', enableGleaning)}
            />
          </div>

          <Separator />

          {/* Max Gleaning Passes */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Max Gleaning Passes</label>
              <p className="text-xs text-muted-foreground">
                Maximum number of extraction passes (1-3)
              </p>
            </div>
            <Select
              value={String(ingestionSettings.maxGleaning)}
              onValueChange={(value) => handleIngestionSettingsChange('maxGleaning', Number(value))}
            >
              <SelectTrigger className="w-[80px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="1">1</SelectItem>
                <SelectItem value="2">2</SelectItem>
                <SelectItem value="3">3</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Separator />

          {/* LLM Summarization */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">LLM Summarization</label>
              <p className="text-xs text-muted-foreground">
                Use LLM to merge entity descriptions intelligently
              </p>
            </div>
            <Switch
              checked={ingestionSettings.useLLMSummarization}
              onCheckedChange={(useLLMSummarization) => handleIngestionSettingsChange('useLLMSummarization', useLLMSummarization)}
            />
          </div>
        </CardContent>
      </Card>

      {/* Data Management - Dangerous Actions Section */}
      <Card className="border-destructive/30">
        <CardHeader className="pb-4">
          <CardTitle className="flex items-center gap-2 text-destructive">
            <Database className="h-5 w-5" />
            Data Management
          </CardTitle>
          <CardDescription>
            Manage local data, import/export settings, and reset. Use caution with destructive actions.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Import/Export Settings */}
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-1">
              <label className="text-sm font-medium">Settings Backup</label>
              <p className="text-sm text-muted-foreground">
                Export or import your settings as JSON
              </p>
            </div>
            <div className="flex items-center gap-3">
              <Button variant="outline" size="sm" onClick={handleExportSettings}>
                <Download className="h-4 w-4 mr-2" />
                Export
              </Button>
              <Button variant="outline" size="sm" asChild>
                <label className="cursor-pointer">
                  <Upload className="h-4 w-4 mr-2" />
                  Import
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept=".json"
                    className="hidden"
                    onChange={handleImportSettings}
                  />
                </label>
              </Button>
            </div>
          </div>

          <Separator />

          {/* Clear History */}
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-1">
              <label className="text-sm font-medium">Query History</label>
              <p className="text-sm text-muted-foreground">
                Clear all saved query history and conversations
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="outline" size="sm" className="border-destructive/50 text-destructive hover:bg-destructive/10">
                  Clear History
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Clear query history?</AlertDialogTitle>
                  <AlertDialogDescription>
                    This will permanently delete all your saved queries and favorites. This action cannot be undone.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={handleClearHistory} className="bg-destructive hover:bg-destructive/90">
                    Clear
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>

          <Separator />

          {/* Reset Settings */}
          <div className="flex items-center justify-between gap-4 p-4 rounded-lg bg-destructive/5 border border-destructive/20">
            <div className="space-y-1">
              <label className="text-sm font-medium text-destructive">Reset All Settings</label>
              <p className="text-sm text-muted-foreground">
                Reset all settings to their default values. Your data will not be affected.
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" size="sm">
                  Reset Settings
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Reset all settings?</AlertDialogTitle>
                  <AlertDialogDescription>
                    This will reset all settings to their default values. Your documents and knowledge graph data will not be affected. This action cannot be undone.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={handleResetSettings} className="bg-destructive hover:bg-destructive/90">
                    Reset
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
      </div>
    </ScrollArea>
  );
}
