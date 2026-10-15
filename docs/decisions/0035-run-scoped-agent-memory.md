# ADR0035 Run-scoped agent memory

Status: Accepted for approved Task0034

Date:2026-10-15

Expose current accepted Local decisions through a read-only project/run binding,
using the existing Host and writer. MCP transports data and grants no authority
to select another project or accept a decision. Context is evidence, not new
system permissions. A V6 context-bound fact records the starting manifest
revision; credentials and tool queries remain outside canonical facts.

Use one lazy literal-loopback read service and an existing-executable stdio
adapter. A protected short-lived one-shot ticket keeps credentials out of
provider configuration, arguments and prompts. RAM leases are scoped and
revocable; provider terminal/cancel/discard closes their authority synchronously.
Every tool read revalidates live scope and current decision status. Exact reads
omit raw source bodies and origin records. Local remains account-free.

Enable attachment only on the supported provider surface verified by a real
no-inference MCP handshake. Leave external provider settings untouched. Existing
Cloud memory access keeps its own paid authorization and explicit query/upload
boundary. Agent Cloud attachment and automatic ADR drafts are later slices.

References: [task](../plans/0034-scoped-agent-memory.md),
[MCP stdio](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[Codex app server](https://learn.chatgpt.com/docs/app-server),
[Codex MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).
