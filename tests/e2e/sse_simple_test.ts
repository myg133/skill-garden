/**
 * Simple SSE Test using native fetch and EventSource
 */

const SSE_URL = "http://127.0.0.1:8080/sse";
const MESSAGE_URL = "http://127.0.0.1:8080/sse";

// Test 1: Connect to SSE and receive endpoint event
async function testSseConnect() {
  console.log("Test 1: Connecting to SSE endpoint...");
  const response = await fetch(SSE_URL, {
    headers: {
      "Accept": "text/event-stream",
    }
  });

  if (!response.ok) {
    throw new Error(`SSE connection failed: ${response.status}`);
  }

  const reader = response.body?.getReader();
  if (!reader) throw new Error("No response body");

  const decoder = new TextDecoder();
  let endpoint = null;

  // Read until we get the endpoint event
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    const text = decoder.decode(value, { stream: true });
    console.log("Received:", text);

    // Parse SSE data
    const lines = text.split("\n");
    for (const line of lines) {
      if (line.startsWith("data:")) {
        const data = line.slice(5).trim();
        try {
          const parsed = JSON.parse(data);
          if (parsed.endpoint) {
            endpoint = parsed.endpoint;
          }
        } catch {
          // Not JSON, might be endpoint string
          if (data.startsWith("/sse/")) {
            endpoint = data;
          }
        }
      }
    }

    if (endpoint) break;
  }

  console.log("Got endpoint:", endpoint);
  reader.cancel();

  return endpoint;
}

// Test 2: Send a message to the SSE endpoint
async function testSendMessage(endpoint) {
  console.log("\nTest 2: Sending initialize message to", endpoint);

  const fullUrl = `http://127.0.0.1:8080${endpoint}`;

  const response = await fetch(fullUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      method: "initialize",
      params: {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: {
          name: "test-client",
          version: "1.0.0"
        }
      },
      id: 1
    })
  });

  console.log("Response status:", response.status);
  const text = await response.text();
  console.log("Response:", text);
  return text;
}

// Run tests
async function runTests() {
  try {
    const endpoint = await testSseConnect();
    if (endpoint) {
      await testSendMessage(endpoint);
    }
    console.log("\nAll tests passed!");
  } catch (e) {
    console.error("Test failed:", e);
  }
}

runTests();