import { expect, test, type Page } from '@playwright/test';

const TENANT = {
  id: 'tenant-mission-06',
  name: 'Mission Tenant',
  slug: 'mission-tenant',
  created_at: '2026-04-16T00:00:00Z',
  updated_at: '2026-04-16T00:00:00Z',
};

const WORKSPACE = {
  id: 'workspace-mission-06',
  tenant_id: TENANT.id,
  name: 'Mission Workspace',
  slug: 'mission-workspace',
  description: 'Workspace used for stable mission verification',
  entity_types: ['PERSON', 'ORGANIZATION', 'MACHINE'],
  llm_provider: 'openai',
  llm_model: 'gpt-4o-mini',
  embedding_provider: 'openai',
  embedding_model: 'text-embedding-3-small',
  embedding_dimension: 1536,
  created_at: '2026-04-16T00:00:00Z',
  updated_at: '2026-04-16T00:00:00Z',
};

const BASE_GRAPH = {
  nodes: [
    {
      id: 'node-1',
      label: 'ALPHA',
      node_type: 'PERSON',
      entity_type: 'PERSON',
      description: 'Alpha entity',
      degree: 1,
      properties: { source: 'mission-test' },
    },
    {
      id: 'node-2',
      label: 'BETA',
      node_type: 'ORGANIZATION',
      entity_type: 'ORGANIZATION',
      description: 'Beta entity',
      degree: 1,
      properties: { source: 'mission-test' },
    },
  ],
  edges: [
    {
      source: 'node-1',
      target: 'node-2',
      relationship_type: 'WORKS_AT',
      weight: 1,
      properties: {},
    },
  ],
};

type MergePayload = {
  source_entity: string;
  target_entity: string;
  merge_strategy?: string;
};

type MockState = {
  workspaceDeleted: boolean;
  mergeCalls: MergePayload[];
  deletedEntities: string[];
};

function buildStreamBody() {
  return [
    `data: ${JSON.stringify({
      type: 'metadata',
      total_nodes: BASE_GRAPH.nodes.length,
      total_edges: BASE_GRAPH.edges.length,
      nodes_to_stream: BASE_GRAPH.nodes.length,
      edges_to_stream: BASE_GRAPH.edges.length,
    })}`,
    `data: ${JSON.stringify({
      type: 'nodes',
      nodes: BASE_GRAPH.nodes,
      batch: 1,
      total_batches: 1,
    })}`,
    `data: ${JSON.stringify({
      type: 'edges',
      edges: BASE_GRAPH.edges,
    })}`,
    `data: ${JSON.stringify({
      type: 'done',
      nodes_count: BASE_GRAPH.nodes.length,
      edges_count: BASE_GRAPH.edges.length,
      duration_ms: 1,
    })}`,
    '',
  ].join('\n\n');
}

async function installMissionMocks(page: Page, state: MockState) {
  await page.addInitScript(
    ({ tenantId, workspaceId }) => {
      localStorage.clear();
      sessionStorage.clear();

      localStorage.setItem('tenantId', tenantId);
      localStorage.setItem('workspaceId', workspaceId);
      localStorage.setItem(
        'edgequake-tenant',
        JSON.stringify({
          state: {
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
          },
          version: 1,
        }),
      );
    },
    { tenantId: TENANT.id, workspaceId: WORKSPACE.id },
  );

  await page.route('**/health', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ status: 'healthy' }),
    });
  });

  await page.route('**/ready', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ status: 'ready' }),
    });
  });

  await page.route('**/live**', async (route) => {
    await route.fulfill({ status: 200, body: 'OK' });
  });

  await page.route('**/api/v1/tenants', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([TENANT]),
    });
  });

  await page.route('**/api/v1/tenants/*/workspaces**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(state.workspaceDeleted ? [] : [WORKSPACE]),
    });
  });

  await page.route(`**/api/v1/workspaces/${WORKSPACE.id}/stats`, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        documents_count: 2,
        entities_count: BASE_GRAPH.nodes.length,
        relationships_count: BASE_GRAPH.edges.length,
        last_updated: '2026-04-16T00:00:00Z',
      }),
    });
  });

  await page.route(`**/api/v1/workspaces/${WORKSPACE.id}`, async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: state.workspaceDeleted ? 404 : 200,
        contentType: 'application/json',
        body: JSON.stringify(state.workspaceDeleted ? { message: 'Not found' } : WORKSPACE),
      });
      return;
    }

    await route.fallback();
  });

  await page.route('**/api/v1/models/discover/**', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 'ok', message: 'discovery refreshed' }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route('**/api/v1/models**', async (route) => {
    const url = route.request().url();
    if (url.includes('/models/health') || url.includes('/models/discover')) {
      await route.fallback();
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        providers: [
          {
            name: 'openai',
            enabled: true,
            display_name: 'OpenAI',
            provider_type: 'openai',
            priority: 1,
            description: 'OpenAI API',
            models: [
              {
                name: 'gpt-4o-mini',
                display_name: 'GPT-4o Mini',
                model_type: 'llm',
                capabilities: { supports_streaming: true },
                cost: { input_per_1k: 0, output_per_1k: 0 },
              },
              {
                name: 'text-embedding-3-small',
                display_name: 'Text Embedding 3 Small',
                model_type: 'embedding',
                capabilities: { supports_streaming: false },
                cost: { input_per_1k: 0, output_per_1k: 0 },
              },
            ],
          },
        ],
        default_llm_provider: 'openai',
        default_llm_model: 'gpt-4o-mini',
        default_embedding_provider: 'openai',
        default_embedding_model: 'text-embedding-3-small',
      }),
    });
  });

  // Must register after the catch-all `/models**` route so health wins precedence.
  await page.route('**/api/v1/models/health**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          name: 'openai',
          display_name: 'OpenAI',
          provider_type: 'openai',
          enabled: true,
          priority: 1,
          description: 'OpenAI API',
          auth_kind: 'api_key',
          models: [
            {
              name: 'gpt-4o-mini',
              display_name: 'GPT-4o Mini',
              model_type: 'llm',
              capabilities: { supports_streaming: true },
            },
          ],
          health: {
            available: true,
            latency_ms: 50,
            checked_at: '2026-04-16T00:00:00Z',
          },
        },
      ]),
    });
  });

  await page.route('**/api/v1/graph/stream**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: buildStreamBody(),
      headers: {
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      },
    });
  });

  await page.route('**/api/v1/graph**', async (route) => {
    if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(BASE_GRAPH),
      });
      return;
    }

    await route.fallback();
  });
}

