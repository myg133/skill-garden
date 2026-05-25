/**
 * MCP E2E Tests using Deno TS with MCP SDK
 *
 * MCP server must be running separately:
 *   .\start-http-server.ps1
 *
 * Run tests:
 *   deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts
 */

import { Client } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client";
import { StreamableHTTPClientTransport } from "https://esm.run/@modelcontextprotocol/sdk@1.29.0/client/streamableHttp.js";

const MCP_SERVER_URL = "http://127.0.0.1:8080/mcp";

async function createClient() {
  const client = new Client({ name: "test-client", version: "1.0.0" });
  const transport = new StreamableHTTPClientTransport(MCP_SERVER_URL);
  await client.connect(transport);
  return client;
}

async function withClient<T>(fn: (client: Client) => Promise<T>): Promise<T> {
  const client = await createClient();
  try {
    return await fn(client);
  } finally {
    await client.close();
  }
}

Deno.test({
  name: "MCP E2E - Health Check",
  async fn() {
    await withClient(async (client) => {
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
  name: "MCP E2E - List Tools",
  async fn() {
    await withClient(async (client) => {
      const { tools } = await client.listTools();
      const toolNames = tools.map((t: { name: string }) => t.name);
      if (!toolNames.includes("health_check")) throw new Error("Missing health_check");
      if (!toolNames.includes("skills_search")) throw new Error("Missing skills_search");
      if (!toolNames.includes("skills_list")) throw new Error("Missing skills_list");
      if (!toolNames.includes("skills_create")) throw new Error("Missing skills_create");
    });
  },
});

Deno.test({
  name: "MCP E2E - Skills Create and Get",
  async fn() {
    await withClient(async (client) => {
      const createResult = await client.callTool({
        name: "skills_create",
        arguments: {
          name: "test-skill-sdk",
          description: "A test skill created by E2E test with SDK",
          tags: ["test", "sdk"],
          content: "# Test Skill\n\nThis is a test skill for E2E testing with MCP SDK.",
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

      if (parsedInfo.name !== "test-skill-sdk") throw new Error(`Wrong name: ${parsedInfo.name}`);
    });
  },
});

Deno.test({
  name: "MCP E2E - Skills Search",
  async fn() {
    await withClient(async (client) => {
      const uniqueName = `searchable-skill-sdk-${Date.now()}`;
      const createResult = await client.callTool({
        name: "skills_create",
        arguments: {
          name: uniqueName,
          description: "This skill is searchable via SDK",
          tags: ["search", "sdk"],
          content: "# Searchable Skill\n\nContent that should be searchable via MCP SDK.",
          version: "1.0.0",
        },
      });

      if (createResult.isError) throw new Error("Create returned error");

      const searchResult = await client.callTool({
        name: "skills_search",
        arguments: { query: "searchable", limit: 10 },
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
  name: "MCP E2E - Skills Delete",
  async fn() {
    await withClient(async (client) => {
      const createResult = await client.callTool({
        name: "skills_create",
        arguments: {
          name: `delete-test-skill-sdk-${Date.now()}`,
          description: "Will be deleted",
          tags: ["test", "sdk"],
          content: "# To Delete",
          version: "1.0.0",
        },
      });

      if (createResult.isError) throw new Error("Create returned error");
      const skillId = JSON.parse((createResult.content[0] as { text?: string }).text!).id;

      const deleteResult = await client.callTool({
        name: "skills_delete",
        arguments: { skill_id: skillId },
      });

      if (deleteResult.isError) throw new Error("Delete returned error");
      const deleteParsed = JSON.parse((deleteResult.content[0] as { text?: string }).text!);
      if (deleteParsed.deleted !== skillId) throw new Error("Delete returned wrong id");

      let gotError = false;
      try {
        await client.callTool({
          name: "skills_info",
          arguments: { skill_id: skillId },
        });
      } catch (e) {
        gotError = true;
      }
      if (!gotError) throw new Error("Expected error when getting deleted skill");
    });
  },
});

Deno.test({
  name: "MCP E2E - Unknown Tool Returns Error",
  async fn() {
    await withClient(async (client) => {
      let gotError = false;
      try {
        await client.callTool({
          name: "nonexistent_tool",
          arguments: {},
        });
      } catch (e) {
        gotError = true;
      }
      if (!gotError) throw new Error("Expected error for unknown tool");
    });
  },
});

console.log(`
========================================
MCP E2E Tests (using MCP SDK)
========================================
Start MCP server first:
  .\start-http-server.ps1

Run tests:
  deno test --allow-all --filter "MCP E2E" tests/e2e/mcp_e2e_test.ts
========================================
`);