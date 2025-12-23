// Test script to reproduce the streaming issue
async function testStreaming() {
  try {
    console.log('Testing streaming API...');
    
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
      console.error('Response not ok:', response.status, response.statusText);
      return;
    }
    
    console.log('Response received, reading stream...');
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let result = '';
    
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      
      const chunk = decoder.decode(value, { stream: true });
      console.log('Chunk received:', JSON.stringify(chunk));
      
      // Process SSE data
      const lines = chunk.split('\n');
      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed.startsWith('data:')) {
          const content = trimmed.slice(5);
          result += content;
          console.log('Token:', JSON.stringify(content));
        }
      }
    }
    
    console.log('\nFinal result:');
    console.log(result);
    
  } catch (error) {
    console.error('Error:', error);
  }
}

testStreaming();