// Debug streaming in the browser console
window.testStreaming = async () => {
  console.clear();
  console.log('🔍 Testing streaming with EdgeQuake API...');
  
  try {
    const response = await fetch('http://localhost:8080/api/v1/query/stream', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        query: 'What is AI?',
        mode: 'hybrid',
        stream: true
      })
    });
    
    if (!response.ok) {
      console.error('❌ Response not ok:', response.status, response.statusText);
      return;
    }
    
    console.log('✅ Response received, reading stream...');
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    let result = '';
    let tokenCount = 0;
    
    while (true) {
      const { value, done } = await reader.read();
      if (done) {
        // Process any remaining buffer
        if (buffer.trim()) {
          console.log('🔚 Final buffer:', JSON.stringify(buffer));
          const parsed = parseSSEData(buffer);
          if (parsed !== null) {
            console.log('🔚 Final parsed:', parsed);
            if (parsed.content) {
              result += parsed.content;
              tokenCount++;
            }
          }
        }
        break;
      }
      
      buffer += decoder.decode(value, { stream: true });
      console.log('📦 Buffer:', JSON.stringify(buffer));
      
      // Process complete SSE events (separated by double newlines)
      const events = buffer.split('\n\n');
      buffer = events.pop() || '';
      
      console.log('🎯 Events to process:', events.length);
      
      for (const event of events) {
        if (event.trim()) {
          console.log('🔄 Processing event:', JSON.stringify(event));
          const parsed = parseSSEData(event);
          console.log('⚡ Parsed result:', parsed);
          
          if (parsed && parsed.content) {
            result += parsed.content;
            tokenCount++;
            console.log(`📝 Token ${tokenCount}: "${parsed.content}"`);
            console.log(`📄 Result so far: "${result}"`);
          }
        }
      }
    }
    
    console.log('\n🎉 FINAL RESULTS:');
    console.log('📊 Total tokens:', tokenCount);
    console.log('📜 Full result:', JSON.stringify(result));
    console.log('🖊️  Result preview:', result);
    
    return result;
    
  } catch (error) {
    console.error('❌ Error:', error);
  }
};

// Helper function to parse SSE data (same as in client.ts)
function parseSSEData(event) {
  const lines = event.split('\n');
  const dataChunks = [];

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('data:')) {
      // Extract content after "data: " - preserve leading space for word separation
      const content = trimmed.slice(5); // Don't trim start to preserve spaces!
      if (content) {
        dataChunks.push(content);
      }
    } else if (
      trimmed.startsWith('event:') ||
      trimmed.startsWith('id:') ||
      trimmed.startsWith('retry:')
    ) {
      // Ignore other SSE fields for now
      continue;
    } else if (trimmed && !trimmed.startsWith(':')) {
      // Non-SSE line - might be plain content or NDJSON fallback
      dataChunks.push(trimmed);
    }
  }

  if (dataChunks.length === 0) {
    return null;
  }

  // Join data chunks - preserve spaces for word separation
  const data = dataChunks.join('');

  // Try to parse as JSON first (structured response)
  try {
    return JSON.parse(data);
  } catch {
    // Not JSON, return as raw text wrapped in expected format
    return { type: 'token', content: data };
  }
}

console.log('🚀 Debug script loaded! Run: await testStreaming()');