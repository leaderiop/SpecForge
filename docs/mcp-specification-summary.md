# Model Context Protocol (MCP) - Comprehensive Specification Summary

**Protocol Version:** 2025-03-26 (current as of 2025-11-25)
**Source:** https://modelcontextprotocol.io/specification

## Executive Summary

MCP is an open protocol enabling seamless integration between LLM applications and external data sources and tools. Built on JSON-RPC 2.0, it provides a stateful session protocol for context exchange and sampling coordination between clients and servers.

**Key Insight:** MCP is inspired by Language Server Protocol (LSP) - standardizing AI context integration across tools like LSP standardized programming language support.

---

## 1. Core Architecture

### Components

```
┌─────────────────────────────────────────┐
│        Application Host Process         │
│  ┌──────┐                               │
│  │ Host │ (Orchestrator)                │
│  └───┬──┘                               │
│      ├─────► Client 1 ◄──► Server 1     │
│      ├─────► Client 2 ◄──► Server 2     │
│      └─────► Client 3 ◄──► Server 3     │
└─────────────────────────────────────────┘
```

**Host:**
- Creates and manages multiple client instances
- Controls client connection permissions and lifecycle
- Enforces security policies and consent requirements
- Handles user authorization decisions
- Coordinates AI/LLM integration and sampling
- Manages context aggregation across clients

**Clients:**
- One stateful session per server (1:1 relationship)
- Handles protocol negotiation and capability exchange
- Routes protocol messages bidirectionally
- Manages subscriptions and notifications
- Maintains security boundaries between servers

**Servers:**
- Expose resources, tools and prompts via MCP primitives
- Operate independently with focused responsibilities
- Can request sampling through client interfaces
- Must respect security constraints
- Can be local processes or remote services

### Design Principles

1. **Servers should be extremely easy to build** - hosts handle complexity
2. **Servers should be highly composable** - focused, isolated functionality
3. **Servers should not see the whole conversation or other servers** - isolation enforced
4. **Progressive feature addition** - capabilities negotiated, backwards compatible

---

## 2. Protocol Foundation

### Transport Layer

MCP uses **JSON-RPC 2.0** over two standard transports:

#### stdio (Standard Input/Output)
- Client launches server as subprocess
- Messages delimited by newlines (MUST NOT contain embedded newlines)
- Server reads from stdin, writes to stdout
- Server MAY write logs to stderr
- UTF-8 encoding required
- **Clients SHOULD support stdio whenever possible**

#### Streamable HTTP
- Server operates as independent process (multiple client connections)
- Single HTTP endpoint supporting POST and GET
- POST: Send messages to server (requests, notifications, responses)
- GET: Open SSE stream for server-to-client messages
- Session management via `Mcp-Session-Id` header
- Supports resumability with SSE event IDs and `Last-Event-ID` header
- JSON-RPC batching supported

**Security Warning for HTTP:**
- Servers MUST validate `Origin` header (prevent DNS rebinding)
- Local servers SHOULD bind to localhost only (127.0.0.1)
- Servers SHOULD implement proper authentication

#### Custom Transports
- Implementations MAY create custom transports
- MUST preserve JSON-RPC message format and lifecycle requirements

### Message Format

- All messages MUST be UTF-8 encoded JSON-RPC 2.0
- Supports: requests, responses, notifications, batches
- Messages can be sent bidirectionally

---

## 3. Connection Lifecycle

### Initialization Phase (REQUIRED)

