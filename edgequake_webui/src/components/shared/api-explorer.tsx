/**
 * @module ApiExplorer
 * @description Interactive API endpoint explorer for testing.
 * Allows users to test API calls directly from the UI.
 * 
 * @implements UC0901 - Developer tests API endpoints
 * @implements FEAT0639 - Interactive API testing
 * @implements FEAT0640 - Request/response visualization
 * @implements SPEC-032 - Provider integration endpoints
 * 
 * @enforces BR0625 - API responses formatted for readability
 * 
 * @see {@link specs/API.md} for endpoint specifications
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Textarea } from '@/components/ui/textarea';
import { api } from '@/lib/api/client';
import {
    Check,
    ChevronDown,
    ChevronRight,
    Clock,
    Copy,
    Loader2,
    Play,
} from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';

interface Endpoint {
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  path: string;
  description: string;
  category: string;
  body?: string;
}

const endpoints: Endpoint[] = [
  // Health
  { method: 'GET', path: '/health', description: 'Check API health status', category: 'Health' },
  
  // Auth
  { method: 'POST', path: '/auth/login', description: 'Authenticate user', category: 'Auth', body: '{\n  "username": "admin",\n  "password": "password"\n}' },
  { method: 'GET', path: '/auth/me', description: 'Get current user', category: 'Auth' },
  
  // Models (SPEC-032)
  { method: 'GET', path: '/models', description: 'List all available models grouped by provider', category: 'Models' },
  { method: 'GET', path: '/models/check/{provider}', description: 'Check provider availability', category: 'Models' },
  { method: 'GET', path: '/models/{provider}/models', description: 'List models for a specific provider', category: 'Models' },
  { method: 'GET', path: '/models/status', description: 'Get status of all configured providers', category: 'Models' },
  
  // Documents
  { method: 'GET', path: '/documents', description: 'List all documents', category: 'Documents' },
  { method: 'POST', path: '/documents', description: 'Upload document', category: 'Documents', body: '{\n  "content": "Your document text here...",\n  "title": "My Document",\n  "source_type": "text"\n}' },
  { method: 'GET', path: '/documents/{id}', description: 'Get document by ID', category: 'Documents' },
  { method: 'DELETE', path: '/documents/{id}', description: 'Delete document', category: 'Documents' },
  
  // Query
  { method: 'POST', path: '/query', description: 'Query knowledge graph', category: 'Query', body: '{\n  "query": "What is the main topic?",\n  "mode": "hybrid",\n  "top_k": 10\n}' },
  
  // Graph
  { method: 'GET', path: '/graph', description: 'Get knowledge graph', category: 'Graph' },
  { method: 'GET', path: '/graph/labels', description: 'Get graph labels', category: 'Graph' },
  { method: 'GET', path: '/graph/stats', description: 'Get graph statistics', category: 'Graph' },
  
  // Entities
  { method: 'GET', path: '/entities', description: 'List all entities', category: 'Entities' },
  { method: 'GET', path: '/entities/{id}', description: 'Get entity by ID', category: 'Entities' },
  { method: 'PATCH', path: '/entities/{id}', description: 'Update entity', category: 'Entities', body: '{\n  "label": "New Label",\n  "description": "Updated description"\n}' },
  { method: 'DELETE', path: '/entities/{id}', description: 'Delete entity', category: 'Entities' },
  { method: 'POST', path: '/entities/merge', description: 'Merge entities', category: 'Entities', body: '{\n  "source_ids": ["id1", "id2"],\n  "target_label": "Merged Entity"\n}' },
  
  // Relationships
  { method: 'GET', path: '/relationships', description: 'List all relationships', category: 'Relationships' },
  { method: 'DELETE', path: '/relationships/{id}', description: 'Delete relationship', category: 'Relationships' },
  
  // Pipeline
  { method: 'GET', path: '/pipeline/status', description: 'Get pipeline status', category: 'Pipeline' },
  
  // Tenants
  { method: 'GET', path: '/tenants', description: 'List all tenants', category: 'Tenants' },
  { method: 'POST', path: '/tenants', description: 'Create a new tenant', category: 'Tenants', body: '{\n  "name": "My Tenant",\n  "description": "Tenant description",\n  "default_llm_provider": "ollama",\n  "default_llm_model": "gemma3:12b"\n}' },
  { method: 'GET', path: '/tenants/{id}', description: 'Get tenant by ID', category: 'Tenants' },
  { method: 'DELETE', path: '/tenants/{id}', description: 'Delete tenant', category: 'Tenants' },
  
  // Workspaces
  { method: 'GET', path: '/tenants/{tenant_id}/workspaces', description: 'List workspaces in tenant', category: 'Workspaces' },
  { method: 'POST', path: '/tenants/{tenant_id}/workspaces', description: 'Create workspace', category: 'Workspaces', body: '{\n  "name": "My Workspace",\n  "llm_provider": "ollama",\n  "llm_model": "gemma3:12b",\n  "embedding_provider": "ollama",\n  "embedding_model": "embeddinggemma",\n  "embedding_dimension": 768\n}' },
];

const methodColors = {
  GET: 'bg-green-500/10 text-green-600 border border-green-500/30 dark:bg-green-500/20 dark:text-green-400',
  POST: 'bg-blue-500/10 text-blue-600 border border-blue-500/30 dark:bg-blue-500/20 dark:text-blue-400',
  PUT: 'bg-yellow-500/10 text-yellow-600 border border-yellow-500/30 dark:bg-yellow-500/20 dark:text-yellow-400',
  PATCH: 'bg-orange-500/10 text-orange-600 border border-orange-500/30 dark:bg-orange-500/20 dark:text-orange-400',
  DELETE: 'bg-red-500/10 text-red-600 border border-red-500/30 dark:bg-red-500/20 dark:text-red-400',
} as const;

export function ApiExplorer() {
  const [selectedEndpoint, setSelectedEndpoint] = useState<Endpoint | null>(null);
  const [requestBody, setRequestBody] = useState('');
  const [response, setResponse] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);
  const [copied, setCopied] = useState(false);
  const [responseTime, setResponseTime] = useState<number | null>(null);
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(
    new Set(endpoints.map((e) => e.category))
  );

  const categories = [...new Set(endpoints.map((e) => e.category))];

  const toggleCategory = (category: string) => {
    const newExpanded = new Set(expandedCategories);
    if (newExpanded.has(category)) {
      newExpanded.delete(category);
    } else {
      newExpanded.add(category);
    }
    setExpandedCategories(newExpanded);
  };

  const selectEndpoint = (endpoint: Endpoint) => {
    setSelectedEndpoint(endpoint);
    setRequestBody(endpoint.body || '');
    setResponse('');
    setResponseTime(null);
  };

  const executeRequest = async () => {
    if (!selectedEndpoint) return;

    setIsLoading(true);
    setResponse('');
    setResponseTime(null);
    const startTime = performance.now();

    try {
      let result: unknown;
      const path = selectedEndpoint.path;
      const body = requestBody ? JSON.parse(requestBody) : undefined;

      switch (selectedEndpoint.method) {
        case 'GET':
          result = await api.get(path);
          break;
        case 'POST':
          result = await api.post(path, body);
          break;
        case 'PUT':
          result = await api.put(path, body);
          break;
        case 'PATCH':
          result = await api.patch(path, body);
          break;
        case 'DELETE':
          result = await api.delete(path);
          break;
      }

      const endTime = performance.now();
      setResponseTime(endTime - startTime);
      setResponse(JSON.stringify(result, null, 2));
    } catch (error) {
      const endTime = performance.now();
      setResponseTime(endTime - startTime);
      if (error instanceof SyntaxError) {
        setResponse(JSON.stringify({ error: 'Invalid JSON in request body' }, null, 2));
      } else if (error instanceof Error) {
        setResponse(JSON.stringify({ error: error.message }, null, 2));
      } else {
        setResponse(JSON.stringify({ error: 'Request failed' }, null, 2));
      }
    } finally {
      setIsLoading(false);
    }
  };

  const copyResponse = async () => {
    await navigator.clipboard.writeText(response);
    setCopied(true);
    toast.success('Copied to clipboard');
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="flex h-full">
      {/* Endpoint List */}
      <div className="w-80 border-r bg-card overflow-auto">
        <div className="p-4">
          <h2 className="text-lg font-semibold mb-4">API Endpoints</h2>
          <div className="space-y-2">
            {categories.map((category) => (
              <Collapsible
                key={category}
                open={expandedCategories.has(category)}
                onOpenChange={() => toggleCategory(category)}
              >
                <CollapsibleTrigger className="flex items-center gap-2 w-full p-2 hover:bg-muted rounded text-sm font-medium">
                  {expandedCategories.has(category) ? (
                    <ChevronDown className="h-4 w-4" />
                  ) : (
                    <ChevronRight className="h-4 w-4" />
                  )}
                  {category}
                  <Badge variant="secondary" className="ml-auto">
                    {endpoints.filter((e) => e.category === category).length}
                  </Badge>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="ml-4 space-y-1 mt-1">
                    {endpoints
                      .filter((e) => e.category === category)
                      .map((endpoint, index) => (
                        <button
                          key={`${endpoint.method}-${endpoint.path}-${index}`}
                          onClick={() => selectEndpoint(endpoint)}
                          className={`w-full flex items-center gap-2 p-2 rounded text-sm text-left hover:bg-muted ${
                            selectedEndpoint === endpoint ? 'bg-muted' : ''
                          }`}
                        >
                          <Badge
                            className={`${methodColors[endpoint.method]} text-[10px] px-1.5 font-semibold`}
                          >
                            {endpoint.method}
                          </Badge>
                          <span className="truncate font-mono text-xs">{endpoint.path}</span>
                        </button>
                      ))}
                  </div>
                </CollapsibleContent>
              </Collapsible>
            ))}
          </div>
        </div>
      </div>

      {/* Request/Response Area */}
      <div className="flex-1 flex flex-col">
        {selectedEndpoint ? (
          <>
            {/* Header */}
            <div className="flex items-center gap-4 border-b px-4 py-3">
              <Badge className={`${methodColors[selectedEndpoint.method]} font-semibold`}>
                {selectedEndpoint.method}
              </Badge>
              <code className="text-sm font-mono">{selectedEndpoint.path}</code>
              <span className="text-sm text-muted-foreground flex-1">
                {selectedEndpoint.description}
              </span>
              <Button onClick={executeRequest} disabled={isLoading}>
                {isLoading ? (
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                ) : (
                  <Play className="h-4 w-4 mr-2" />
                )}
                Execute
              </Button>
            </div>

            {/* Request Body */}
            {selectedEndpoint.body !== undefined && (
              <div className="border-b p-4">
                <h3 className="text-sm font-medium mb-2">Request Body</h3>
                <Textarea
                  value={requestBody}
                  onChange={(e) => setRequestBody(e.target.value)}
                  placeholder="Enter JSON request body..."
                  className="font-mono text-sm min-h-[150px]"
                />
              </div>
            )}

            {/* Response */}
            <div className="flex-1 flex flex-col min-h-0">
              <div className="flex items-center justify-between px-4 py-2 border-b">
                <div className="flex items-center gap-3">
                  <h3 className="text-sm font-medium">Response</h3>
                  {responseTime !== null && (
                    <Badge variant="outline" className="text-xs flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {responseTime < 1000 
                        ? `${Math.round(responseTime)}ms`
                        : `${(responseTime / 1000).toFixed(2)}s`
                      }
                    </Badge>
                  )}
                </div>
                {response && (
                  <Button variant="ghost" size="sm" onClick={copyResponse}>
                    {copied ? (
                      <Check className="h-4 w-4 mr-1" />
                    ) : (
                      <Copy className="h-4 w-4 mr-1" />
                    )}
                    Copy
                  </Button>
                )}
              </div>
              <ScrollArea className="flex-1 p-4">
                {response ? (
                  <pre className="text-sm font-mono whitespace-pre-wrap">{response}</pre>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    Click Execute to see the response
                  </p>
                )}
              </ScrollArea>
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-center">
            <div>
              <h2 className="text-lg font-medium mb-2">Select an Endpoint</h2>
              <p className="text-sm text-muted-foreground">
                Choose an API endpoint from the list to test it
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default ApiExplorer;
