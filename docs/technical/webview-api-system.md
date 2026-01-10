# Webview API System (dashApp)

HTTP-based API for webview applications to communicate with the Rust backend, providing access to AWS resources, page management, and data operations.

## Overview

The dashApp API enables browser-based pages (built by agents or developers) to interact with AWS Dash's Rust infrastructure. Pages call JavaScript methods like `dashApp.listAccounts()` which translate to HTTP requests handled by an Axum API server in the main process.

**Key benefits:**
- Pages access the shared AWS client and resource cache
- Secure token-based authentication prevents unauthorized access
- Custom `wry://localhost` protocol enables proper CORS handling
- Automatic console.log interception writes to per-page log files

## How to Use

### Basic API Calls

```javascript
// List all configured AWS accounts
const accounts = await dashApp.listAccounts();
console.log(`Found ${accounts.length} accounts`);

// Query Lambda functions from cache
const result = await dashApp.queryCachedResources({
  resourceTypes: ['AWS::Lambda::Function']
});
result.resources.forEach(fn => {
  console.log(`${fn.displayName} in ${fn.region}`);
});
```

### Loading and Querying Resources

```javascript
// Step 1: Load resources into cache
const loadResult = await dashApp.loadCache({
  accounts: null,  // All accounts
  regions: ['us-east-1', 'us-west-2'],
  resourceTypes: ['AWS::EC2::Instance', 'AWS::Lambda::Function']
});
console.log(`Loaded ${loadResult.totalCount} resources`);

// Step 2: Query from cache
const instances = await dashApp.queryCachedResources({
  resourceTypes: ['AWS::EC2::Instance']
});
```

### CloudWatch Logs

```javascript
const logs = await dashApp.queryCloudWatchLogEvents({
  logGroupName: '/aws/lambda/my-function',
  accountId: '123456789012',
  region: 'us-east-1',
  filterPattern: 'ERROR',
  startTime: Date.now() - (60 * 60 * 1000), // Last hour
  limit: 100
});
logs.events.forEach(e => console.log(e.message));
```

### Page Management

```javascript
// List all pages
const pages = await dashApp.listPages();

// Open a page in new window
await dashApp.viewPage('my-dashboard');

// Delete a page
await dashApp.deletePage('old-dashboard');
```

## How it Works

### Architecture

```
Webview Process              Main Process
┌──────────────┐            ┌──────────────────────────┐
│ JavaScript   │  HTTP POST │ API Server (Axum)        │
│ dashApp.xxx()├───────────>│ validate X-API-Token     │
└──────────────┘            │ execute with             │
                            │ GLOBAL_AWS_CLIENT        │
                            │ GLOBAL_EXPLORER_STATE    │
                            └──────────────────────────┘
```

### Security Model

1. **Token Generation**: On startup, the API server generates a random 64-character hex token
2. **Token Injection**: The token is passed to webviews via `window.__DASH_API_TOKEN__`
3. **Request Validation**: Every API request must include `X-API-Token` header
4. **Origin Handling**: Custom `wry://localhost` protocol provides consistent origin for CORS

### Request Flow

1. JavaScript calls `dashApp.listAccounts()`
2. `invoke('listAccounts', {})` makes HTTP POST to `http://127.0.0.1:{port}/api/command`
3. API server validates token from `X-API-Token` header
4. `execute_command()` routes to appropriate handler
5. Handler uses `GLOBAL_AWS_CLIENT` and `GLOBAL_EXPLORER_STATE`
6. Result returned as JSON response

### Console Log Interception

Pages automatically log to `~/.local/share/awsdash/pages/{page-name}/page.log`:

```javascript
// This writes to both browser console AND page.log file
console.log('Fetched 42 resources');

// Access original console.log if needed
console.__original_log('Debug only - not to file');
```

## API Reference

### Account and Region

| Method | Description |
|--------|-------------|
| `listAccounts()` | Get all configured AWS accounts with id, name, alias, email |
| `listRegions()` | Get all AWS regions with code and name |

### Resource Operations

| Method | Description |
|--------|-------------|
| `loadCache(options)` | Query AWS and populate resource cache |
| `queryCachedResources(options)` | Get resources from cache |
| `getResourceSchema(resourceType)` | Get example resource structure |
| `showInExplorer(config)` | Open Explorer with specified configuration |

### Data Plane

| Method | Description |
|--------|-------------|
| `queryCloudWatchLogEvents(params)` | Query CloudWatch Logs events |
| `getCloudTrailEvents(params)` | Get CloudTrail audit events |

### Page Management

| Method | Description |
|--------|-------------|
| `listPages()` | List all pages with metadata |
| `viewPage(pageName)` | Open page in new window |
| `deletePage(pageName)` | Delete page and all files |
| `renamePage(oldName, newName)` | Rename page folder and update URLs |
| `editPage(pageName)` | Open Agent Manager to edit page |

### Bookmarks

| Method | Description |
|--------|-------------|
| `listBookmarks()` | Get all saved bookmarks |
| `queryBookmarks(id, options)` | Execute a bookmark's query |

## Integration Points

### Custom Protocol Handler

Files served via `wry://localhost/pages/{page-name}/` protocol:

```rust
// In custom_protocol.rs
match uri_path {
    p if p.starts_with("/pages/") => serve_page_file(p),
    p if p == "/dashapp.js" => serve_dashapp_script(),
    _ => not_found(),
}
```

### API Server Lifecycle

```rust
// Server starts with app initialization
let api_server = ApiServer::start().await?;

// Token and port passed to webview spawner
spawn_webview_with_api(
    api_server.port(),
    api_server.token(),
);
```

## Testing

### Manual Testing

Open any page and use browser DevTools console:

```javascript
// Test API connectivity
await dashApp.listAccounts();

// Check token configuration
console.log('Token:', window.__DASH_API_TOKEN__?.substring(0, 16) + '...');
console.log('API URL:', window.__DASH_API_URL__);
```

### Debug Logging

API requests are logged in `~/.local/share/awsdash/logs/awsdash.log`:

```
📨 API request from origin: "wry://localhost"
📨 API command: queryCachedResources
✅ API response: queryCachedResources (success)
```

## Key Source Files

- [`src/app/webview/dashapp.js`](../src/app/webview/dashapp.js) - JavaScript API implementation
- [`src/app/webview/api_server.rs`](../src/app/webview/api_server.rs) - Axum HTTP server
- [`src/app/webview/commands.rs`](../src/app/webview/commands.rs) - Command implementations
- [`src/app/webview/custom_protocol.rs`](../src/app/webview/custom_protocol.rs) - wry:// protocol handler

## Related Documentation

- [V8 Bindings vs Webview API](v8-bindings-vs-webview-api.md) - Comparison with agent JavaScript execution
- [Page Builder System](page-builder-system.md) - How agents create pages using this API
- [Code Execution Tool](code-execution-tool.md) - V8-based JavaScript for agents