```json
// Client → Server: initialize request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-03-26",
    "capabilities": {
      "roots": { "listChanged": true },
      "sampling": {}
    },
    "clientInfo": {
      "name": "ExampleClient",
      "version": "1.0.0"
    }
  }
}

// Server → Client: initialize response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-03-26",
    "capabilities": {
      "logging": {},
      "prompts": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true },
      "tools": { "listChanged": true }
    },
    "serverInfo": {
      "name": "ExampleServer",
      "version": "1.0.0"
    },
    "instructions": "Optional instructions for the client"
  }
}

// Client → Server: initialized notification
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

**Rules:**
- Initialize request MUST be first interaction
- Initialize request MUST NOT be in a JSON-RPC batch
- Client SHOULD NOT send other requests before initialize response (except pings)
- Server SHOULD NOT send other requests before initialized notification (except pings/logging)

### Version Negotiation

- Client sends latest supported protocol version
- Server responds with same version if supported, otherwise its latest version
- If client doesn't support server's version, client SHOULD disconnect

### Capability Negotiation

Both parties declare optional features during initialization:

| Category | Capability | Description |
|----------|-----------|-------------|
| **Client** | `roots` | Provide filesystem roots |
| | `sampling` | Support LLM sampling requests |
| | `experimental` | Non-standard experimental features |
| **Server** | `prompts` | Offer prompt templates |
| | `resources` | Provide readable resources |
| | `tools` | Expose callable tools |
| | `logging` | Emit structured log messages |
| | `completions` | Support argument autocompletion |
| | `experimental` | Non-standard experimental features |

Sub-capabilities:
- `listChanged`: Support for list change notifications (prompts, resources, tools)
- `subscribe`: Support for resource subscriptions

### Operation Phase

Normal message exchange according to negotiated capabilities.

### Shutdown Phase

- No specific shutdown messages defined
- Use underlying transport mechanism to signal termination
- **stdio:** Client closes input stream, waits for exit, SIGTERM, then SIGKILL if needed
- **HTTP:** Close HTTP connection(s)

### Timeouts

- Implementations SHOULD establish timeouts for all requests
- On timeout: send cancellation notification, stop waiting
- SDKs SHOULD allow per-request timeout configuration
- MAY reset timeout on progress notifications
- SHOULD enforce maximum timeout regardless of progress

---

## 4. Server Features (Server → Client)

### 4.1 Resources

**Purpose:** Share data/context with language models (files, database schemas, application data)

**User Model:** Application-driven (apps determine how to incorporate context)

**Capability Declaration:**
```json
{
  "capabilities": {
    "resources": {
      "subscribe": true,    // Optional: individual resource subscriptions
      "listChanged": true   // Optional: list change notifications
    }
  }
}
```

#### Resource URIs

**Unique identifiers** for resources. Standard schemes:
- `https://` - Web-accessible resources (client can fetch directly)
- `file://` - Filesystem-like resources (not necessarily physical files)
- `git://` - Git version control
- Custom schemes allowed

#### Operations

**List Resources** (`resources/list`)
```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "resources/list",
  "params": {
    "cursor": "optional-cursor-value"  // For pagination
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "resources": [
      {
        "uri": "file:///project/src/main.rs",
        "name": "main.rs",
        "description": "Primary application entry point",
        "mimeType": "text/x-rust"
      }
    ],
    "nextCursor": "next-page-cursor"
  }
}
```

**Read Resource** (`resources/read`)
```json
// Request
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "resources/read",
  "params": {
    "uri": "file:///project/src/main.rs"
  }
}

// Response (text content)
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "contents": [
      {
        "uri": "file:///project/src/main.rs",
        "mimeType": "text/x-rust",
        "text": "fn main() {\n    println!(\"Hello world!\");\n}"
      }
    ]
  }
}

// Response (binary content)
{
  "contents": [
    {
      "uri": "file:///example.png",
      "mimeType": "image/png",
      "blob": "base64-encoded-data"
    }
  ]
}
```

**List Resource Templates** (`resources/templates/list`)
- URI templates using RFC 6570 syntax
- Example: `file:///{path}` with parameterized path argument
- Arguments may be auto-completed via completion API

**Subscribe to Resource** (`resources/subscribe`)
```json
// Request
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "resources/subscribe",
  "params": {
    "uri": "file:///project/src/main.rs"
  }
}

// Update Notification (when resource changes)
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "file:///project/src/main.rs"
  }
}
```