test.describe('Mission 06 regression proof', () => {
  test('workspace delete flow is explicit and clears the selection', async ({ page }) => {
    const state: MockState = {
      workspaceDeleted: false,
      mergeCalls: [],
      deletedEntities: [],
    };

    await installMissionMocks(page, state);

    await page.route(`**/api/v1/workspaces/${WORKSPACE.id}`, async (route) => {
      if (route.request().method() === 'DELETE') {
        state.workspaceDeleted = true;
        await route.fulfill({ status: 204, body: '' });
        return;
      }

      await route.fallback();
    });

    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto('/workspace');

    await expect(page.getByRole('heading', { name: WORKSPACE.name })).toBeVisible({
      timeout: 15_000,
    });

    const deleteWorkspaceButton = page.getByRole('button', {
      name: /delete( this)? workspace/i,
    });
    await deleteWorkspaceButton.scrollIntoViewIfNeeded();
    await expect(deleteWorkspaceButton).toBeVisible();

    await deleteWorkspaceButton.click();

    const dialog = page.locator('[role="alertdialog"]');
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText(/delete workspace/i);
    await expect(dialog).toContainText(/permanently remove all documents/i);

    await dialog.getByRole('button', { name: /^Delete$/ }).click();

    await expect.poll(() => state.workspaceDeleted).toBe(true);
    await expect(page).toHaveURL(/\/$/, { timeout: 15_000 });
  });

  test('graph actions expose workspace entity types and wire merge and delete', async ({ page }) => {
    const state: MockState = {
      workspaceDeleted: false,
      mergeCalls: [],
      deletedEntities: [],
    };

    await installMissionMocks(page, state);

    await page.route('**/api/v1/graph/entities/merge', async (route) => {
      state.mergeCalls.push(route.request().postDataJSON() as MergePayload);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'success',
          message: 'Entities merged successfully',
          merged_entity: {
            id: 'BETA',
            entity_name: 'BETA',
            entity_type: 'ORGANIZATION',
            description: 'Beta entity',
          },
          merge_details: {
            source_entity_id: 'ALPHA',
            target_entity_id: 'BETA',
            relationships_merged: 1,
            duplicate_relationships_removed: 0,
            description_strategy: 'prefer_target',
            metadata_strategy: 'merge',
          },
        }),
      });
    });

    await page.route('**/api/v1/graph/entities/*', async (route) => {
      if (route.request().method() === 'DELETE') {
        const entityId = route.request().url().split('/').pop();
        if (entityId) {
          state.deletedEntities.push(entityId);
        }
        await route.fulfill({ status: 204, body: '' });
        return;
      }

      await route.fallback();
    });

    await page.goto('/graph?stream=0');

    await expect(page.getByRole('complementary', { name: /entity browser/i })).toBeVisible();
    await expect(page.getByText('ALPHA')).toBeVisible({ timeout: 10000 });

    await page.getByText('ALPHA').first().click();

    await expect(page.getByRole('button', { name: /^Edit/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /^Merge/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /^Delete/i })).toBeVisible();

    await page.getByRole('button', { name: /^Edit/i }).click();
    const editDialog = page.locator('[role="dialog"]').last();
    await expect(editDialog).toContainText(/edit entity/i);

    await editDialog.locator('#entity-type').click();
    await expect(page.getByRole('option', { name: 'MACHINE' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('listbox')).not.toBeVisible();
    await page.keyboard.press('Escape');
    await expect(editDialog).not.toBeVisible();

    await page.getByText('ALPHA').first().click();
    await page.getByRole('button', { name: /^Merge/i }).click();
    const mergeDialog = page.locator('[role="dialog"]').last();
    await expect(mergeDialog).toContainText(/merge entities/i);
    await mergeDialog.getByTestId('merge-target-combobox').click();
    await page.getByTestId('merge-target-search').fill('BETA');
    await page.getByRole('option', { name: /BETA/i }).click();
    await mergeDialog.getByRole('button', { name: /merge entities/i }).click();

    const mergeConfirmDialog = page.locator('[role="dialog"]').last();
    await expect(mergeConfirmDialog).toContainText(/confirm merge/i);
    await mergeConfirmDialog.getByRole('button', { name: /merge entities/i }).click();

    await expect.poll(() => state.mergeCalls.length).toBe(1);
    await expect.poll(() => state.mergeCalls[0]?.target_entity).toBe('BETA');
    await expect.poll(() => state.mergeCalls[0]?.source_entity).toBe('ALPHA');

    await page.getByText('BETA').first().click();
    await page.getByRole('button', { name: /^Delete/i }).click();

    const deleteDialog = page.locator('[role="alertdialog"]').last();
    await expect(deleteDialog).toContainText(/delete entity/i);
    await deleteDialog.getByRole('button', { name: /^Delete$/ }).click();

    await expect.poll(() => state.deletedEntities.includes('node-2')).toBe(true);
  });
});
