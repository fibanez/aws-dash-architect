//! HTTP API Test Window - Test webview API integration
//!
//! This window allows testing the API infrastructure by spawning a test webview
//! that communicates with the Rust backend.

use eframe::egui;

/// HTTP API Test Window
pub struct ApiTestWindow {
    /// Test HTML content to inject
    test_html: String,

    /// Status message
    status: String,

    /// Whether webview is running
    webview_running: bool,
}

impl Default for ApiTestWindow {
    fn default() -> Self {
        Self {
            test_html: generate_test_html(),
            status: "Ready to test HTTP API. Click 'Launch Test Webview' to begin.".to_string(),
            webview_running: false,
        }
    }
}

impl ApiTestWindow {
    /// Render the window
    pub fn render(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new("HTTP API Test Window")
            .open(open)
            .default_width(600.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.heading("WebView HTTP API Test");
                ui.separator();

                ui.label("This test spawns a webview with the dashApp API and allows you to test HTTP API communication.");
                ui.add_space(10.0);

                // Status
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(
                        if self.webview_running {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::GRAY
                        },
                        &self.status,
                    );
                });

                ui.add_space(10.0);

                // Launch button
                if ui.button("Launch Test Webview").clicked() {
                    match self.launch_test_webview() {
                        Ok(_) => {
                            self.status = "Test webview launched! Check the webview window.".to_string();
                            self.webview_running = true;
                        }
                        Err(e) => {
                            self.status = format!("Failed to launch webview: {}", e);
                            self.webview_running = false;
                        }
                    }
                }

                ui.add_space(10.0);

                // Launch disk-based SPA test
                if ui.button("Launch Disk-Based SPA Test").clicked() {
                    match self.launch_disk_spa_test() {
                        Ok(_) => {
                            self.status = "Disk-based SPA test launched! Check the webview window.".to_string();
                            self.webview_running = true;
                        }
                        Err(e) => {
                            self.status = format!("Failed to launch disk SPA test: {}", e);
                            self.webview_running = false;
                        }
                    }
                }

                ui.add_space(20.0);
                ui.separator();

                ui.heading("Test HTML Preview");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.code(&self.test_html);
                    });
            });
    }

    /// Launch test webview with embedded HTML
    fn launch_test_webview(&self) -> anyhow::Result<()> {
        tracing::info!("Launching API test webview with embedded HTML");

        // Spawn webview process with embedded HTML
        crate::app::webview::spawn_webview_process_with_html(
            self.test_html.clone(),
            "API Test".to_string(),
        )?;

        Ok(())
    }

    /// Launch disk-based SPA test
    fn launch_disk_spa_test(&self) -> anyhow::Result<()> {
        tracing::info!("Launching disk-based SPA test");

        // Read the index.html from disk
        let page_path = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find local data directory"))?
            .join("awsdash/pages/test-app/index.html");

        if !page_path.exists() {
            return Err(anyhow::anyhow!(
                "Test page not found at {:?}. Please create the test page files first.",
                page_path
            ));
        }

        let html = std::fs::read_to_string(&page_path)?;
        tracing::info!("Loaded test page HTML from {:?} ({} bytes)", page_path, html.len());

        // Spawn webview process with HTML that will load assets from disk
        crate::app::webview::spawn_webview_process_with_html(
            html,
            "Dash Page Test - Disk Files".to_string(),
        )?;

        Ok(())
    }
}

