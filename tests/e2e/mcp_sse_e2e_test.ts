/**
 * MCP SSE E2E Tests using Deno TS with MCP SDK
 *
 * NOTE: The MCP SDK's SSE transport uses the `eventsource` npm package
 * which has compatibility issues in Deno. This test file documents the
 * expected behavior and can be run in Node.js or browser environments.
 *
 * MCP server must be running separately:
 *   .\start-http-server.ps1 -Port 8080
 *
 * For SSE testing, use the manual SSE test approach or run in Node.js:
 *   node --experimental-vm-modules node_modules/.bin/deno test ...
 */

import { Client } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client";
import { SSEClientTransport } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client/sse.js";

const SSE_SERVER_URL = "http://127.0.0.1:8080/sse";

async function createSseClient() {
  const client = new Client({ name: "test-client-sse", version: "1.0.0" });
  const transport = new SSEClientTransport(SSE_SERVER_URL);
  await client.connect(transport);
  return client;
}

async function withSseClient<T>(fn: (client: Client) => Promise<T>): Promise<T> {
  const client = await createSseClient();
  try {
    return await fn(client);
  } finally {
    await client.close();
  }
}

Deno.test({
  name: "MCP SSE - Health Check",
  async fn() {
    await withSseClient(async (client) => {
      const result = await client.callTool({
        name: "health_check",
        arguments: {},
      });

      if (result.isError) throw new Error("Health check returned error");
      const text = (result.content[0] as { text?: string }).text!;
      const parsed = JSON.parse(text);
      if (parsed.status !== "OK") throw new Error(`Health check failed: ${text}`);
    });
  },
});

Deno.test({
  name: "MCP SSE - List Tools",
  async fn() {
    await withSseClient(async (client) => {
      const { tools } = await client.listTools();
      const toolNames = tools.map((t: { name: string }) => t.name);
      if (!toolNames.includes("health_check")) throw new Error("Missing health_check");
      if (!toolNames.includes("skills_search")) throw new Error("Missing skills_search");
      if (!toolNames.includes("skills_list")) throw new Error("Missing skills_list");
    });
  },
});

Deno.test({
  name: "MCP SSE - Skills Create and Get",
  async fn() {
    await withSseClient(async (client) => {
      const createResult = await client.callTool({
        name: "skills_create",
        arguments: {
          name: "test-skill-sse",
          description: "A test skill created by SSE E2E test",
          tags: ["test", "sse"],
          content: "# Test Skill\n\nThis is a test skill for E2E testing with SSE transport.",
          version: "1.0.0",
        },
      });

      if (createResult.isError) throw new Error("Create returned error");
      const createText = (createResult.content[0] as { text?: string }).text!;
      const parsedCreate = JSON.parse(createText);
      const skillId = parsedCreate.id;

      const infoResult = await client.callTool({
        name: "skills_info",
        arguments: { skill_id: skillId },
      });

      if (infoResult.isError) throw new Error("Info returned error");
      const infoText = (infoResult.content[0] as { text?: string }).text!;
      const parsedInfo = JSON.parse(infoText);

      if (parsedInfo.name !== "test-skill-sse") throw new Error(`Wrong name: ${parsedInfo.name}`);
    });
  },
});

Deno.test({
  name: "MCP SSE - Skills Search",
  async fn() {
    await withSseClient(async (client) => {
      const uniqueName = `sse-searchable-${Date.now()}`;
      const createResult = await client.callTool({
        name: "skills_create",
        arguments: {
          name: uniqueName,
          description: "This skill is searchable via SSE transport",
          tags: ["search", "sse"],
          content: "# SSE Searchable Skill\n\nContent that should be searchable via MCP SSE transport.",
          version: "1.0.0",
        },
      });

      if (createResult.isError) throw new Error("Create returned error");

      const searchResult = await client.callTool({
        name: "skills_search",
        arguments: { query: "SSE Searchable", limit: 10 },
      });

      if (searchResult.isError) throw new Error("Search returned error");
      const searchText = (searchResult.content[0] as { text?: string }).text!;
      const searchParsed = JSON.parse(searchText);

      if (!Array.isArray(searchParsed) || searchParsed.length === 0) {
        throw new Error("No search results found");
      }
    });
  },
});

Deno.test({
  name: "MCP SSE - Multiple Requests in Session",
  async fn() {
    await withSseClient(async (client) => {
      const result1 = await client.callTool({
        name: "health_check",
        arguments: {},
      });
      if (result1.isError) throw new Error("First health check failed");

      const result2 = await client.callTool({
        name: "health_check",
        arguments: {},
      });
      if (result2.isError) throw new Error("Second health check failed");

      const result3 = await client.callTool({
        name: "health_check",
        arguments: {},
      });
      if (result3.isError) throw new Error("Third health check failed");
    });
  },
});

console.log(`
========================================
MCP SSE E2E Tests (using MCP SDK)
========================================
NOTE: Requires Node.js or browser environment
due to EventSource API compatibility in Deno.

Start MCP server first:
  .\start-http-server.ps1 -Port 8080

Run tests (Node.js):
  node test-mcp-sse.js

Or run HTTP tests which work in Deno:
  deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts
========================================
`);