**List Changed Notification** (`notifications/resources/list_changed`)
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/list_changed"
}
```

#### Special MIME Types
- XDG MIME types (e.g., `inode/directory`) for non-regular files

---

### 4.2 Tools

**Purpose:** Functions that LLMs can invoke to interact with external systems

**User Model:** Model-controlled (LLM discovers and invokes automatically)

**Capability Declaration:**
```json
{
  "capabilities": {
    "tools": {
      "listChanged": true  // Optional: list change notifications
    }
  }
}
```

**Critical Security Note:** SHOULD always have human in the loop with ability to deny tool invocations.

#### Tool Definition

```json
{
  "name": "get_weather",
  "description": "Get current weather information for a location",
  "inputSchema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City name or zip code"
      }
    },
    "required": ["location"]
  }
}
```

#### Operations

**List Tools** (`tools/list`)
```json
// Response includes tools array, supports pagination
{
  "tools": [...],
  "nextCursor": "optional"
}
```

**Call Tool** (`tools/call`)
```json
// Request
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_weather",
    "arguments": {
      "location": "New York"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Current weather in New York:\nTemperature: 72°F\nConditions: Partly cloudy"
      }
    ],
    "isError": false
  }
}
```

#### Tool Result Content Types

1. **Text:** `{"type": "text", "text": "..."}`
2. **Image:** `{"type": "image", "data": "base64...", "mimeType": "image/png"}`
3. **Audio:** `{"type": "audio", "data": "base64...", "mimeType": "audio/wav"}`
4. **Embedded Resource:** `{"type": "resource", "resource": {...}}`

#### Error Handling

**Protocol Errors** (JSON-RPC standard):
- Unknown tool
- Invalid arguments
- Server errors

**Tool Execution Errors** (in result with `isError: true`):
```json
{
  "content": [{"type": "text", "text": "Failed: API rate limit exceeded"}],
  "isError": true
}
```

**List Changed Notification** (`notifications/tools/list_changed`)

---

### 4.3 Prompts

**Purpose:** Structured message templates and instructions for LLM interaction

**User Model:** User-controlled (explicitly selected by users, e.g., slash commands)

**Capability Declaration:**
```json
{
  "capabilities": {
    "prompts": {
      "listChanged": true  // Optional: list change notifications
    }
  }
}
```

#### Prompt Definition

```json
{
  "name": "code_review",
  "description": "Asks the LLM to analyze code quality and suggest improvements",
  "arguments": [
    {
      "name": "code",
      "description": "The code to review",
      "required": true
    }
  ]
}
```

#### Operations

**List Prompts** (`prompts/list`)
- Returns array of available prompts
- Supports pagination

**Get Prompt** (`prompts/get`)
```json
// Request
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": {
      "code": "def hello():\n    print('world')"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "description": "Code review prompt",
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Please review this Python code:\ndef hello():\n    print('world')"
        }
      }
    ]
  }
}
```

#### Prompt Message Content Types

1. **Text:** `{"type": "text", "text": "..."}`
2. **Image:** `{"type": "image", "data": "base64...", "mimeType": "image/png"}`
3. **Audio:** `{"type": "audio", "data": "base64...", "mimeType": "audio/wav"}`
4. **Embedded Resource:** `{"type": "resource", "resource": {...}}`

Message roles: `user` or `assistant`

**List Changed Notification** (`notifications/prompts/list_changed`)

---

## 5. Client Features (Client → Server)

### 5.1 Sampling

**Purpose:** Servers can request LLM completions/generations from client

**User Model:** Enables agentic behaviors, nested LLM calls inside server features

**Capability Declaration:**
```json
{
  "capabilities": {
    "sampling": {}
  }
}
```

**Critical Security Note:** SHOULD always have human in the loop to review/approve sampling requests.

#### Sampling Request

```json
// Request (Server → Client)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "sampling/createMessage",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "What is the capital of France?"
        }
      }
    ],
    "modelPreferences": {
      "hints": [
        {"name": "claude-3-sonnet"}
      ],
      "intelligencePriority": 0.8,
      "speedPriority": 0.5,
      "costPriority": 0.3
    },
    "systemPrompt": "You are a helpful assistant.",
    "maxTokens": 100
  }
}

