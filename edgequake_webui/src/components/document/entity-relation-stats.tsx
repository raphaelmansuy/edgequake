// Entity and relation stats with link to graph
'use client';

import { Button } from '@/components/ui/button';
import { ExternalLink } from 'lucide-react';
import { useRouter } from 'next/navigation';

interface EntityRelationStatsProps {
  entities?: number;
  relationships?: number;
  documentId: string;
}

export function EntityRelationStats({
  entities,
  relationships,
  documentId,
}: EntityRelationStatsProps) {
  const router = useRouter();

  const handleViewInGraph = () => {
    router.push(`/graph?highlight=${documentId}`);
  };

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <div className="p-3 rounded-lg bg-purple-500/10 border border-purple-200 dark:border-purple-900">
          <div className="text-xs text-muted-foreground mb-1">Entities</div>
          <div className="text-2xl font-bold text-purple-600 dark:text-purple-400">
            {entities ?? 0}
          </div>
        </div>
        <div className="p-3 rounded-lg bg-green-500/10 border border-green-200 dark:border-green-900">
          <div className="text-xs text-muted-foreground mb-1">Relations</div>
          <div className="text-2xl font-bold text-green-600 dark:text-green-400">
            {relationships ?? 0}
          </div>
        </div>
      </div>
      
      <Button 
        variant="outline" 
        className="w-full" 
        size="sm"
        onClick={handleViewInGraph}
      >
        <ExternalLink className="h-3.5 w-3.5 mr-2" />
        View in Knowledge Graph
      </Button>
    </div>
  );
}
