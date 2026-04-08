
import { test } from '@playwright/test';

test('capture graph API response to check edge relationship_type', async ({ page }) => {
  const responses: any[] = [];
  
  page.on('response', async response => {
    const url = response.url();
    if (url.includes('/graph') && !url.includes('labels') && !url.includes('stats') && !url.includes('stream')) {
      try {
        const body = await response.json();
        const edges = body.edges || [];
        console.log('GRAPH API URL:', url.replace(/(https?:\/\/[^\/]+)/, ''));
        console.log('EDGE COUNT:', edges.length);
        for (const e of edges.slice(0, 5)) {
          console.log('  EDGE rel_type=' + (e.relationship_type || 'NULL/EMPTY') + ' src=' + (e.source || '').substring(0, 20) + ' tgt=' + (e.target || '').substring(0, 20));
        }
        responses.push({ url, edges: edges.length, sample: edges.slice(0,3) });
      } catch {}
    }
  });
  
  await page.goto('/graph');
  await page.waitForTimeout(5000);
  
  if (responses.length === 0) {
    console.log('NO GRAPH API CALLS CAPTURED');
    
    // Check what network requests were made
    const allUrls = await page.evaluate(() => {
      return window.__requestUrls || [];
    });
    console.log('Page URL:', page.url());
  }
});
