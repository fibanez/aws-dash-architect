# AWS Resource Explorer - User Guide

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Resource Querying](#resource-querying)
4. [Tag Filtering](#tag-filtering)
5. [Tag Grouping](#tag-grouping)
6. [Bookmarks](#bookmarks)
7. [Bookmark Folders](#bookmark-folders)
8. [Keyboard Shortcuts](#keyboard-shortcuts)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The AWS Resource Explorer provides a unified view of your AWS resources across multiple accounts and regions. It enables powerful filtering, grouping, and organization capabilities through tags and bookmarks.

### Key Features

- **Multi-Account/Region Queries**: Query resources across hundreds of AWS accounts and dozens of regions
- **Advanced Tag Filtering**: Filter resources using complex boolean logic (AND/OR/NOT)
- **Hierarchical Tag Grouping**: Organize resources by tag hierarchies (e.g., Environment → Team → Project)
- **Bookmark System**: Save and organize frequently used queries with folder support
- **Session Persistence**: Automatically restores your last Explorer state
- **Concurrent Tag Fetching**: Efficient parallel tag loading with 15-minute cache

### Supported Resource Types

The Explorer supports 174 AWS resource types across all major services:
- **Compute**: EC2, Lambda, ECS, EKS, Fargate, App Runner
- **Storage**: S3, EBS, EFS, FSx
- **Database**: RDS, DynamoDB, ElastiCache, Neptune, DocumentDB, Redshift
- **AI/ML**: Bedrock (Models, Agents, Knowledge Bases), SageMaker
- **Networking**: VPC, Subnets, Security Groups, Load Balancers, Transit Gateways
- **Security**: IAM, KMS, Secrets Manager, ACM, GuardDuty, Security Hub
- **Management**: CloudFormation, CloudTrail, Config, Organizations
- **Analytics**: Athena, Glue, QuickSight, Kinesis, Data Brew
- **And many more...**

---

## Getting Started

### Opening the Explorer

1. **Command Palette**: Press `Space` and type "Explorer" or "Open AWS Explorer"
2. **Menu**: Navigate to Windows → AWS Resource Explorer

### Initial Setup

On first launch, you'll need to select:

1. **Accounts**: Click "Add Account" and select AWS accounts from the fuzzy search dialog
2. **Regions**: Click "Add Region" and select AWS regions (e.g., us-east-1, eu-west-1)
3. **Resource Types**: Click "Add Resource Type" and select resource types to query

**Tip**: The Explorer auto-populates with your current project's accounts and regions if a project is open.

### Querying Resources

Once you've selected accounts, regions, and resource types:

1. Click the **"Query Resources"** button (or press `Ctrl+Q`)
2. A loading spinner will appear while resources are fetched
3. Resources appear in a tree view based on your selected grouping mode
4. Query results are cached for the session (until app restart)

### Refresh Dialog

To refresh specific combinations without clearing the entire cache:

1. Click **"Refresh"** button
2. Select specific account/region/resource combinations to re-query
3. Use "Select All" or "Clear All" for convenience
4. Click "Refresh Selected" to re-query only those combinations

---

## Resource Querying

### Active Selection Display

Your current query scope is shown as colored tags:

- **Account Tags**: Display account name and ID
- **Region Tags**: Display region name and code
- **Resource Type Tags**: Display service and resource type

Each tag has an **X button** to remove it from the query.

### Search and Filtering

**Global Search** (top of window):
- Type to search across all resource properties
- Filters in real-time as you type
- Highlights matching text in yellow
- Searches: names, IDs, tags, ARNs, descriptions

**Search Behavior**:
- Minimum 3 characters required to start filtering (performance optimization)
- Case-insensitive matching
- Preserves parent hierarchy for matched resources

---

## Tag Filtering

### Tag Filter Types

The Explorer supports 11 filter types for sophisticated tag-based queries:

| Filter Type | Description | Example |
|------------|-------------|---------|
| **Equals** | Exact tag value match | Environment = prod |
| **Not Equals** | Tag value does not match | Environment ≠ dev |
| **In** | Tag value in list | Environment IN [prod, staging] |
| **Not In** | Tag value not in list | Environment NOT IN [dev, test] |
| **Exists** | Resource has tag key (any value) | CostCenter EXISTS |
| **Not Exists** | Resource missing tag key | Owner NOT EXISTS |
| **Contains** | Tag value contains substring | Project CONTAINS api |
| **Not Contains** | Tag value doesn't contain substring | Team NOT CONTAINS temp |
| **Starts With** | Tag value starts with prefix | Name STARTS WITH web- |
| **Ends With** | Tag value ends with suffix | Name ENDS WITH -prod |
| **Regex** | Tag value matches regex pattern | Version MATCHES ^v[0-9]+\. |

### Quick Filters

At the top of the Tag Filters section:

- **Show only tagged resources**: Hide all resources without any tags
- **Show only untagged resources**: Show only resources with no tags
- **Note**: These checkboxes are mutually exclusive

### Advanced Tag Filter Builder

Click **"Advanced Filters..."** to open the visual query builder:

#### Adding Filters

1. Click **"+ Add Filter"** to create a new filter row
2. Select **Tag Key** from dropdown (autocomplete with usage counts)
3. Select **Filter Type** (Equals, Contains, Exists, etc.)
4. Enter **Value(s)** if applicable (autocomplete with frequency counts)
5. Click **X** on any row to delete that filter

#### Boolean Logic

**AND/OR Operators**:
- Toggle between **AND** (all filters must match) or **OR** (any filter can match)
- Operator applies to all filters at the current level

**Nested Groups**:
1. Click **"Add Group"** to create a nested condition
2. Each group has its own AND/OR operator
3. Groups are visually indented with borders
4. Supports unlimited nesting (recommended max: 5 levels)

**Example Complex Filter**:
```
(Environment=prod AND Team=backend) OR (Environment=staging AND Team=frontend)
```

This shows:
- All prod backend resources, OR
- All staging frontend resources

#### Filter Summary

At the bottom of the filter builder:
- **Preview**: Shows readable summary of your filter logic
- **Copy Button**: Copies summary to clipboard for documentation

#### Applying Filters

1. Click **"Apply"** to activate filters and close dialog
2. Click **"Cancel"** to discard changes
3. Click **"Clear All"** to remove all filters

---

## Tag Grouping

### Grouping Modes

Resources can be grouped by:

**Built-in Groupings**:
- **Account**: Group by AWS account
- **Region**: Group by AWS region
- **Resource Type**: Group by AWS service and resource type

**Tag Groupings**:
- **Tag: [TagName]**: Group by a single tag key's values
- **Tag Hierarchy...**: Group by multiple tags in hierarchical order

### Single Tag Grouping

**Automatic Tag Discovery**:
- The GroupBy dropdown automatically shows discovered tag keys
- Only shows tags with 2+ unique values (can't group single value)
- Shows usage statistics: "Tag: Environment (45 resources, 3 values)"
- Filtered by minimum resource count (default: 1, configurable 1-100)

**Example**: Selecting "Tag: Environment" creates groups:
```
├─ Environment: prod (45 resources)
├─ Environment: staging (30 resources)
├─ Environment: dev (12 resources)
└─ No Environment (8 resources)  ← Resources without Environment tag
```

**Visual Indicators**:
- **Blue tags**: Resources with the tag present
- **Gray tags**: Resources missing the tag ("No Environment")
- Missing tag groups appear at the bottom

### Hierarchical Tag Grouping

**Opening the Hierarchy Builder**:
1. Select **"Tag Hierarchy..."** from GroupBy dropdown
2. The Tag Hierarchy Builder dialog opens

**Two-Panel Layout**:

**Left Panel: Available Tag Keys**
- Lists all discovered tag keys with statistics
- Shows resource count: "Environment (45 resources)"
- Shows value count: "Environment (3 values)"
- Search box for filtering
- Grayed out if already in hierarchy

**Right Panel: Selected Hierarchy**
- Shows tag keys in hierarchical order (top to bottom)
- Drag handle (**::**) for reordering
- Delete button (**X**) to remove
- Empty state: "Drag tag keys here or double-click to add"
- Maximum 5 levels

**Building a Hierarchy**:

Method 1: Drag-and-Drop
- Drag tag key from left panel to right panel
- Drag within right panel to reorder

Method 2: Double-Click
- Double-click tag key in left panel to add

**Example Hierarchy**:
```
1. Environment
   ↓
2. Team
   ↓
3. Project
```

Creates tree structure:
```
├─ Environment: prod
│  ├─ Team: backend
│  │  ├─ Project: api (12 resources)
│  │  └─ Project: database (8 resources)
│  └─ Team: frontend
│     └─ Project: web (15 resources)
├─ Environment: staging
│  └─ Team: backend
│     ├─ Project: api (5 resources)
│     └─ No Project (3 resources)
└─ No Environment (7 resources)
```

**Preview Section**:
- Shows sample tree structure with your hierarchy
- Updates in real-time as you change the hierarchy
- Helps visualize the grouping before applying

**Applying the Hierarchy**:
1. Click **"Apply"** to activate the hierarchy
2. Click **"Cancel"** to close without changes

**Minimum Resource Count**:
- Configure how many resources a tag must have to appear in GroupBy
- Default: 1 (all tags shown)
- Range: 1-100
- Helps reduce clutter with rarely-used tags

---

## Bookmarks

### What are Bookmarks?

Bookmarks save your complete Explorer state:
- Selected accounts, regions, and resource types
- Active tag filters (including nested boolean logic)
- Tag grouping configuration (single tag or hierarchy)
- Search filter text
- Grouping mode

### Creating a Bookmark

**Method 1: Bookmark Bar**
1. Configure Explorer with desired state (query, filters, grouping)
2. Click **"+"** button in bookmark bar
3. Enter bookmark name and optional description
4. Click **"Save"**

**Method 2: Keyboard Shortcut**
1. Press `Ctrl+D` to open bookmark dialog
2. Enter name and description
3. Click **"Save"**

**Bookmark Icon**:
- Auto-assigned emoji based on resource types
- Can customize in Bookmark Manager

### Using Bookmarks

**Click to Apply**:
- Click any bookmark in the bar to apply its saved state
- Explorer immediately updates to match the bookmark
- Active bookmark is highlighted with a border

**Access Statistics**:
- Bookmark tracks usage count
- Last accessed timestamp
- Most frequently used bookmarks can be identified

### Bookmark Bar

**Layout**:
- Horizontal bar below main controls
- Always visible
- Scrollable if more bookmarks than fit

**Features**:
- **Click**: Apply bookmark
- **Drag**: Reorder bookmarks (drag by ":: " handle)
- **Right-click**: Context menu (Edit, Delete, Copy, Cut, Paste)
- **Overflow**: Additional bookmarks accessible via "..." menu

**Drag-and-Drop**:
- Each bookmark has a ":: " drag handle
- Drag to reorder within bookmark bar
- Drag into folders in Bookmark Manager
- Visual feedback shows valid drop zones

### Bookmark Manager

**Opening the Manager**:
1. Click **"Manage Bookmarks"** button
2. Or press `Ctrl+Shift+B`

**Manager Layout**:

**Top Section**: Folder controls
- **"New Folder"** button: Create bookmark folder
- **Right-click on folder**: Rename or delete

**Tree View**: Hierarchical folder structure
- **🗁 Top Folder**: Root level (drop target for un-nesting)
- **Folders**: Collapsible with arrow icons
- **Bookmarks**: Listed under their parent folder
- **Drag handles (":: ")**: Drag folders or bookmarks to reorganize

**Bookmark Details** (when selected):
- Name and description
- Icon (emoji)
- Creation and modification dates
- Access count and last accessed time
- Full configuration preview (accounts, regions, filters, etc.)

**Actions**:
- **Edit**: Modify name, description, icon
- **Delete**: Remove bookmark (confirmation required)
- **Copy/Cut/Paste**: Organize bookmarks via context menus
- **Move**: Drag-and-drop to different folders

### Auto-Save Feature

**Automatic Session Persistence**:
- Explorer automatically saves your current state when closing
- State restored when reopening Explorer
- Includes all filters, grouping, and selections

**What Gets Saved**:
- Account/region/resource selections
- Active tag filters
- Tag grouping configuration
- Search text
- Tree expansion state
- Active bookmark

**Note**: Auto-save is transparent - no manual action needed.

---

## Bookmark Folders

### Creating Folders

**From Bookmark Manager**:
1. Click **"New Folder"** button
2. Enter folder name
3. Select parent folder (or Top Folder for root level)
4. Click **"Create"**

**Folder Nesting**:
- Unlimited nesting depth
- Folders can contain both subfolders and bookmarks
- Example structure:
  ```
  🗁 Top Folder
  ├─ 📁 Production
  │  ├─ 📁 Backend Services
  │  │  ├─ API Bookmark
  │  │  └─ Database Bookmark
  │  └─ 📁 Frontend
  ├─ 📁 Development
  └─ 📁 Archived
  ```

### Organizing with Drag-and-Drop

**Moving Bookmarks**:
1. Locate bookmark in tree view
2. Drag by the ":: " handle
3. Drop onto target folder or "Top Folder"
4. Bookmark moves to new location

**Moving Folders**:
1. Locate folder in tree view
2. Drag by the ":: " handle
3. Drop onto target parent folder or "Top Folder"
4. Folder (and all contents) moves to new location

**Visual Feedback**:
- **Blue highlight**: Valid drop target
- **No highlight**: Invalid drop (circular reference prevented)

**Drop on "Top Folder"**:
- Moves item to root level (un-nesting)
- "Top Folder" always accepts drops

**Circular Reference Prevention**:
- Cannot drag folder into its own descendant
- System prevents invalid moves automatically

### Folder Context Menu

**Right-click on any folder**:
- **Paste Bookmark Here**: Paste copied/cut bookmark into folder
- **Rename Folder**: Change folder name
- **Delete Folder**: Remove folder (must be empty)

**Delete Protection**:
- Cannot delete folder with contents
- Must move or delete all bookmarks/subfolders first

### Collapse/Expand State

**Arrow Icons**:
- **▼**: Folder expanded (contents visible)
- **▶**: Folder collapsed (contents hidden)
- Click arrow to toggle

**State Persistence**:
- Expansion state saved across sessions
- Reopen Manager with same folders expanded

---

## Keyboard Shortcuts

### Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `Space` | Open Command Palette |
| `Ctrl+Q` | Query Resources |
| `Ctrl+D` | Create Bookmark from current state |
| `Ctrl+Shift+B` | Open Bookmark Manager |
| `Ctrl+F` | Focus search box |
| `Esc` | Clear search / Close dialogs |

### Fuzzy Search Dialogs

| Shortcut | Action |
|----------|--------|
| `↑/↓` | Navigate items |
| `Enter` | Select highlighted item |
| `Esc` | Cancel and close |
| `Double-click` | Select item |

### Bookmark Manager

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Copy selected bookmark |
| `Ctrl+X` | Cut selected bookmark |
| `Ctrl+V` | Paste bookmark into selected folder |
| `Delete` | Delete selected bookmark/folder |
| `F2` | Rename selected folder |
| `Right-click` | Open context menu |

### Tree Navigation

| Shortcut | Action |
|----------|--------|
| `Click arrow` | Expand/collapse folder |
| `Click resource` | Select resource |
| `Ctrl+A` | Collapse/Expand All (in tag hierarchy preview) |

---

## Troubleshooting

### Common Issues

#### Tags Not Appearing

**Symptoms**: Resources show no tags in tree view

**Causes & Solutions**:

1. **Tag fetch failure**:
   - Check logs: `$HOME/.local/share/awsdash/logs/awsdash.log`
   - Look for: "Failed to fetch tags" warnings
   - Solution: Verify AWS credentials and permissions

2. **Permission denied**:
   - Error: "Access denied when fetching tags"
   - Solution: Ensure IAM role has `tag:GetResources` permission
   - Required for Resource Groups Tagging API

3. **Service-specific tag APIs**:
   - Some services (EC2, S3, Lambda, IAM) use service-specific tag APIs
   - Solution: Ensure role has service-specific tag permissions (e.g., `ec2:DescribeTags`)

4. **Cache issues**:
   - Tags cached for 15 minutes
   - Solution: Click "Refresh" and select resources to clear cache
   - Or restart application to clear all caches

**Verification**:
- Query a resource you know has tags (e.g., manually tagged EC2 instance)
- If still no tags, check CloudTrail for API denials

#### Drag-and-Drop Not Working

**Symptoms**: Cannot drag bookmarks or folders

**Causes & Solutions**:

1. **Not using drag handle**:
   - Solution: Drag by the ":: " handle icon, not the text

2. **Circular reference**:
   - Cannot drag folder into its own descendant
   - Solution: Move to different folder or Top Folder

3. **Arrow interference**:
   - Clicking collapse arrow doesn't drag
   - Solution: Use drag handle, not the arrow icon

4. **UI state issue**:
   - Rare: drag state stuck
   - Solution: Close and reopen Bookmark Manager

#### Search Not Filtering

**Symptoms**: Typing in search box doesn't filter resources

**Causes & Solutions**:

1. **Too few characters**:
   - Search requires minimum 3 characters (performance optimization)
   - Solution: Type at least 3 characters

2. **No matches**:
   - Search text doesn't match any resource properties
   - Solution: Try different search terms or check spelling

3. **Conflicting filters**:
   - Tag filters might exclude all resources before search applies
   - Solution: Clear tag filters and try search alone

#### Bookmarks Not Saving

**Symptoms**: Bookmarks disappear after restart

**Causes & Solutions**:

1. **File permission error**:
   - Check logs for "Failed to save bookmarks" errors
   - Solution: Verify write permissions for `~/.config/awsdash/`

2. **Corrupted bookmark file**:
   - File: `~/.config/awsdash/bookmarks.json`
   - Solution: Backup and delete corrupted file, restart application

3. **Auto-save failure**:
   - Check logs for save errors
   - Solution: Manually trigger save by editing a bookmark

**Recovery**:
- Backup file location: `~/.config/awsdash/bookmarks.json.backup`
- Restore by renaming backup to `bookmarks.json`

#### Tag Filters Not Working

**Symptoms**: Tag filters applied but resources not filtered correctly

**Causes & Solutions**:

1. **Boolean logic error**:
   - Complex nested filters can be confusing
   - Solution: Use filter summary to verify logic
   - Test simple filters first, then add complexity

2. **Case sensitivity**:
   - Tag filter matching is case-insensitive for "Contains"
   - But case-sensitive for "Equals"
   - Solution: Check exact tag value casing

3. **Missing tag handling**:
   - "Not Exists" filter shows resources WITHOUT tag
   - "Exists" filter shows resources WITH tag (any value)
   - Solution: Verify filter type matches intent

4. **Regex errors**:
   - Invalid regex pattern causes filter to fail
   - Solution: Test regex pattern separately
   - Use simple patterns first

### Performance Issues

#### Slow Query Performance

**Symptoms**: Queries take > 30 seconds

**Causes & Solutions**:

1. **Too many combinations**:
   - 10 accounts × 10 regions × 10 resource types = 1,000 API calls
   - Solution: Reduce scope (fewer accounts/regions/types per query)

2. **No cache hits**:
   - First query always slower (no cache)
   - Solution: Subsequent queries use cache (15-min TTL)

3. **Network latency**:
   - Remote regions have higher latency
   - Solution: Query fewer regions simultaneously

**Optimization Tips**:
- Query frequently used combinations first
- Use bookmarks to save common queries
- Refresh selectively (not full re-query)

#### Slow Tag Filtering

**Symptoms**: Tag filter changes cause UI lag

**Causes & Solutions**:

1. **Large resource set**:
   - Filtering 1,000+ resources can take time
   - Solution: Narrow query scope before applying complex filters

2. **Complex regex patterns**:
   - Regex evaluation is slower than simple filters
   - Solution: Use simpler filter types when possible

3. **Deep nesting**:
   - 5+ levels of nested filter groups
   - Solution: Simplify filter logic to 2-3 levels max

#### Tree Rendering Issues

**Symptoms**: Tree expansion/collapse is slow

**Causes & Solutions**:

1. **Too many resources**:
   - 10,000+ resources in single tree
   - Solution: Use tag filters to reduce visible resources
   - Solution: Use tag grouping to organize into smaller groups

2. **Frequent rebuilds**:
   - Tree rebuilds on every filter change
   - Solution: Cache optimization already implemented
   - System only rebuilds when data actually changes

### Error Messages

#### "No resources match tag filters"

**Meaning**: All resources filtered out by tag filters

**Solution**:
- Click "Clear Tag Filters" to reset
- Review filter summary for overly restrictive logic
- Try broader filter (OR instead of AND)

#### "Access denied when querying [service]"

**Meaning**: IAM role lacks permission for service API

**Solution**:
- Verify IAM role has service-specific read permissions
- Check: `[service]:Describe*`, `[service]:List*`
- Example: `ec2:DescribeInstances`, `s3:ListBuckets`

#### "Failed to fetch tags for resource"

**Meaning**: Tag API call failed for specific resource

**Solution**:
- Resource may not support tags (rare)
- Permission may be denied for that resource type
- Check logs for specific error message

#### "Cannot delete folder with contents"

**Meaning**: Folder contains bookmarks or subfolders

**Solution**:
- Move contents to other folders
- Or delete contents first
- Then delete empty folder

### Getting Help

**Check Logs**:
- Location: `$HOME/.local/share/awsdash/logs/awsdash.log`
- Search for: ERROR, WARN, "Failed to"
- Include relevant log excerpts when reporting issues

**Debug Mode**:
- Enable verbose logging in application settings
- Restart application
- Reproduce issue
- Check logs for detailed trace

**Report Issues**:
- GitHub: https://github.com/anthropics/aws-dash/issues
- Include: OS, AWS services affected, log excerpts
- Describe: Expected vs actual behavior

---

## Best Practices

### Tag Strategy

**Consistent Tag Keys**:
- Use standard tag keys across all resources
- Example: Environment, Team, Project, CostCenter, Owner
- Enables effective grouping and filtering

**Tag Values**:
- Keep values concise and consistent
- Use lowercase or Title Case consistently
- Avoid spaces (use hyphens: `web-api` not `web api`)

**Hierarchical Tags**:
- Design tag hierarchy for your organization structure
- Example: Environment → Team → Project → Component
- Maximum 3-5 levels for usability

### Bookmark Organization

**Folder Structure**:
```
🗁 Top Folder
├─ 📁 By Environment
│  ├─ 📁 Production
│  ├─ 📁 Staging
│  └─ 📁 Development
├─ 📁 By Team
│  ├─ 📁 Backend
│  ├─ 📁 Frontend
│  └─ 📁 DevOps
└─ 📁 Troubleshooting
   ├─ 📁 High CPU
   ├─ 📁 Network Issues
   └─ 📁 Security Alerts
```

**Naming Conventions**:
- Use descriptive names: "Prod Backend APIs" not "Query 1"
- Include environment: "Staging Frontend Servers"
- Add purpose: "Troubleshooting: High Memory RDS"

**Descriptions**:
- Document filter logic
- Example: "All production EC2 instances in us-east-1 with Team=backend tag"
- Helps team members understand bookmark purpose

### Query Optimization

**Start Broad, Filter Narrow**:
1. Query all resource types in region
2. Use tag filters to narrow results
3. Create bookmark for frequently used combination

**Use Cache Effectively**:
- Query once, filter multiple times (in-memory)
- Refresh only when data changes needed
- 15-minute cache window is usually sufficient

**Bookmark Common Queries**:
- Save time with one-click access
- Reduces API calls (bookmarks use cached data)
- Share bookmarks with team (export/import)

---

## Advanced Tips

### Finding Untagged Resources

**Quick Method**:
1. Check "Show only untagged resources"
2. Review list of resources without any tags
3. Add tags to improve organization

**Comprehensive Method**:
1. Create filter: `[TagKey] NOT EXISTS`
2. Repeat for each required tag key
3. Combine with AND logic
4. Find resources missing critical tags

### Tag Compliance Auditing

**Required Tags Check**:
```
CostCenter NOT EXISTS OR
Owner NOT EXISTS OR
Environment NOT EXISTS
```

**Result**: All resources missing any required tag

**Create Bookmark**: "Tag Compliance Issues"

### Cross-Account Analysis

**Query Setup**:
1. Add multiple accounts
2. Add all regions
3. Select specific resource type (e.g., EC2::Instance)
4. Group by: Account
5. Secondary group by: Tag: Environment

**Result**: See environment distribution across all accounts

### Tag Value Standardization

**Find Inconsistencies**:
1. Group by: Tag: Environment
2. Look for variations: "prod", "Prod", "production", "PROD"
3. Fix tags in AWS Console
4. Refresh Explorer to verify

**Note**: Tag filters are case-sensitive for "Equals", case-insensitive for "Contains"

---

## Appendix

### File Locations

**Configuration**:
- Bookmarks: `~/.config/awsdash/bookmarks.json`
- Settings: `~/.config/awsdash/settings.json`

**Logs**:
- Application log: `~/.local/share/awsdash/logs/awsdash.log`
- Previous log: `~/.local/share/awsdash/logs/awsdash.log.old`

**Cache**:
- Session cache: In-memory only (cleared on restart)
- Tag cache: In-memory, 15-minute TTL

### Tag Cache Behavior

**Cache Key**: `{resource_arn}`

**TTL**: 15 minutes (900 seconds)

**Size Limit**: 10,000 entries (LRU eviction)

**Invalidation**:
- Automatic: After 15 minutes
- Manual: Click "Refresh" button
- Full: Restart application

**Performance**:
- Cache hit: < 1ms (no API call)
- Cache miss: 50-200ms (API call)
- Hit rate target: > 80% for repeat queries

### Supported Boolean Operators

**Filter Group Operators**:
- **AND**: All conditions must be true
- **OR**: Any condition can be true

**Nesting**:
- Groups can contain filters and sub-groups
- Sub-groups evaluated recursively
- Unlimited depth (recommended max: 5)

**Short-Circuit Evaluation**:
- AND: Stops at first false (performance optimization)
- OR: Stops at first true (performance optimization)

### AWS API Calls

**Query Phase**:
- Per account/region/resource combination: 1-2 API calls
- Example: DescribeInstances, DescribeDBInstances

**Tag Fetching**:
- Default: Resource Groups Tagging API (`GetResources`)
- Service-specific: EC2, S3, Lambda, IAM use dedicated APIs
- Parallel: All tags fetched concurrently
- Cached: 15-minute TTL

**Rate Limiting**:
- Explorer handles AWS API rate limits automatically
- Retries with exponential backoff on throttling
- No user action required

---

**Document Version**: 1.0
**Last Updated**: 2025-11-14
**Compatible with**: AWS Dash Explorer v0.1.0+