// Response (Client → Server)
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "role": "assistant",
    "content": {
      "type": "text",
      "text": "The capital of France is Paris."
    },
    "model": "claude-3-sonnet-20240307",
    "stopReason": "endTurn"
  }
}
```

#### Model Preferences

**Capability Priorities** (0-1 normalized):
- `costPriority`: Importance of minimizing costs
- `speedPriority`: Importance of low latency
- `intelligencePriority`: Importance of advanced capabilities

**Model Hints:**
- Advisory suggestions (substrings matching model names)
- Ordered by preference
- Clients MAY map to equivalent models from different providers
- Example: `{"name": "claude-3-sonnet"}` might map to `gemini-1.5-pro`

#### Message Content Types

Same as prompts: text, image, audio

#### Workflow

1. Server requests sampling from client
2. Client presents request to user for approval
3. User reviews/modifies prompt
4. Client forwards to LLM
5. LLM returns generation
6. Client presents response to user for review
7. Client returns approved response to server

---

### 5.2 Roots

**Purpose:** Clients expose filesystem "roots" defining server access boundaries

**User Model:** Workspace/project configuration (directory/file pickers)

**Capability Declaration:**
```json
{
  "capabilities": {
    "roots": {
      "listChanged": true  // Optional: list change notifications
    }
  }
}
```

#### Operations

**List Roots** (`roots/list`)
```json
// Request (Server → Client)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "roots/list"
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "roots": [
      {
        "uri": "file:///home/user/projects/myproject",
        "name": "My Project"
      }
    ]
  }
}
```

**Root URIs:** MUST be `file://` URIs in current specification

**List Changed Notification** (`notifications/roots/list_changed`)

#### Security Considerations

- Clients MUST only expose roots with appropriate permissions
- Clients MUST validate URIs to prevent path traversal
- Servers SHOULD respect root boundaries during operations

---

## 6. Utility Features

### 6.1 Pagination

**Cursor-based pagination** for list operations (not numbered pages)

**Operations Supporting Pagination:**
- `resources/list`
- `resources/templates/list`
- `prompts/list`
- `tools/list`

**Pattern:**
```json
// First request (no cursor)
{
  "method": "resources/list",
  "params": {}
}

// Response with cursor
{
  "result": {
    "resources": [...],
    "nextCursor": "eyJwYWdlIjogM30="
  }
}

// Next request (with cursor)
{
  "method": "resources/list",
  "params": {
    "cursor": "eyJwYWdlIjogM30="
  }
}
```

**Rules:**
- Cursors are opaque strings (clients MUST NOT parse/modify)
- Page size is server-determined
- Missing `nextCursor` = end of results
- Cursors MUST NOT be persisted across sessions

---

### 6.2 Completion

**Purpose:** Argument auto-completion for prompts and resource templates

**Capability Declaration:**
```json
{
  "capabilities": {
    "completions": {}
  }
}
```

#### Completion Request

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "completion/complete",
  "params": {
    "ref": {
      "type": "ref/prompt",
      "name": "code_review"
    },
    "argument": {
      "name": "language",
      "value": "py"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "completion": {
      "values": ["python", "pytorch", "pyside"],
      "total": 10,
      "hasMore": true
    }
  }
}
```

**Reference Types:**
- `ref/prompt`: References a prompt by name
- `ref/resource`: References a resource URI (template)

**Completion Results:**
- Maximum 100 items per response
- Ranked by relevance
- Optional total count
- `hasMore` flag

---

### 6.3 Logging

**Purpose:** Servers emit structured log messages to clients

**Capability Declaration:**
```json
{
  "capabilities": {
    "logging": {}
  }
}
```

#### Log Levels (RFC 5424 syslog)

| Level | Description | Example |
|-------|-------------|---------|
| debug | Detailed debugging | Function entry/exit |
| info | Informational | Operation progress |
| notice | Significant events | Configuration changes |
| warning | Warning conditions | Deprecated feature usage |
| error | Error conditions | Operation failures |
| critical | Critical conditions | System component failures |
| alert | Action required immediately | Data corruption |
| emergency | System unusable | Complete system failure |

#### Operations

**Set Log Level** (`logging/setLevel`)
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "logging/setLevel",
  "params": {
    "level": "info"
  }
}
```

