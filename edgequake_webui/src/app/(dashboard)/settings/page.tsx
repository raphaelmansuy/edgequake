'use client';

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
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { useQueryStore } from '@/stores/use-query-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { Database, Globe, Monitor, Moon, Palette, Sun, Trash2 } from 'lucide-react';
import { useTheme } from 'next-themes';
import { toast } from 'sonner';

export default function SettingsPage() {
  const { theme, setTheme } = useTheme();
  const { 
    language, 
    graphSettings, 
    querySettings,
    setLanguage, 
    setGraphSettings,
    setQuerySettings,
    resetSettings 
  } = useSettingsStore();
  const { clearHistory } = useQueryStore();

  const handleClearHistory = () => {
    clearHistory();
    toast.success('Query history cleared');
  };

  const handleResetSettings = () => {
    resetSettings();
    toast.success('Settings reset to defaults');
  };

  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Settings</h1>
        <p className="text-muted-foreground">
          Customize your EdgeQuake experience
        </p>
      </div>

      {/* Appearance */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Palette className="h-5 w-5" />
            Appearance
          </CardTitle>
          <CardDescription>
            Customize the look and feel of the application
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Theme */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Theme</label>
              <p className="text-xs text-muted-foreground">
                Select your preferred color scheme
              </p>
            </div>
            <Select value={theme} onValueChange={setTheme}>
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
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Language</label>
              <p className="text-xs text-muted-foreground">
                Select your preferred language
              </p>
            </div>
            <Select value={language} onValueChange={(v: 'en' | 'zh' | 'ja' | 'ko') => setLanguage(v)}>
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
              onCheckedChange={(showLabels) => setGraphSettings({ showLabels })}
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
              onCheckedChange={(showEdgeLabels) => setGraphSettings({ showEdgeLabels })}
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
              onValueChange={(nodeSize: 'small' | 'medium' | 'large') => setGraphSettings({ nodeSize })}
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
              onValueChange={(layout: 'force' | 'circular' | 'random') => setGraphSettings({ layout })}
            >
              <SelectTrigger className="w-[150px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="force">Force-Directed</SelectItem>
                <SelectItem value="circular">Circular</SelectItem>
                <SelectItem value="random">Random</SelectItem>
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
                setQuerySettings({ mode })
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
              onCheckedChange={(stream) => setQuerySettings({ stream })}
            />
          </div>
        </CardContent>
      </Card>

      {/* Data Management */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Trash2 className="h-5 w-5" />
            Data Management
          </CardTitle>
          <CardDescription>
            Manage local data and reset settings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Clear History */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Query History</label>
              <p className="text-xs text-muted-foreground">
                Clear all saved query history
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="outline" size="sm">
                  Clear History
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Clear query history?</AlertDialogTitle>
                  <AlertDialogDescription>
                    This will permanently delete all your saved queries and favorites.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={handleClearHistory}>
                    Clear
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>

          <Separator />

          {/* Reset Settings */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Reset All Settings</label>
              <p className="text-xs text-muted-foreground">
                Reset all settings to their default values
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
                    This will reset all settings to their default values. Your data will not be affected.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={handleResetSettings}>
                    Reset
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
