#!/usr/bin/env node
/**
 * SSE Event Capture Diagnostic Tool
 * 
 * Purpose: Capture raw SSE events from the streaming API to understand
 * exactly what bytes/content are coming from the server.
 * 
 * This helps identify:
 * 1. Whether markdown formatting issues originate from the LLM
 * 2. Whether the server SSE format is correct
 * 3. What tokens arrive and in what sequence
 */

const BACKEND_URL = 'http://localhost:8080/api/v1';

// Valid IDs from the database
const TENANT_ID = 'cbbef300-8906-4d0c-a00a-1f09c084d6c2';
const USER_ID = '00000000-0000-0000-0000-000000000001';

// Test queries that are likely to produce markdown
const TEST_QUERIES = [
  "What are the main entities in my knowledge graph?",
  "List the key concepts in my documents with bullet points",
  "Summarize the products and concepts in bold formatting"
];

async function captureSSEEvents(query) {
  console.log('\n' + '='.repeat(80));
  console.log(`QUERY: "${query}"`);
  console.log('='.repeat(80));

  const requestBody = {
    conversation_id: null,
    message: query,
    mode: 'hybrid',
    stream: true
  };

  try {
    const response = await fetch(`${BACKEND_URL}/chat/completions/stream`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Tenant-ID': TENANT_ID,
        'X-User-ID': USER_ID,
      },
      body: JSON.stringify(requestBody)
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    
    let buffer = '';
    let eventCount = 0;
    let tokenChunks = [];
    let rawContent = '';
    
    console.log('\n--- RAW SSE EVENTS ---\n');

    while (true) {
      const { done, value } = await reader.read();
      
      if (done) {
        if (buffer.trim()) {
          processEvent(buffer, eventCount++, tokenChunks);
        }
        break;
      }

      const chunk = decoder.decode(value, { stream: true });
      buffer += chunk;

      // Process complete events (separated by double newlines)
      const events = buffer.split('\n\n');
      buffer = events.pop() || '';

      for (const event of events) {
        if (event.trim()) {
          processEvent(event, eventCount++, tokenChunks);
        }
      }
    }

    // Reconstruct content from tokens
    rawContent = tokenChunks.join('');
    
    console.log('\n--- ANALYSIS ---\n');
    console.log(`Total events: ${eventCount}`);
    console.log(`Token chunks: ${tokenChunks.length}`);
    
    console.log('\n--- TOKEN SEQUENCE (showing whitespace) ---\n');
    tokenChunks.forEach((token, i) => {
      // Show whitespace explicitly
      const visualToken = token
        .replace(/ /g, '·')  // Visible space
        .replace(/\n/g, '↵\n')  // Visible newline
        .replace(/\t/g, '→');  // Visible tab
      console.log(`[${i.toString().padStart(3, '0')}] "${visualToken}"`);
    });

    console.log('\n--- RAW CONCATENATED CONTENT ---\n');
    console.log(rawContent);
    
    console.log('\n--- MARKDOWN ISSUES DETECTED ---\n');
    analyzeMarkdownIssues(rawContent, tokenChunks);

    return { rawContent, tokenChunks, eventCount };

  } catch (error) {
    console.error(`Error: ${error.message}`);
    return null;
  }
}

function processEvent(eventText, eventNum, tokenChunks) {
  const lines = eventText.split('\n');
  let jsonData = null;
  
  for (const line of lines) {
    if (line.startsWith('data:')) {
      let data = line.slice(5);
      if (data.startsWith(' ')) {
        data = data.slice(1); // Remove SSE-mandated space
      }
      
      try {
        jsonData = JSON.parse(data);
      } catch {
        // Not JSON, treat as raw data
        jsonData = { type: 'raw', content: data };
      }
    }
  }

  if (jsonData) {
    const typeColor = getTypeColor(jsonData.type);
    console.log(`[${eventNum.toString().padStart(3, '0')}] ${typeColor}${jsonData.type}\x1b[0m: ${formatEventContent(jsonData)}`);
    
    if (jsonData.type === 'token' && jsonData.content) {
      tokenChunks.push(jsonData.content);
    }
  }
}

function getTypeColor(type) {
  const colors = {
    'conversation': '\x1b[36m',  // Cyan
    'context': '\x1b[33m',       // Yellow
    'token': '\x1b[32m',         // Green
    'thinking': '\x1b[35m',      // Magenta
    'done': '\x1b[34m',          // Blue
    'error': '\x1b[31m',         // Red
  };
  return colors[type] || '\x1b[0m';
}