**Log Message Notification** (`notifications/message`)
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/message",
  "params": {
    "level": "error",
    "logger": "database",
    "data": {
      "error": "Connection failed",
      "details": {
        "host": "localhost",
        "port": 5432
      }
    }
  }
}
```

#### Security

Log messages MUST NOT contain:
- Credentials or secrets
- Personal identifying information
- Internal system details aiding attacks

---

### 6.4 Progress Notifications

**Purpose:** Report progress updates for long-running operations

#### Pattern

1. Requester includes `progressToken` in request metadata:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "some_method",
  "params": {
    "_meta": {
      "progressToken": "abc123"
    }
  }
}
```

2. Receiver sends progress notifications:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": "abc123",
    "progress": 50,
    "total": 100,
    "message": "Reticulating splines..."
  }
}
```

**Rules:**
- Progress tokens MUST be string or integer, unique across active requests
- `progress` value MUST increase with each notification
- `progress` and `total` MAY be floating point
- Receiver MAY choose not to send progress notifications
- Progress MUST stop after operation completes

---

### 6.5 Cancellation

**Purpose:** Cancel in-progress requests

#### Cancellation Notification

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/cancelled",
  "params": {
    "requestId": "123",
    "reason": "User requested cancellation"
  }
}
```

**Rules:**
- MUST only reference requests that are believed in-progress
- `initialize` request MUST NOT be cancelled by clients
- Receivers SHOULD stop processing and free resources
- Receivers SHOULD NOT send response for cancelled request
- Receivers MAY ignore cancellation if already completed or uncancellable
- Sender SHOULD ignore any response arriving after cancellation

**Race Conditions:** Due to network latency, cancellation may arrive after completion. Both parties MUST handle gracefully.

---

### 6.6 Ping

**Purpose:** Keepalive and connection health check

#### Ping Request

```json
// Request (either direction)
{
  "jsonrpc": "2.0",
  "id": "123",
  "method": "ping"
}

// Response
{
  "jsonrpc": "2.0",
  "id": "123",
  "result": {}
}
```

**Rules:**
- Receiver MUST respond promptly with empty response
- If no response within reasonable timeout, sender MAY consider connection stale
- Implementations SHOULD periodically issue pings
- Ping frequency SHOULD be configurable
- Excessive pinging SHOULD be avoided

---

## 7. Authorization (HTTP Transports)

**Note:** Authorization is **OPTIONAL**. When supported on HTTP transports, implementations SHOULD follow this spec.

### OAuth 2.1 Based Flow

1. MCP auth implementations MUST implement OAuth 2.1 with appropriate security
2. Implementations SHOULD support OAuth 2.0 Dynamic Client Registration (RFC 7591)
3. Servers SHOULD and clients MUST implement OAuth 2.0 Authorization Server Metadata (RFC 8414)

### Supported Grant Types

- **Authorization Code:** User-initiated flows (human end user authorization)
- **Client Credentials:** Application-to-application (no human user)

### Authorization Base URL

Computed by discarding path component from MCP server URL:
- MCP server: `https://api.example.com/v1/mcp`
- Authorization base: `https://api.example.com`
- Metadata endpoint: `https://api.example.com/.well-known/oauth-authorization-server`

### Metadata Discovery

1. Client sends `GET /.well-known/oauth-authorization-server`
2. May include `MCP-Protocol-Version: <protocol-version>` header
3. If metadata not available, fallback to default endpoints:
   - `/authorize` (authorization endpoint)
   - `/token` (token endpoint)
   - `/register` (registration endpoint)

### Dynamic Client Registration

Flow enables clients to automatically obtain OAuth credentials without manual user registration.

### Access Token Usage

1. Clients MUST use `Authorization: Bearer <access-token>` header
2. Access tokens MUST NOT be in URI query string
3. Authorization MUST be included in every HTTP request
4. Invalid/expired tokens MUST receive HTTP 401

### Security Requirements

