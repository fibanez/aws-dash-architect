//! Pages Manager Window - Manage Dash Pages
//!
//! A webview-based window for viewing, editing, and deleting Dash Pages.
//! Supports Catppuccin dark and light themes.

#![warn(clippy::all, rust_2018_idioms)]

/// Generate the Pages Manager HTML content
///
/// Returns HTML string with theme-aware styling and JavaScript
/// that calls dashApp API methods for page management.
///
/// # Arguments
/// * `is_dark_theme` - If true, use Catppuccin dark theme colors; otherwise use light theme
pub fn generate_pages_manager_html(is_dark_theme: bool) -> String {
    // CSS variables for light theme (Catppuccin Latte)
    let light_theme = r#"
        :root {
            --primary-color: #1e66f5;
            --success-color: #40a02b;
            --warning-color: #df8e1d;
            --error-color: #d20f39;
            --text-color: #4c4f69;
            --text-secondary: #6c6f85;
            --border-color: #ccd0da;
            --background-color: #eff1f5;
            --component-background: #e6e9ef;
            --hover-background: #ccd0da;
            --table-header-bg: #dce0e8;
            --modal-background: #e6e9ef;
        }
    "#;

    // CSS variables for dark theme (Catppuccin Mocha)
    let dark_theme = r#"
        :root {
            --primary-color: #89b4fa;
            --success-color: #a6e3a1;
            --warning-color: #f9e2af;
            --error-color: #f38ba8;
            --text-color: #cdd6f4;
            --text-secondary: #a6adc8;
            --border-color: #45475a;
            --background-color: #1e1e2e;
            --component-background: #313244;
            --hover-background: #45475a;
            --table-header-bg: #181825;
            --modal-background: #313244;
        }
    "#;

    let theme_css = if is_dark_theme { dark_theme } else { light_theme };

    // Build HTML by concatenating parts to avoid format string escaping issues with CSS braces
    let html_start = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AWS Dash Pages</title>
    <!-- Fuse.js for fuzzy search -->
    <script src="https://cdn.jsdelivr.net/npm/fuse.js@7.0.0/dist/fuse.min.js"></script>
    <style>
"##;

    let css_rest = r##"

        * {
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            margin: 0;
            padding: 0;
            background-color: var(--background-color);
            color: var(--text-color);
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 24px;
        }

        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 24px;
            padding-bottom: 16px;
            border-bottom: 1px solid var(--border-color);
        }

        .header h1 {
            margin: 0;
            font-size: 24px;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .header h1 .icon {
            color: #faad14;
            font-size: 28px;
        }

        .header-actions {
            display: flex;
            gap: 8px;
        }

        .btn {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            padding: 8px 16px;
            font-size: 14px;
            font-weight: 400;
            line-height: 1.5;
            border: 1px solid var(--border-color);
            border-radius: 6px;
            cursor: pointer;
            transition: all 0.2s;
            background: var(--component-background);
            color: var(--text-color);
        }

        .btn:hover {
            color: var(--primary-color);
            border-color: var(--primary-color);
        }

        .btn-primary {
            background: var(--primary-color);
            border-color: var(--primary-color);
            color: white;
        }

        .btn-primary:hover {
            background: #40a9ff;
            border-color: #40a9ff;
            color: white;
        }

        .btn-danger {
            color: var(--error-color);
            border-color: var(--error-color);
        }

        .btn-danger:hover {
            background: var(--error-color);
            color: white;
        }

        .btn-success {
            color: var(--success-color);
            border-color: var(--success-color);
        }

        .btn-success:hover {
            background: var(--success-color);
            color: white;
        }

        .btn-sm {
            padding: 4px 12px;
            font-size: 12px;
        }

        .card {
            background: var(--component-background);
            border-radius: 8px;
            box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03), 0 1px 6px -1px rgba(0, 0, 0, 0.02), 0 2px 4px rgba(0, 0, 0, 0.02);
        }

        .table-container {
            overflow-x: auto;
        }

        table {
            width: 100%;
            border-collapse: collapse;
        }

        th, td {
            padding: 16px;
            text-align: left;
            border-bottom: 1px solid var(--border-color);
        }

        th {
            background: var(--table-header-bg);
            font-weight: 500;
            color: var(--text-color);
        }

        th.sortable {
            cursor: pointer;
            user-select: none;
            transition: background 0.2s;
        }

        th.sortable:hover {
            background: var(--hover-background);
            color: var(--primary-color);
        }

        th.sortable .sort-indicator {
            display: inline-block;
            margin-left: 4px;
            color: var(--text-secondary);
            font-size: 12px;
        }

        th.sortable.asc .sort-indicator::after {
            content: ' ▲';
            color: var(--primary-color);
        }

        th.sortable.desc .sort-indicator::after {
            content: ' ▼';
            color: var(--primary-color);
        }

        tr:hover td {
            background: var(--hover-background);
        }

        .page-name {
            font-weight: 500;
            color: var(--primary-color);
            cursor: pointer;
            text-decoration: none;
        }

        .page-name:hover {
            text-decoration: underline;
        }

        .btn-icon {
            padding: 4px 8px;
            font-size: 16px;
            line-height: 1;
            min-width: auto;
        }

        .text-secondary {
            color: var(--text-secondary);
            font-size: 13px;
        }

        .actions {
            display: flex;
            gap: 8px;
        }

        .empty-state {
            text-align: center;
            padding: 48px 24px;
            color: var(--text-secondary);
        }

        .empty-state h3 {
            margin: 16px 0 8px;
            color: var(--text-color);
            font-weight: 500;
        }

        .loading {
            text-align: center;
            padding: 48px 24px;
            color: var(--text-secondary);
        }

        .spinner {
            display: inline-block;
            width: 24px;
            height: 24px;
            border: 2px solid var(--border-color);
            border-top-color: var(--primary-color);
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }

        .status-message {
            padding: 12px 16px;
            margin-bottom: 16px;
            border-radius: 6px;
            display: none;
        }

        .status-message.success {
            background: var(--component-background);
            border: 1px solid var(--success-color);
            color: var(--success-color);
            display: block;
        }

        .status-message.error {
            background: var(--component-background);
            border: 1px solid var(--error-color);
            color: var(--error-color);
            display: block;
        }

        .modal-overlay {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0, 0, 0, 0.45);
            z-index: 1000;
            justify-content: center;
            align-items: center;
        }

        .modal-overlay.visible {
            display: flex;
        }

        .modal {
            background: var(--modal-background);
            border-radius: 8px;
            padding: 24px;
            max-width: 420px;
            width: 90%;
            box-shadow: 0 3px 6px -4px rgba(0,0,0,.12), 0 6px 16px 0 rgba(0,0,0,.08), 0 9px 28px 8px rgba(0,0,0,.05);
            color: var(--text-color);
        }

        .modal h3 {
            margin: 0 0 16px;
            font-size: 16px;
            font-weight: 500;
        }

        .modal p {
            margin: 0 0 24px;
            color: var(--text-secondary);
        }

        .modal-actions {
            display: flex;
            justify-content: flex-end;
            gap: 8px;
        }

        .size-badge {
            display: inline-block;
            padding: 2px 8px;
            background: var(--hover-background);
            border-radius: 4px;
            font-size: 12px;
            color: var(--text-secondary);
        }

        .search-container {
            margin-bottom: 16px;
            display: flex;
            align-items: center;
            gap: 16px;
            flex-wrap: wrap;
        }

        .agent-hint {
            font-size: 13px;
            color: var(--text-secondary);
        }

        .search-input {
            width: 100%;
            max-width: 400px;
            padding: 10px 16px;
            font-size: 14px;
            border: 1px solid var(--border-color);
            border-radius: 6px;
            outline: none;
            transition: border-color 0.2s, box-shadow 0.2s;
            background: var(--component-background);
            color: var(--text-color);
        }

        .search-input:focus {
            border-color: var(--primary-color);
            box-shadow: 0 0 0 2px rgba(137, 180, 250, 0.2);
        }

        .search-input::placeholder {
            color: var(--text-secondary);
        }

        .search-results-info {
            margin-top: 8px;
            font-size: 13px;
            color: var(--text-secondary);
        }

        .modal-input {
            width: 100%;
            padding: 10px 12px;
            font-size: 14px;
            border: 1px solid var(--border-color);
            border-radius: 6px;
            outline: none;
            margin-bottom: 16px;
            background: var(--background-color);
            color: var(--text-color);
        }

        .modal-input:focus {
            border-color: var(--primary-color);
            box-shadow: 0 0 0 2px rgba(137, 180, 250, 0.2);
        }

        .btn-warning {
            color: var(--warning-color);
            border-color: var(--warning-color);
        }

        .btn-warning:hover {
            background: var(--warning-color);
            color: var(--background-color);
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1><span class="icon">&#9889;</span> AWS Dash Pages</h1>
            <div class="header-actions">
                <button class="btn" onclick="refreshPages()">Refresh</button>
            </div>
        </div>

        <div class="search-container">
            <span class="agent-hint">Use an Agent to create or edit pages</span>
            <input type="text" id="search-input" class="search-input" placeholder="Search pages..." oninput="onSearch(this.value)">
            <div id="search-results-info" class="search-results-info" style="display: none;"></div>
        </div>

        <div id="status-message" class="status-message"></div>

        <div class="card">
            <div id="loading" class="loading">
                <div class="spinner"></div>
                <p>Loading pages...</p>
            </div>

            <div id="empty-state" class="empty-state" style="display: none;">
                <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                </svg>
                <h3>No Pages Found</h3>
                <p>Create a page using the Task Manager agent to get started.</p>
            </div>

            <div id="table-container" class="table-container" style="display: none;">
                <table>
                    <thead>
                        <tr>
                            <th class="sortable" data-sort="name" onclick="sortBy('name')">Name<span class="sort-indicator"></span></th>
                            <th class="sortable" data-sort="createdAt" onclick="sortBy('createdAt')">Created<span class="sort-indicator"></span></th>
                            <th class="sortable" data-sort="lastModified" onclick="sortBy('lastModified')">Last Modified<span class="sort-indicator"></span></th>
                            <th class="sortable" data-sort="totalSize" onclick="sortBy('totalSize')">Size<span class="sort-indicator"></span></th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody id="pages-tbody">
                    </tbody>
                </table>
            </div>
        </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div id="delete-modal" class="modal-overlay">
        <div class="modal">
            <h3>Delete Page</h3>
            <p>Are you sure you want to delete "<span id="delete-page-name"></span>"? This action cannot be undone.</p>
            <div class="modal-actions">
                <button class="btn" onclick="closeDeleteModal()">Cancel</button>
                <button class="btn btn-danger" onclick="confirmDelete()">Delete</button>
            </div>
        </div>
    </div>

    <!-- Rename Modal -->
    <div id="rename-modal" class="modal-overlay">
        <div class="modal">
            <h3>Rename Page</h3>
            <p>Enter new name for "<span id="rename-page-old-name"></span>":</p>
            <input type="text" id="rename-new-name" class="modal-input" placeholder="New page name">
            <div class="modal-actions">
                <button class="btn" onclick="closeRenameModal()">Cancel</button>
                <button class="btn btn-primary" onclick="confirmRename()">Rename</button>
            </div>
        </div>
    </div>

    <script>
        // State
        let pages = [];
        let filteredPages = [];
        let pageToDelete = null;
        let pageToRename = null;
        let currentSort = { field: 'name', direction: 'asc' };
        let searchQuery = '';
        let fuse = null;

        // Format timestamp to readable date
        function formatDate(timestamp) {
            if (!timestamp || timestamp === 0) return '-';
            const date = new Date(timestamp * 1000);
            return date.toLocaleDateString('en-US', {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit'
            });
        }

        // Format bytes to human readable size
        function formatSize(bytes) {
            if (bytes === 0) return '0 B';
            const units = ['B', 'KB', 'MB', 'GB'];
            const i = Math.floor(Math.log(bytes) / Math.log(1024));
            return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
        }

        // Show status message
        function showStatus(message, isError = false) {
            const el = document.getElementById('status-message');
            el.textContent = message;
            el.className = 'status-message ' + (isError ? 'error' : 'success');

            // Auto-hide after 5 seconds
            setTimeout(() => {
                el.className = 'status-message';
            }, 5000);
        }

        // Initialize Fuse.js for fuzzy search
        function initFuse() {
            const options = {
                keys: ['name'],
                threshold: 0.4,
                distance: 100,
                includeScore: true,
            };
            fuse = new Fuse(pages, options);
        }

        // Handle search input
        function onSearch(query) {
            searchQuery = query.trim();
            updateFilteredPages();
            renderPages();
        }

        // Update filtered pages based on search query
        function updateFilteredPages() {
            const searchResultsInfo = document.getElementById('search-results-info');

            if (!searchQuery) {
                filteredPages = pages;
                searchResultsInfo.style.display = 'none';
            } else if (fuse) {
                const results = fuse.search(searchQuery);
                filteredPages = results.map(r => r.item);
                searchResultsInfo.style.display = 'block';
                searchResultsInfo.textContent = `Found ${filteredPages.length} of ${pages.length} pages`;
            } else {
                // Fallback to simple filter if Fuse not available
                filteredPages = pages.filter(p =>
                    p.name.toLowerCase().includes(searchQuery.toLowerCase())
                );
                searchResultsInfo.style.display = 'block';
                searchResultsInfo.textContent = `Found ${filteredPages.length} of ${pages.length} pages`;
            }
        }

        // Sort pages by field
        function sortBy(field) {
            // Toggle direction if same field, otherwise default to ascending
            if (currentSort.field === field) {
                currentSort.direction = currentSort.direction === 'asc' ? 'desc' : 'asc';
            } else {
                currentSort.field = field;
                currentSort.direction = 'asc';
            }
            renderPages();
        }

        // Update sort indicators on headers
        function updateSortIndicators() {
            document.querySelectorAll('th.sortable').forEach(th => {
                th.classList.remove('asc', 'desc');
                if (th.dataset.sort === currentSort.field) {
                    th.classList.add(currentSort.direction);
                }
            });
        }

        // Sort pages array based on current sort settings
        function getSortedPages() {
            return [...filteredPages].sort((a, b) => {
                let aVal = a[currentSort.field];
                let bVal = b[currentSort.field];

                // Handle string comparison for name
                if (currentSort.field === 'name') {
                    aVal = aVal.toLowerCase();
                    bVal = bVal.toLowerCase();
                    if (aVal < bVal) return currentSort.direction === 'asc' ? -1 : 1;
                    if (aVal > bVal) return currentSort.direction === 'asc' ? 1 : -1;
                    return 0;
                }

                // Numeric comparison for dates and size
                return currentSort.direction === 'asc' ? aVal - bVal : bVal - aVal;
            });
        }

        // Render pages table
        function renderPages() {
            const loading = document.getElementById('loading');
            const emptyState = document.getElementById('empty-state');
            const tableContainer = document.getElementById('table-container');
            const tbody = document.getElementById('pages-tbody');

            loading.style.display = 'none';

            if (filteredPages.length === 0) {
                if (pages.length === 0) {
                    emptyState.style.display = 'block';
                } else {
                    emptyState.style.display = 'none';
                }
                tableContainer.style.display = 'none';
                return;
            }

            emptyState.style.display = 'none';
            tableContainer.style.display = 'block';

            // Update sort indicators
            updateSortIndicators();

            // Get sorted pages
            const sortedPages = getSortedPages();

            tbody.innerHTML = sortedPages.map(page => `
                <tr>
                    <td>
                        <a class="page-name" onclick="viewPage('${escapeHtml(page.name)}')">${escapeHtml(page.name)}</a>
                        <br>
                        <span class="text-secondary">${page.fileCount} file${page.fileCount !== 1 ? 's' : ''}</span>
                    </td>
                    <td class="text-secondary">${formatDate(page.createdAt)}</td>
                    <td class="text-secondary">${formatDate(page.lastModified)}</td>
                    <td><span class="size-badge">${formatSize(page.totalSize)}</span></td>
                    <td>
                        <div class="actions">
                            <button class="btn btn-sm btn-warning" onclick="renamePage('${escapeHtml(page.name)}')">Rename</button>
                            <button class="btn btn-sm btn-icon btn-danger" onclick="deletePage('${escapeHtml(page.name)}')" title="Delete">&#128465;</button>
                        </div>
                    </td>
                </tr>
            `).join('');
        }

        // Escape HTML to prevent XSS
        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        // Load pages from API
        async function loadPages() {
            try {
                const loading = document.getElementById('loading');
                loading.style.display = 'block';

                if (!window.dashApp) {
                    throw new Error('dashApp not available');
                }

                pages = await window.dashApp.listPages();
                initFuse();
                updateFilteredPages();
                renderPages();
            } catch (error) {
                console.error('Failed to load pages:', error);
                showStatus('Failed to load pages: ' + error.message, true);
                document.getElementById('loading').style.display = 'none';
            }
        }

        // Refresh pages
        function refreshPages() {
            loadPages();
        }

        // View page in new window
        async function viewPage(pageName) {
            try {
                await window.dashApp.viewPage(pageName);
                showStatus('Opening page: ' + pageName);
            } catch (error) {
                console.error('Failed to view page:', error);
                showStatus('Failed to view page: ' + error.message, true);
            }
        }

        // Delete page - show confirmation modal
        function deletePage(pageName) {
            pageToDelete = pageName;
            document.getElementById('delete-page-name').textContent = pageName;
            document.getElementById('delete-modal').classList.add('visible');
        }

        // Close delete modal
        function closeDeleteModal() {
            pageToDelete = null;
            document.getElementById('delete-modal').classList.remove('visible');
        }

        // Confirm delete
        async function confirmDelete() {
            if (!pageToDelete) return;

            const pageName = pageToDelete;
            closeDeleteModal();

            try {
                await window.dashApp.deletePage(pageName);
                showStatus('Deleted page: ' + pageName);

                // Refresh the list
                await loadPages();
            } catch (error) {
                console.error('Failed to delete page:', error);
                showStatus('Failed to delete page: ' + error.message, true);
            }
        }

        // Rename page - show rename modal
        function renamePage(pageName) {
            pageToRename = pageName;
            document.getElementById('rename-page-old-name').textContent = pageName;
            document.getElementById('rename-new-name').value = pageName;
            document.getElementById('rename-modal').classList.add('visible');
            // Focus and select the input
            setTimeout(() => {
                const input = document.getElementById('rename-new-name');
                input.focus();
                input.select();
            }, 100);
        }

        // Close rename modal
        function closeRenameModal() {
            pageToRename = null;
            document.getElementById('rename-modal').classList.remove('visible');
            document.getElementById('rename-new-name').value = '';
        }

        // Confirm rename
        async function confirmRename() {
            if (!pageToRename) return;

            const oldName = pageToRename;
            const newName = document.getElementById('rename-new-name').value.trim();

            if (!newName) {
                showStatus('Please enter a new name', true);
                return;
            }

            if (newName === oldName) {
                closeRenameModal();
                return;
            }

            closeRenameModal();

            try {
                await window.dashApp.renamePage(oldName, newName);
                showStatus('Renamed page to: ' + newName);

                // Refresh the list
                await loadPages();
            } catch (error) {
                console.error('Failed to rename page:', error);
                showStatus('Failed to rename page: ' + error.message, true);
            }
        }

        // Close modal on backdrop click
        document.getElementById('delete-modal').addEventListener('click', function(e) {
            if (e.target === this) {
                closeDeleteModal();
            }
        });

        // Close rename modal on backdrop click
        document.getElementById('rename-modal').addEventListener('click', function(e) {
            if (e.target === this) {
                closeRenameModal();
            }
        });

        // Close modal on Escape key
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape') {
                closeDeleteModal();
                closeRenameModal();
            }
            // Submit rename on Enter when rename modal is open and input is focused
            if (e.key === 'Enter' && document.activeElement.id === 'rename-new-name') {
                confirmRename();
            }
        });

        // Initialize on load
        window.addEventListener('DOMContentLoaded', () => {
            console.log('[PagesManager] Initializing...');

            // Wait a bit for dashApp to be injected
            setTimeout(() => {
                if (window.dashApp) {
                    console.log('[PagesManager] dashApp available, loading pages...');
                    loadPages();
                } else {
                    console.error('[PagesManager] dashApp not available!');
                    showStatus('Error: dashApp API not available', true);
                    document.getElementById('loading').style.display = 'none';
                }
            }, 100);
        });
    </script>
</body>
</html>"##;

    // Concatenate all parts: html_start + theme_css + css_rest
    html_start.to_string() + theme_css + css_rest
}

/// Spawn a Pages Manager webview window
///
/// # Arguments
/// * `is_dark_theme` - If true, use dark theme colors; otherwise use light theme
pub fn spawn_pages_manager_window(is_dark_theme: bool) -> std::io::Result<()> {
    let html = generate_pages_manager_html(is_dark_theme);
    crate::app::webview::spawn_webview_process_with_html(html, "AWS Dash Pages".to_string())
}