function formatEventContent(event) {
  switch (event.type) {
    case 'conversation':
      return `conv_id=${event.conversation_id?.slice(0, 8)}...`;
    case 'context':
      return `${event.sources?.length || 0} sources`;
    case 'token':
      // Show the actual token with visible whitespace
      const visible = event.content
        .replace(/ /g, '·')
        .replace(/\n/g, '↵')
        .replace(/\t/g, '→');
      return `"${visible}"`;
    case 'done':
      return `tokens=${event.tokens_used} duration=${event.duration_ms}ms`;
    case 'error':
      return `${event.code}: ${event.message}`;
    default:
      return JSON.stringify(event).slice(0, 50);
  }
}

function analyzeMarkdownIssues(content, tokens) {
  const issues = [];

  // Check for "word** " pattern (marker attached to previous word)
  const pattern1 = /([a-zA-Z0-9])\*\* /g;
  let match;
  while ((match = pattern1.exec(content)) !== null) {
    issues.push({
      type: 'BOLD_ATTACHED_PREV',
      position: match.index,
      context: content.slice(Math.max(0, match.index - 10), match.index + 20),
      description: '** marker attached to previous word'
    });
  }

  // Check for " **word" pattern (space before marker, marker attached to next word)
  const pattern2 = /\*\*(\S+)\*\* /g;
  while ((match = pattern2.exec(content)) !== null) {
    if (match[0].includes(' **') && match[0].includes('** ')) {
      issues.push({
        type: 'BOLD_SPACE_AROUND',
        position: match.index,
        context: content.slice(Math.max(0, match.index - 5), match.index + match[0].length + 5),
        description: 'Unusual spacing around bold markers'
      });
    }
  }

  // Check for "** text" pattern (space after opening marker)
  const pattern3 = /\*\* [^*]/g;
  while ((match = pattern3.exec(content)) !== null) {
    issues.push({
      type: 'BOLD_SPACE_AFTER_OPEN',
      position: match.index,
      context: content.slice(Math.max(0, match.index - 5), match.index + 15),
      description: 'Space after opening ** marker'
    });
  }

  // Check for "text **" pattern (space before closing marker)
  const pattern4 = /[^*] \*\*/g;
  while ((match = pattern4.exec(content)) !== null) {
    issues.push({
      type: 'BOLD_SPACE_BEFORE_CLOSE',
      position: match.index,
      context: content.slice(Math.max(0, match.index - 10), match.index + 10),
      description: 'Space before closing ** marker'
    });
  }

  // Analyze token boundaries for markdown issues
  let position = 0;
  for (let i = 0; i < tokens.length; i++) {
    const token = tokens[i];
    const nextToken = tokens[i + 1] || '';
    
    // Check if ** is split across tokens
    if (token.endsWith('*') && nextToken.startsWith('*')) {
      issues.push({
        type: 'SPLIT_MARKER',
        position,
        tokens: [token, nextToken],
        description: '** marker split across tokens'
      });
    }
    
    // Check if token boundary creates bad placement
    if (token === '**' && nextToken.startsWith(' ')) {
      issues.push({
        type: 'TOKEN_BOUNDARY_SPACE',
        position,
        tokens: [token, nextToken],
        description: '** followed by space-prefixed token'
      });
    }

    position += token.length;
  }

  if (issues.length === 0) {
    console.log('✅ No obvious markdown issues detected');
  } else {
    console.log(`⚠️  Found ${issues.length} potential issues:\n`);
    issues.forEach((issue, i) => {
      console.log(`${i + 1}. [${issue.type}] at position ${issue.position}`);
      console.log(`   Description: ${issue.description}`);
      if (issue.context) {
        console.log(`   Context: "${issue.context}"`);
      }
      if (issue.tokens) {
        console.log(`   Tokens: ${JSON.stringify(issue.tokens)}`);
      }
      console.log();
    });
  }
}

// Main execution
async function main() {
  console.log('SSE Event Capture Diagnostic Tool');
  console.log('=' .repeat(80));
  console.log(`Backend: ${BACKEND_URL}`);
  console.log(`Time: ${new Date().toISOString()}`);
  
  // Use first test query
  const query = TEST_QUERIES[0];
  const result = await captureSSEEvents(query);
  
  if (result) {
    // Save raw output for further analysis
    console.log('\n--- SAVING RESULTS ---\n');
    const outputPath = './diagnostic_output.json';
    const output = {
      timestamp: new Date().toISOString(),
      query,
      eventCount: result.eventCount,
      tokenChunks: result.tokenChunks,
      rawContent: result.rawContent
    };
    
    const fs = await import('fs');
    fs.writeFileSync(outputPath, JSON.stringify(output, null, 2));
    console.log(`Saved to: ${outputPath}`);
  }
}

main().catch(console.error);
