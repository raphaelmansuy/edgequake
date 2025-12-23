// Final streaming test
async function testStreamingAPI() {
  try {
    console.log('🧪 Final streaming API test...');
    
    const response = await fetch('http://localhost:8080/api/v1/query/stream', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        query: 'What are the benefits of using a knowledge graph?',
        mode: 'hybrid',
        stream: true
      })
    });
    
    if (!response.ok) {
      console.error('❌ Response not ok:', response.status);
      return;
    }
    
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let result = '';
    let tokenCount = 0;
    
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      
      const chunk = decoder.decode(value, { stream: true });
      const lines = chunk.split('\n');
      
      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed.startsWith('data:')) {
          const content = trimmed.slice(5);
          if (content) {
            result += content;
            tokenCount++;
          }
        }
      }
    }
    
    console.log('✅ Final streaming test completed');
    console.log('📊 Tokens received:', tokenCount);
    console.log('📝 Sample result (first 100 chars):', result.substring(0, 100));
    console.log('🔍 Word separation check:', result.includes(' knowledge ') && result.includes(' graph '));
    
    return result;
    
  } catch (error) {
    console.error('❌ Final streaming test error:', error);
  }
}

testStreamingAPI();