- Clients MUST securely store tokens
- Servers SHOULD enforce token expiration and rotation
- All endpoints MUST use HTTPS
- Servers MUST validate redirect URIs
- Redirect URIs MUST be localhost URLs or HTTPS URLs
- PKCE REQUIRED for all clients

### Third-Party Authorization

Servers MAY support delegated authorization:
1. MCP client initiates OAuth with MCP server
2. MCP server redirects to third-party auth server
3. User authorizes with third-party
4. Third-party redirects back to MCP server
5. MCP server exchanges code for third-party token
6. MCP server generates bound MCP token
7. MCP server completes OAuth flow with MCP client

---

## 8. Error Handling

### Standard JSON-RPC Error Codes

| Code | Message | Usage |
|------|---------|-------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Invalid JSON-RPC |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid parameters |
| -32603 | Internal error | Server internal error |
| -32002 | (MCP) | Resource not found |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Unknown tool: invalid_tool_name",
    "data": {
      "additional": "context"
    }
  }
}
```

---

## 9. Security and Trust Considerations

### Key Principles

1. **User Consent and Control**
   - Users must explicitly consent to data access and operations
   - Users must understand and control what is shared and executed
   - Clear UIs for reviewing and authorizing activities

2. **Data Privacy**
   - Explicit user consent before exposing user data to servers
   - User data must not be transmitted elsewhere without consent
   - Appropriate access controls

3. **Tool Safety**
   - Tools represent arbitrary code execution
   - Tool descriptions/annotations should be treated as untrusted (unless from trusted server)
   - Explicit user consent before tool invocation
   - Users should understand tool behavior before authorization

4. **LLM Sampling Controls**
   - Users must explicitly approve sampling requests
   - Users should control: whether sampling occurs, the actual prompt, visible results
   - Protocol intentionally limits server visibility into prompts

### Implementation Guidelines

Implementors SHOULD:
- Build robust consent and authorization flows
- Provide clear security documentation
- Implement appropriate access controls
- Follow security best practices
- Consider privacy implications

---

## 10. JSON-RPC Specifics

### Message Types

1. **Request:** Has `id`, `method`, `params`
2. **Response:** Has `id`, `result` or `error`
3. **Notification:** Has `method`, `params`, NO `id`
4. **Batch:** Array of requests/notifications/responses

### Batching

- JSON-RPC batches supported (array of messages)
- Initialize request MUST NOT be in a batch
- Multiple requests can be batched for efficiency

### Request Metadata

Special `_meta` field in params:
```json
{
  "params": {
    "_meta": {
      "progressToken": "abc123"
    }
    // ... normal params
  }
}
```

---

## 11. All MCP Primitives Summary

### Core Primitives (Exposed by Servers)

1. **Resources** - Data and context
2. **Tools** - Executable functions
3. **Prompts** - Message templates

### Client Capabilities

4. **Sampling** - LLM completion requests
5. **Roots** - Filesystem boundaries

### Utility Operations

6. **Pagination** - Cursor-based result paging
7. **Completion** - Argument auto-completion
8. **Logging** - Structured log messages
9. **Progress** - Long-running operation updates
10. **Cancellation** - Request cancellation
11. **Ping** - Connection health check
12. **Authorization** - OAuth 2.1 authentication (HTTP)

### Lifecycle & Protocol

13. **Initialization** - Protocol negotiation
14. **Capability Negotiation** - Feature discovery
15. **Subscriptions** - Resource change notifications
16. **List Change Notifications** - Dynamic list updates

### Notifications

All notifications use `notifications/<category>/<event>` pattern:
- `notifications/initialized`
- `notifications/cancelled`
- `notifications/progress`
- `notifications/message` (logging)
- `notifications/resources/updated`
- `notifications/resources/list_changed`
- `notifications/tools/list_changed`
- `notifications/prompts/list_changed`
- `notifications/roots/list_changed`

---

## 12. Capability Declaration Matrix

| Feature | Capability Key | Sub-Capabilities | Direction |
|---------|---------------|------------------|-----------|
| Resources | `resources` | `subscribe`, `listChanged` | Server |
| Tools | `tools` | `listChanged` | Server |
| Prompts | `prompts` | `listChanged` | Server |
| Logging | `logging` | - | Server |
| Completions | `completions` | - | Server |
| Sampling | `sampling` | - | Client |
| Roots | `roots` | `listChanged` | Client |
| Experimental | `experimental` | (custom) | Both |

---

## 13. Content Type Summary

All content types supported across resources, tools, prompts, and sampling:

| Type | Format | Usage |
|------|--------|-------|
| Text | `{"type": "text", "text": "..."}` | Plain text messages |
| Image | `{"type": "image", "data": "base64...", "mimeType": "..."}` | Visual content |
| Audio | `{"type": "audio", "data": "base64...", "mimeType": "..."}` | Audio content |
| Resource | `{"type": "resource", "resource": {...}}` | Embedded resource references |

For resources specifically:
- Text: `{uri, mimeType, text}`
- Binary: `{uri, mimeType, blob}` (base64)

---

## 14. URI Schemes

| Scheme | Purpose | Notes |
|--------|---------|-------|
| `https://` | Web resources | Client can fetch directly |
| `file://` | Filesystem-like | Not necessarily physical files |
| `git://` | Git version control | - |
| Custom | Implementation-specific | Allowed |

