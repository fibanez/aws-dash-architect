/**
 * AWS Dash Application API
 *
 * Tauri-inspired IPC library for webview communication with Rust backend.
 * Provides a window.dashApp API for calling Rust functions from JavaScript.
 *
 * Key Features:
 * - Dual callback system (success + error) with mutual cleanup
 * - Crypto-random callback IDs (prevents collisions)
 * - Type conversion for Map, Uint8Array, ArrayBuffer
 * - Security via invoke key
 *
 * Based on Tauri's production IPC implementation.
 */
(function() {
  'use strict';

  // ========== SAVE ORIGINAL CONSOLE METHODS (MUST BE FIRST!) ==========

  /**
   * Save original console methods before we override them
   * This must be done BEFORE any other code so invoke() can use them
   */
  const originalConsoleLog = console.log;
  const originalConsoleWarn = console.warn;
  const originalConsoleError = console.error;

  // ========== CALLBACK REGISTRY ==========

  /**
   * Callback registry using crypto-random IDs
   * Map<number, (data: any) => void>
   */
  const callbacks = new Map();

  /**
   * Register a callback and return its crypto-random ID
   *
   * @param {Function} callback - Callback function to register
   * @param {boolean} once - If true, callback is automatically deleted after invocation
   * @returns {number} - Crypto-random callback ID (u32)
   */
  function registerCallback(callback, once = false) {
    // Generate crypto-random ID (Tauri pattern)
    const id = window.crypto.getRandomValues(new Uint32Array(1))[0];

    callbacks.set(id, (data) => {
      if (once) {
        callbacks.delete(id);
      }
      return callback && callback(data);
    });

    return id;
  }

  /**
   * Invoke a callback by ID
   *
   * Handles missing callbacks gracefully (can happen on app reload).
   *
   * @param {number} id - Callback ID
   * @param {any} data - Data to pass to callback
   */
  function runCallback(id, data) {
    const callback = callbacks.get(id);
    if (callback) {
      callback(data);
    } else {
      // Use original console.warn to avoid triggering log interception
      originalConsoleWarn(
        `[DashApp] Callback ${id} not found. ` +
        `This might happen when the app is reloaded while Rust is running an operation.`
      );
    }
  }

  // ========== TYPE CONVERSION ==========

  /**
   * Serialize arguments for IPC with proper type conversion
   *
   * Handles edge cases from Tauri:
   * - Map → Plain object
   * - Uint8Array → Array
   * - ArrayBuffer → Array
   *
   * @param {any} args - Arguments to serialize
   * @returns {string} - JSON string
   */
  function serializeArgs(args) {
    return JSON.stringify(args, (_k, val) => {
      if (val instanceof Map) {
        return Object.fromEntries(val.entries());
      } else if (val instanceof Uint8Array) {
        return Array.from(val);
      } else if (val instanceof ArrayBuffer) {
        return Array.from(new Uint8Array(val));
      } else {
        return val;
      }
    });
  }

  // ========== INVOKE API ==========

  /**
   * Invoke a Rust command via HTTP API
   *
   * Makes HTTP POST request to main process API server.
   * The main process executes using its AWS client and cache.
   *
   * @param {string} cmd - Command name (e.g., "listAccounts")
   * @param {any} args - Command arguments
   * @returns {Promise<any>} - Promise resolving with command result
   */
  async function invoke(cmd, args = {}) {
    // Get API URL (injected by Rust)
    const apiUrl = window.__DASH_API_URL__;
    const apiToken = window.__DASH_API_TOKEN__; // May be undefined

    // Use original console.log to avoid infinite loop with console.log override
    originalConsoleLog('[DASH] API URL:', apiUrl);
    originalConsoleLog('[DASH] API Token:', apiToken ? apiToken.substring(0, 16) + '...' : 'none');
    originalConsoleLog('[DASH] Page origin:', window.location.origin);
    originalConsoleLog('[DASH] Page href:', window.location.href);

    if (!apiUrl) {
      throw new Error('API server not configured');
    }

    const fullUrl = `${apiUrl}/api/command`;
    originalConsoleLog('[DASH] Fetching:', fullUrl);

    // Make HTTP POST request with API token for authentication
    const headers = {
      'Content-Type': 'application/json',
    };

    // API token is required for authentication
    if (!apiToken) {
      throw new Error('API token not configured - cannot make authenticated requests');
    }
    headers['X-API-Token'] = apiToken;

    try {
      const response = await fetch(fullUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify({
          cmd,
          payload: args,
        }),
      });

      originalConsoleLog('[DASH] Response status:', response.status);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result = await response.json();
      originalConsoleLog('[DASH] Response data:', result);

      if (!result.success) {
        throw new Error(result.data.error || 'Unknown error');
      }

      return result.data;
    } catch (error) {
      originalConsoleError('[DASH] Fetch error:', error);
      throw error;
    }
  }

  // ========== PUBLIC API ==========

  /**
   * AWS Dash Application API
   *
   * All methods return Promises that resolve with results or reject with errors.
   */
  window.dashApp = {
    /**
     * Low-level invoke function for custom commands
     * @param {string} cmd - Command name
     * @param {any} args - Command arguments
     * @returns {Promise<any>}
     */
    invoke,

    // ========== PAGE MANAGEMENT ==========

    /**
     * Open a page in a new webview window
     *
     * @param {string} pageName - The workspace name of the page to open
     * @param {string} [message] - Optional message to display
     * @returns {Promise<object>} - Result with status, message, page_name, and page_url
     *
     * @example
     * await dashApp.openPage('my-dashboard', 'Opening dashboard in new window');
     */
    async openPage(pageName, message) {
      return invoke('openPage', { pageName, message });
    },

    // ========== AWS ACCOUNT & REGION FUNCTIONS ==========

    /**
     * List all configured AWS accounts
     *
     * @returns {Promise<Array>} - Array of account objects with id, name, alias, email
     *
     * @example
     * const accounts = await dashApp.listAccounts();
     * console.log(`Found ${accounts.length} accounts`);
     * accounts.forEach(a => console.log(`${a.id}: ${a.name}`));
     */
    async listAccounts() {
      return invoke('listAccounts', {});
    },

    /**
     * List all AWS regions
     *
     * @returns {Promise<Array>} - Array of region objects with code and name
     *
     * @example
     * const regions = await dashApp.listRegions();
     * const usRegions = regions.filter(r => r.code.startsWith('us-'));
     */
    async listRegions() {
      return invoke('listRegions', {});
    },

    // ========== RESOURCE QUERY FUNCTIONS ==========

    /**
     * Load AWS resources into cache
     *
     * Queries AWS resources and returns counts per account:region:resourceType.
     * Use this to populate the cache before calling queryCachedResources.
     *
     * @param {object} options - Query options
     * @param {string[]} [options.accounts] - Account IDs (null = all accounts)
     * @param {string[]} [options.regions] - Region codes (null = default regions)
     * @param {string[]} options.resourceTypes - CloudFormation resource types (REQUIRED)
     * @returns {Promise<object>} - Result with status, countByScope, totalCount, etc.
     *
     * @example
     * const result = await dashApp.loadCache({
     *   accounts: null,
     *   regions: ['us-east-1', 'us-west-2'],
     *   resourceTypes: ['AWS::Lambda::Function', 'AWS::EC2::Instance']
     * });
     * console.log(`Loaded ${result.totalCount} resources`);
     */
    async loadCache(options) {
      return invoke('loadCache', options);
    },

    /**
     * Get resource schema for a resource type
     *
     * Returns an example resource showing structure and available properties.
     * Useful for understanding what data is available before querying.
     *
     * @param {string} resourceType - CloudFormation resource type
     * @returns {Promise<object>} - Schema result with exampleResource and cacheStats
     *
     * @example
     * const schema = await dashApp.getResourceSchema('AWS::EC2::SecurityGroup');
     * console.log('Available properties:', Object.keys(schema.exampleResource.properties));
     */
    async getResourceSchema(resourceType) {
      return invoke('getResourceSchema', { resourceType });
    },

    /**
     * Show resources in Explorer window
     *
     * Opens the Explorer UI with the specified configuration.
     *
     * @param {object} config - Explorer configuration
     * @param {string[]} [config.accounts] - Account IDs to display
     * @param {string[]} [config.regions] - Region codes to display
     * @param {string[]} [config.resourceTypes] - Resource types to display
     * @param {object} [config.grouping] - Grouping mode
     * @param {object} [config.tagFilters] - Tag filters
     * @param {string} [config.searchFilter] - Search filter text
     * @param {string} [config.title] - Display title
     * @returns {Promise<object>} - Result with status
     *
     * @example
     * await dashApp.showInExplorer({
     *   resourceTypes: ['AWS::EC2::SecurityGroup'],
     *   title: 'Security Groups Overview'
     * });
     */
    async showInExplorer(config) {
      return invoke('showInExplorer', config);
    },

    /**
     * Query cached resources
     *
     * Returns actual resource objects from cache for filtering/analysis.
     * Call loadCache first to populate the cache.
     *
     * NOTE: This API returns FULL results (all matching resources). When called from
     * the webview (dashApp API), you receive complete data including all resources.
     * This is different from execute_javascript in agent context, which returns
     * optimized summaries for context efficiency with full data saved to VFS.
     *
     * @param {object} options - Query options
     * @param {string[]} [options.accounts] - Account IDs (null = all cached)
     * @param {string[]} [options.regions] - Region codes (null = all cached)
     * @param {string[]} options.resourceTypes - Resource types to query (REQUIRED)
     * @returns {Promise<object>} - Result with resources array and count
     *
     * @example
     * const result = await dashApp.queryCachedResources({
     *   resourceTypes: ['AWS::Lambda::Function']
     * });
     * result.resources.forEach(r => {
     *   console.log(`${r.displayName} in ${r.region}`);
     * });
     */
    async queryCachedResources(options) {
      return invoke('queryCachedResources', options);
    },

    // ========== BOOKMARK FUNCTIONS ==========

    /**
     * List all saved bookmarks
     *
     * @returns {Promise<Array>} - Array of bookmark info objects
     *
     * @example
     * const bookmarks = await dashApp.listBookmarks();
     * bookmarks.forEach(b => console.log(`${b.name}: ${b.resourceTypes.join(', ')}`));
     */
    async listBookmarks() {
      return invoke('listBookmarks', {});
    },

    /**
     * Query a bookmark
     *
     * Executes a bookmark's saved query and returns resources.
     *
     * @param {string} bookmarkId - The bookmark ID to query
     * @param {object} [options] - Query options
     * @param {string} [options.detail] - Detail level: 'count', 'summary', 'tags', 'full'
     * @returns {Promise<object>} - Query result with resources
     *
     * @example
     * const result = await dashApp.queryBookmarks('my-bookmark-id', { detail: 'tags' });
     * console.log(`Found ${result.data.length} resources`);
     */
    async queryBookmarks(bookmarkId, options) {
      return invoke('queryBookmarks', { bookmarkId, options });
    },

    // ========== CLOUDWATCH LOGS FUNCTIONS ==========

    /**
     * Query CloudWatch Log events
     *
     * @param {object} params - Query parameters
     * @param {string} params.logGroupName - Log group name (required)
     * @param {string} params.accountId - AWS account ID (required)
     * @param {string} params.region - AWS region (required)
     * @param {number} [params.startTime] - Start time in Unix milliseconds
     * @param {number} [params.endTime] - End time in Unix milliseconds
     * @param {string} [params.filterPattern] - CloudWatch Logs filter pattern
     * @param {number} [params.limit] - Max events to return (default 100, max 10000)
     * @param {string[]} [params.logStreamNames] - Specific log streams
     * @param {boolean} [params.startFromHead] - Query chronologically
     * @returns {Promise<object>} - Result with events array and statistics
     *
     * @example
     * const logs = await dashApp.queryCloudWatchLogEvents({
     *   logGroupName: '/aws/lambda/my-function',
     *   accountId: '123456789012',
     *   region: 'us-east-1',
     *   filterPattern: 'ERROR',
     *   startTime: Date.now() - (60 * 60 * 1000), // Last hour
     *   limit: 100
     * });
     * logs.events.forEach(e => console.log(e.message));
     */
    async queryCloudWatchLogEvents(params) {
      return invoke('queryCloudWatchLogEvents', params);
    },

    // ========== CLOUDTRAIL FUNCTIONS ==========

    /**
     * Get CloudTrail events
     *
     * Query CloudTrail events for governance, compliance, and security analysis.
     *
     * @param {object} params - Query parameters
     * @param {string} params.accountId - AWS account ID (required)
     * @param {string} params.region - AWS region (required)
     * @param {number} [params.startTime] - Start time in Unix milliseconds
     * @param {number} [params.endTime] - End time in Unix milliseconds
     * @param {Array} [params.lookupAttributes] - Filter attributes
     * @param {number} [params.maxResults] - Max events (default 100)
     * @param {string} [params.nextToken] - Pagination token
     * @returns {Promise<object>} - Result with events array
     *
     * @example
     * const events = await dashApp.getCloudTrailEvents({
     *   accountId: '123456789012',
     *   region: 'us-east-1',
     *   startTime: Date.now() - (24 * 60 * 60 * 1000) // Last 24 hours
     * });
     * events.events.forEach(e => console.log(`${e.eventName} by ${e.username}`));
     */
    async getCloudTrailEvents(params) {
      return invoke('getCloudTrailEvents', params);
    },

    // ========== PAGE MANAGEMENT FUNCTIONS ==========

    /**
     * List all pages with metadata
     *
     * Returns information about all pages in the pages directory.
     *
     * @returns {Promise<Array<object>>} - Array of page info objects
     * @property {string} name - Page/workspace name
     * @property {number} createdAt - Creation timestamp (Unix seconds)
     * @property {number} lastModified - Last modification timestamp (Unix seconds)
     * @property {number} totalSize - Total size of all files in bytes
     * @property {number} fileCount - Number of files in the page
     *
     * @example
     * const pages = await dashApp.listPages();
     * pages.forEach(p => console.log(`${p.name} - ${p.fileCount} files`));
     */
    async listPages() {
      return invoke('listPages', {});
    },

    /**
     * Delete a page and all its files
     *
     * Removes the page directory and all contents.
     * This action cannot be undone.
     *
     * @param {string} pageName - Name of the page to delete
     * @returns {Promise<object>} - Result with status and message
     *
     * @example
     * const result = await dashApp.deletePage('my-old-page');
     * console.log(result.message); // "Page 'my-old-page' deleted successfully"
     */
    async deletePage(pageName) {
      return invoke('deletePage', { pageName });
    },

    /**
     * View a page in a new webview window
     *
     * Opens the page for preview in a separate window.
     *
     * @param {string} pageName - Name of the page to view
     * @returns {Promise<object>} - Result with status and page URL
     *
     * @example
     * await dashApp.viewPage('my-dashboard');
     */
    async viewPage(pageName) {
      return invoke('viewPage', { pageName });
    },

    /**
     * Edit a page by opening an agent editor
     *
     * Opens the Agent Manager with a TaskManager configured to edit the page.
     * The user can then describe what changes they want to make.
     *
     * @param {string} pageName - Name of the page to edit
     * @returns {Promise<object>} - Result with status
     *
     * @example
     * await dashApp.editPage('my-dashboard');
     * // Opens Agent Manager, user describes changes
     */
    async editPage(pageName) {
      return invoke('editPage', { pageName });
    },

    /**
     * Rename a page (folder)
     *
     * Renames the page directory to a new name.
     *
     * @param {string} oldName - Current name of the page
     * @param {string} newName - New name for the page
     * @returns {Promise<object>} - Result with status
     *
     * @example
     * await dashApp.renamePage('old-dashboard', 'new-dashboard');
     */
    async renamePage(oldName, newName) {
      return invoke('renamePage', { oldName, newName });
    }
  };

  // ========== PAGE UTILITIES ==========

  /**
   * Extract page name from page's script/link tags
   * Returns null if not in a page context
   */
  function getPageName() {
    // Check if explicitly set
    if (window.__PAGE_NAME__) {
      return window.__PAGE_NAME__;
    }

    // Try to extract from <script> or <link> tags with wry://localhost/pages/{name}/ pattern
    const scripts = document.querySelectorAll('script[src^="wry://localhost/pages/"]');
    const links = document.querySelectorAll('link[href^="wry://localhost/pages/"]');

    for (const elem of [...scripts, ...links]) {
      const url = elem.src || elem.href;
      const match = url.match(/wry:\/\/localhost\/pages\/([^/]+)\//);
      if (match) {
        return match[1];
      }
    }

    return null;
  }

  // ========== AUTOMATIC CONSOLE.LOG INTERCEPTION ==========

  /**
   * Internal function to write a log message to page.log file
   *
   * @param {...any} args - Arguments to log
   */
  function writeToPageLog(...args) {
    // Format message as string with timestamp
    const timestamp = new Date().toISOString();
    const message = args.map(arg => {
      if (typeof arg === 'object') {
        try {
          return JSON.stringify(arg, null, 2);
        } catch (e) {
          return String(arg);
        }
      }
      return String(arg);
    }).join(' ');

    // Get page name
    const pageName = getPageName();
    if (!pageName) {
      return; // Not in a page context, skip file logging
    }

    // Send to page.log via API (don't await - fire and forget)
    invoke('logToPageFile', {
      pageName,
      message: `[${timestamp}] ${message}`
    }).catch(err => {
      // Use original console.warn to avoid recursion
      originalConsoleWarn('[writeToPageLog] Failed to write to page.log:', err);
    });
  }

  /**
   * Override console.log to automatically write to page.log
   *
   * This allows LLM-written code using regular console.log() to automatically
   * log to both browser console AND page.log file without any code changes.
   */
  console.log = function(...args) {
    // Call original console.log (browser console)
    originalConsoleLog.apply(console, args);

    // Write to page.log if in page context
    writeToPageLog(...args);
  };

  // Preserve original console.log for debugging if needed
  console.__original_log = originalConsoleLog;

  // ========== DEBUGGING INTERNALS ==========

  /**
   * Internal API exposed for debugging
   *
   * WARNING: Internal use only. API may change without notice.
   */
  window.__DASH_INTERNALS__ = {
    callbacks,
    runCallback,
    registerCallback,
    serializeArgs,
    version: '1.0.0'
  };

  // ========== INVOKE KEY GENERATION ==========

  /**
   * Generate unique invoke key for this webview instance
   *
   * Security: Only scripts initialized by our Rust code have this key.
   * Prevents unauthorized IPC calls from injected scripts.
   */
  if (!window.__INVOKE_KEY__) {
    window.__INVOKE_KEY__ = window.crypto.getRandomValues(new Uint32Array(4))
      .join('-');
  }

  // Use original console.log to avoid triggering log interception during init
  originalConsoleLog('[DashApp] API initialized');
})();