/// Generate test HTML content
fn generate_test_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AWS Dash API Test</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 800px;
            margin: 20px auto;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .test-section {
            background: white;
            padding: 15px;
            margin: 10px 0;
            border-radius: 5px;
            box-shadow: 0 2px 5px rgba(0,0,0,0.1);
        }
        button {
            background-color: #007bff;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 4px;
            cursor: pointer;
            margin: 5px;
        }
        button:hover {
            background-color: #0056b3;
        }
        .result {
            margin-top: 10px;
            padding: 10px;
            background-color: #e9ecef;
            border-radius: 4px;
            white-space: pre-wrap;
            font-family: monospace;
            font-size: 12px;
        }
        .error {
            background-color: #f8d7da;
            color: #721c24;
        }
        .success {
            background-color: #d4edda;
            color: #155724;
        }
        h1 { color: #333; }
        h2 { color: #666; font-size: 16px; }
        .info {
            background-color: #d1ecf1;
            color: #0c5460;
            padding: 10px;
            border-radius: 4px;
            margin-bottom: 15px;
        }
    </style>
</head>
<body>
    <h1>AWS Dash API Test</h1>

    <div class="test-section">
        <h2>Test 1: List Accounts</h2>
        <button onclick="testListAccounts()">Run Test</button>
        <div id="accounts-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 2: List Regions</h2>
        <button onclick="testListRegions()">Run Test</button>
        <div id="regions-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 3: Load Cache</h2>
        <button onclick="testLoadCache()">Run Test</button>
        <div id="loadcache-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 4: Get Resource Schema</h2>
        <button onclick="testGetResourceSchema()">Run Test</button>
        <div id="schema-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 5: Query Cached Resources</h2>
        <button onclick="testQueryCachedResources()">Run Test</button>
        <div id="query-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 6: Show In Explorer</h2>
        <button onclick="testShowInExplorer()">Run Test</button>
        <div id="explorer-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 7: API Availability Check</h2>
        <button onclick="checkAPI()">Run Test</button>
        <div id="api-result" class="result" style="display:none;"></div>
    </div>

    <div class="test-section">
        <h2>Test 8: Run All Tests</h2>
        <button onclick="runAllTests()">Run All Tests</button>
        <div id="all-tests-result" class="result" style="display:none;"></div>
    </div>

    <script>
        function displayResult(elementId, result, isError = false) {
            const element = document.getElementById(elementId);
            element.style.display = 'block';
            element.className = `result ${isError ? 'error' : 'success'}`;
            element.textContent = typeof result === 'object'
                ? JSON.stringify(result, null, 2)
                : result;
        }

        async function testListAccounts() {
            try {
                console.log('[TEST] Calling dashApp.listAccounts()...');

                if (!window.dashApp || !window.dashApp.listAccounts) {
                    throw new Error('dashApp.listAccounts not available');
                }

                const accounts = await window.dashApp.listAccounts();
                console.log('[TEST] Received accounts:', accounts);

                displayResult('accounts-result', {
                    status: 'SUCCESS',
                    accountCount: accounts.length,
                    accounts: accounts
                });
            } catch (error) {
                console.error('[TEST] Error:', error);
                displayResult('accounts-result', `ERROR: ${error.message}`, true);
            }
        }

        async function testListRegions() {
            try {
                console.log('[TEST] Calling dashApp.listRegions()...');

                if (!window.dashApp || !window.dashApp.listRegions) {
                    throw new Error('dashApp.listRegions not available');
                }

                const regions = await window.dashApp.listRegions();
                console.log('[TEST] Received regions:', regions);

                displayResult('regions-result', {
                    status: 'SUCCESS',
                    regionCount: regions.length,
                    firstFive: regions.slice(0, 5)
                });
            } catch (error) {
                console.error('[TEST] Error:', error);
                displayResult('regions-result', `ERROR: ${error.message}`, true);
            }
        }

        async function testLoadCache() {
            try {
                console.log('[TEST] Calling dashApp.loadCache()...');

                if (!window.dashApp || !window.dashApp.loadCache) {
                    throw new Error('dashApp.loadCache not available');
                }

                const result = await window.dashApp.loadCache({
                    accounts: null,
                    regions: ['us-east-1'],
                    resourceTypes: ['AWS::EC2::Instance', 'AWS::S3::Bucket']
                });
                console.log('[TEST] Received result:', result);

                displayResult('loadcache-result', {
                    status: result.status,
                    totalCount: result.totalCount,
                    accountsQueried: result.accountsQueried,
                    regionsQueried: result.regionsQueried,
                    countByScope: result.countByScope
                });
            } catch (error) {
                console.error('[TEST] Error:', error);
                displayResult('loadcache-result', `ERROR: ${error.message}`, true);
            }
        }

        async function testGetResourceSchema() {
            try {
                console.log('[TEST] Calling dashApp.getResourceSchema()...');

                if (!window.dashApp || !window.dashApp.getResourceSchema) {
                    throw new Error('dashApp.getResourceSchema not available');
                }

                const result = await window.dashApp.getResourceSchema('AWS::S3::Bucket');
                console.log('[TEST] Received result:', result);

                displayResult('schema-result', {
                    status: result.status,
                    resourceType: result.resourceType,
                    hasExample: !!result.exampleResource,
                    cacheStats: result.cacheStats
                });
            } catch (error) {
                console.error('[TEST] Error:', error);
                displayResult('schema-result', `ERROR: ${error.message}`, true);
            }
        }

        async function testQueryCachedResources() {
            try {
                console.log('[TEST] Calling dashApp.queryCachedResources()...');

                if (!window.dashApp || !window.dashApp.queryCachedResources) {
                    throw new Error('dashApp.queryCachedResources not available');
                }

                const result = await window.dashApp.queryCachedResources({
                    accounts: null,
                    regions: ['us-east-1'],
                    resourceTypes: ['AWS::S3::Bucket']
                });
                console.log('[TEST] Received result:', result);

                displayResult('query-result', {
                    status: result.status,
                    resourceCount: result.resources.length,
                    firstResource: result.resources[0] || null
                });
            } catch (error) {
                console.error('[TEST] Error:', error);
                displayResult('query-result', `ERROR: ${error.message}`, true);
            }
        }

        async function testShowInExplorer() {
            try {
                console.log('[TEST] Calling dashApp.showInExplorer()...');

                if (!window.dashApp || !window.dashApp.showInExplorer) {
                    throw new Error('dashApp.showInExplorer not available');
                }

                const result = await window.dashApp.showInExplorer({
                    accounts: null,
                    regions: ['us-east-1'],
                    resourceTypes: ['AWS::EC2::Instance'],
                    title: 'API Test - EC2 Instances'
                });
                console.log('[TEST] Received result:', result);

                displayResult('explorer-result', {
                    status: result.status,
                    message: result.message
                });
            } catch (error) {
                console.error('[TEST] Error:', error);
                displayResult('explorer-result', `ERROR: ${error.message}`, true);
            }
        }

        function checkAPI() {
            const apiMethods = [
                'invoke', 'listAccounts', 'listRegions', 'loadCache',
                'getResourceSchema', 'queryCachedResources',
                'showInExplorer', 'queryCloudWatchLogs',
                'getCloudTrailEvents', 'saveCurrentApp'
            ];

            const results = {
                dashAppAvailable: !!window.dashApp,
                invokeKeySet: !!window.__INVOKE_KEY__,
                internalsAvailable: !!window.__DASH_INTERNALS__,
                methods: {}
            };

            if (window.dashApp) {
                apiMethods.forEach(method => {
                    results.methods[method] = typeof window.dashApp[method] === 'function';
                });
            }

            if (window.__DASH_INTERNALS__) {
                results.callbackCount = window.__DASH_INTERNALS__.callbacks.size;
            }

            displayResult('api-result', results);
        }

        async function runAllTests() {
            const results = [];

            // Check API
            if (!window.dashApp) {
                displayResult('all-tests-result', 'ERROR: window.dashApp not available!', true);
                return;
            }

            results.push('API Check: PASSED');

            // Test listAccounts
            try {
                const accounts = await window.dashApp.listAccounts();
                results.push(`List Accounts: PASSED (${accounts.length} accounts)`);
            } catch (error) {
                results.push(`List Accounts: FAILED - ${error.message}`);
            }

            // Test listRegions
            try {
                const regions = await window.dashApp.listRegions();
                results.push(`List Regions: PASSED (${regions.length} regions)`);
            } catch (error) {
                results.push(`List Regions: FAILED - ${error.message}`);
            }

            // Test loadCache
            try {
                const result = await window.dashApp.loadCache({
                    accounts: null,
                    regions: ['us-east-1'],
                    resourceTypes: ['AWS::EC2::Instance']
                });
                results.push(`Load Cache: PASSED (${result.totalCount} resources)`);
            } catch (error) {
                results.push(`Load Cache: FAILED - ${error.message}`);
            }

            // Test getResourceSchema
            try {
                const result = await window.dashApp.getResourceSchema('AWS::S3::Bucket');
                results.push(`Get Resource Schema: ${result.status === 'success' ? 'PASSED' : 'FAILED'}`);
            } catch (error) {
                results.push(`Get Resource Schema: FAILED - ${error.message}`);
            }

            // Test queryCachedResources
            try {
                const result = await window.dashApp.queryCachedResources({
                    accounts: null,
                    regions: ['us-east-1'],
                    resourceTypes: ['AWS::S3::Bucket']
                });
                results.push(`Query Cached Resources: PASSED (${result.resources.length} resources)`);
            } catch (error) {
                results.push(`Query Cached Resources: FAILED - ${error.message}`);
            }

            // Test showInExplorer (note: just enqueues action, doesn't validate success)
            try {
                const result = await window.dashApp.showInExplorer({
                    accounts: null,
                    regions: ['us-east-1'],
                    resourceTypes: ['AWS::EC2::Instance'],
                    title: 'Test - All Tests'
                });
                results.push(`Show In Explorer: ${result.status === 'success' ? 'PASSED' : 'FAILED'}`);
            } catch (error) {
                results.push(`Show In Explorer: FAILED - ${error.message}`);
            }

            displayResult('all-tests-result', results.join('\n'));
        }

        // Auto-run API check on load
        window.addEventListener('DOMContentLoaded', () => {
            console.log('[TEST] Page loaded');
            console.log('[TEST] window.dashApp:', window.dashApp);
            console.log('[TEST] window.__INVOKE_KEY__:', window.__INVOKE_KEY__);

            if (!window.dashApp) {
                alert('ERROR: window.dashApp is not available!');
            } else {
                console.log('[TEST] library loaded successfully');
            }
        });
    </script>
</body>
</html>"#.to_string()
}