---

## 15. Implementation Checklist

### For Clients

- [ ] Support stdio transport (SHOULD)
- [ ] Implement initialization flow
- [ ] Negotiate capabilities
- [ ] Handle version negotiation
- [ ] Support timeouts for all requests
- [ ] Implement cancellation handling
- [ ] Support progress notifications
- [ ] Implement ping for keepalive
- [ ] Human-in-the-loop for tools (SHOULD)
- [ ] Human-in-the-loop for sampling (SHOULD)
- [ ] OAuth 2.1 authorization (if HTTP)
- [ ] Metadata discovery (MUST for HTTP)
- [ ] Dynamic client registration (SHOULD)

### For Servers

- [ ] Support stdio transport (SHOULD)
- [ ] Respond to initialization
- [ ] Declare capabilities accurately
- [ ] Handle version negotiation
- [ ] Implement subscriptions (if declared)
- [ ] Send list_changed notifications (if declared)
- [ ] Validate all inputs
- [ ] Implement appropriate error handling
- [ ] Support pagination for list operations
- [ ] Respect cancellation requests
- [ ] Send progress notifications (if requested)
- [ ] Respond to ping requests
- [ ] OAuth 2.1 authorization (if HTTP)
- [ ] Metadata discovery endpoint (SHOULD for HTTP)
- [ ] Dynamic client registration (SHOULD)

---

## 16. Key Differences from Other Protocols

**vs Language Server Protocol (LSP):**
- MCP: AI context integration, stateful sessions, bidirectional requests
- LSP: Programming language tooling, document-centric, mostly client → server

**vs HTTP REST APIs:**
- MCP: Stateful sessions, capability negotiation, bidirectional
- REST: Stateless, fixed endpoints, client → server

**vs WebSocket:**
- MCP: JSON-RPC over transport (stdio or HTTP+SSE), structured protocol
- WebSocket: Transport only, no protocol defined

---

## 17. Version History

**Current:** 2025-03-26 (as of 2025-11-25)

**Versioning:** `YYYY-MM-DD` format (date of last backwards-incompatible change)

**Changes from 2024-11-05 → 2025-03-26:**
- Replaced HTTP+SSE transport with Streamable HTTP transport
- Added session management with `Mcp-Session-Id` header
- Enhanced SSE resumability with event IDs
- Added GET endpoint for server-initiated SSE streams
- Backward compatibility guidance provided

---

## 18. Resources and References

**Official:**
- Specification: https://modelcontextprotocol.io/specification
- Documentation: https://modelcontextprotocol.io
- TypeScript Schema: https://github.com/modelcontextprotocol/specification/blob/main/schema/2025-03-26/schema.ts

