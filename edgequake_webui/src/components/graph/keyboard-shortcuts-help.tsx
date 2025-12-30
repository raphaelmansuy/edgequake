'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from '@/components/ui/dialog';
import { Keyboard } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface ShortcutGroup {
  title: string;
  shortcuts: Array<{
    keys: string[];
    description: string;
  }>;
}

const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    title: 'Navigation',
    shortcuts: [
      { keys: ['Tab'], description: 'Next node' },
      { keys: ['Shift', 'Tab'], description: 'Previous node' },
      { keys: ['↑', '↓', '←', '→'], description: 'Navigate nodes' },
      { keys: ['Enter'], description: 'Focus on selected node' },
      { keys: ['Escape'], description: 'Deselect node' },
    ],
  },
  {
    title: 'View Controls',
    shortcuts: [
      { keys: ['+'], description: 'Zoom in' },
      { keys: ['-'], description: 'Zoom out' },
      { keys: ['0'], description: 'Reset view' },
      { keys: ['F'], description: 'Toggle fullscreen' },
    ],
  },
  {
    title: 'Search',
    shortcuts: [
      { keys: ['⌘', 'K'], description: 'Open search' },
      { keys: ['/'], description: 'Quick search' },
    ],
  },
];

export function KeyboardShortcutsHelp() {
  const { t } = useTranslation();

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          aria-label={t('graph.keyboard.showShortcuts', 'Keyboard shortcuts')}
        >
          <Keyboard className="h-4 w-4" />
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Keyboard className="h-5 w-5" />
            {t('graph.keyboard.title', 'Keyboard Shortcuts')}
          </DialogTitle>
          <DialogDescription>
            {t('graph.keyboard.description', 'Use these shortcuts to navigate the graph efficiently.')}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-6 pt-4">
          {SHORTCUT_GROUPS.map((group) => (
            <div key={group.title}>
              <h4 className="text-sm font-medium text-muted-foreground mb-3">
                {group.title}
              </h4>
              <div className="space-y-2">
                {group.shortcuts.map(({ keys, description }) => (
                  <div
                    key={description}
                    className="flex items-center justify-between text-sm"
                  >
                    <span className="text-foreground">{description}</span>
                    <div className="flex items-center gap-1">
                      {keys.map((key, index) => (
                        <span key={key} className="flex items-center gap-1">
                          {index > 0 && <span className="text-muted-foreground">+</span>}
                          <kbd className="px-2 py-1 text-xs font-mono bg-muted rounded border border-border">
                            {key}
                          </kbd>
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
        <div className="pt-4 border-t mt-4">
          <p className="text-xs text-muted-foreground text-center">
            Press <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-muted rounded border">?</kbd> to show this dialog anytime
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default KeyboardShortcutsHelp;