**Standards:**
- JSON-RPC 2.0: https://www.jsonrpc.org/
- OAuth 2.1 Draft: https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-1-12
- RFC 8414: OAuth Authorization Server Metadata
- RFC 7591: OAuth Dynamic Client Registration
- RFC 6570: URI Templates
- RFC 5424: Syslog severity levels
- RFC 3986: URI Generic Syntax

**Inspiration:**
- Language Server Protocol (LSP): https://microsoft.github.io/language-server-protocol/

---

## Appendix A: Complete Message Catalog

### Requests (Method Names)

| Method | Direction | Purpose | Pagination |
|--------|-----------|---------|------------|
| `initialize` | Client → Server | Protocol initialization | No |
| `ping` | Either | Connection health | No |
| `resources/list` | Client → Server | List resources | Yes |
| `resources/read` | Client → Server | Read resource content | No |
| `resources/templates/list` | Client → Server | List resource templates | Yes |
| `resources/subscribe` | Client → Server | Subscribe to resource | No |
| `tools/list` | Client → Server | List tools | Yes |
| `tools/call` | Client → Server | Invoke tool | No |
| `prompts/list` | Client → Server | List prompts | Yes |
| `prompts/get` | Client → Server | Get prompt content | No |
| `completion/complete` | Client → Server | Get completions | No |
| `logging/setLevel` | Client → Server | Set log level | No |
| `sampling/createMessage` | Server → Client | Request LLM sampling | No |
| `roots/list` | Server → Client | List filesystem roots | No |

### Notifications (Method Names)

| Method | Direction | Purpose |
|--------|-----------|---------|
| `notifications/initialized` | Client → Server | Initialization complete |
| `notifications/cancelled` | Either | Cancel request |
| `notifications/progress` | Either | Progress update |
| `notifications/message` | Server → Client | Log message |
| `notifications/resources/updated` | Server → Client | Resource changed |
| `notifications/resources/list_changed` | Server → Client | Resource list changed |
| `notifications/tools/list_changed` | Server → Client | Tool list changed |
| `notifications/prompts/list_changed` | Server → Client | Prompt list changed |
| `notifications/roots/list_changed` | Client → Server | Roots list changed |

---

## Appendix B: Example Full Session Flow

```json
// 1. Client → Server: Initialize
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-03-26",
    "capabilities": {"roots": {}, "sampling": {}},
    "clientInfo": {"name": "MyClient", "version": "1.0.0"}
  }
}

// 2. Server → Client: Initialize Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-03-26",
    "capabilities": {
      "resources": {"subscribe": true, "listChanged": true},
      "tools": {"listChanged": true}
    },
    "serverInfo": {"name": "MyServer", "version": "1.0.0"}
  }
}

// 3. Client → Server: Initialized Notification
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}

// 4. Client → Server: List Resources
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "resources/list"
}

// 5. Server → Client: Resources Response
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "resources": [
      {"uri": "file:///project/README.md", "name": "README"}
    ]
  }
}

// 6. Client → Server: Read Resource
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "resources/read",
  "params": {"uri": "file:///project/README.md"}
}

// 7. Server → Client: Resource Content
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "contents": [{
      "uri": "file:///project/README.md",
      "mimeType": "text/markdown",
      "text": "# My Project\n..."
    }]
  }
}

// 8. Server → Client: Sampling Request
{
  "jsonrpc": "2.0",
  "id": "srv-1",
  "method": "sampling/createMessage",
  "params": {
    "messages": [{"role": "user", "content": {"type": "text", "text": "Summarize this project"}}]
  }
}

// 9. Client → Server: Sampling Response
{
  "jsonrpc": "2.0",
  "id": "srv-1",
  "result": {
    "role": "assistant",
    "content": {"type": "text", "text": "This project is..."},
    "model": "claude-3-sonnet-20240307",
    "stopReason": "endTurn"
  }
}

// 10. Server → Client: Resource Updated Notification
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {"uri": "file:///project/README.md"}
}

// 11. Client disconnects (transport-specific)
```

---

**End of Specification Summary**

*Generated: 2026-03-04*
*MCP Protocol Version: 2025-03-26*
*Source: https://modelcontextprotocol.io/specification*
