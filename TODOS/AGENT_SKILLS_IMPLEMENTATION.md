# Agent Skills Implementation TODO
## Anthropic-style SKILL.md System for AWS Orchestration Agent

**Created**: 2025-01-28
**Status**: Planning
**Priority**: High
**References**:
- Anthropic: "Equipping agents for the real world with Agent Skills"
- Anthropic: "Claude Skills" announcement
- Current prompt analysis: `reference/PROMPT_ANALYSIS_REPORT.md`

---

## Architecture Philosophy: LLM-Managed Intelligence with Rules-Based Safety

### Core Design Principle: 90% LLM-Managed, 10% Rules-Based

This implementation follows Anthropic's philosophy: **"Give the LLM the intelligence to decide, but constrain the mechanics to be safe."**

#### What the LLM Controls (Autonomous Intelligence):

**1. Skill Recognition and Selection**
```
User: "My EC2 instance isn't responding"
↓
LLM semantic reasoning:
  - "not responding" = connectivity/availability issue
  - "EC2 instance" = AWS compute resource
  - Matches: "aws-ec2-troubleshooting: Diagnose EC2 instance issues"
↓
LLM decision: invoke_skill('aws-ec2-troubleshooting')
```

**Key Point**: No keyword matching, no rigid rules. The LLM semantically understands task intent and matches it to skill descriptions using its language understanding capabilities.

**2. Skill Application and Adaptation**
```
Skill loaded: "Check instance status → network config → storage"
↓
LLM interprets as guidance, not rigid steps:
  - User already provided region → skip asking
  - User mentioned "can't SSH" → prioritize network checks
  - Instance stopped → skip performance checks
↓
LLM adapts workflow to user context
```

**Key Point**: Skills provide expert procedures, but LLM decides how to apply them based on user context, not blind execution.

**3. Multi-Skill Orchestration**
```
User: "Lambda timing out and CloudWatch logs are huge"
↓
LLM recognizes two problems:
  - Matches: aws-lambda-optimization (timeout issue)
  - Matches: aws-cloudwatch-analysis (log volume issue)
↓
LLM decides: Load both, create parallel task agents
```

**Key Point**: LLM determines which skills to combine and how to orchestrate them.

#### What Rules Control (Safety and Structure):

**1. Security Boundaries (Hard Constraints)**
```rust
// Rule: Only read from ~/.claude/skills/ and ~/.awsdash/skills/
// Rule: Block path traversal (../, symlinks)
// Rule: Max file size 10MB
// Rule: Never read credentials, /etc/passwd, etc.
```

**2. Skill Discovery and Format (Deterministic)**
```rust
// Rule: Skills must be in SKILL.md files
// Rule: YAML frontmatter required: name, description
// Rule: Directory depth max = 2 levels
```

**3. Tool Mechanics (Fixed Operations)**
```rust
// Rule: read_file requires absolute paths
// Rule: invoke_skill only loads from discovered skills
// Rule: list_directory cannot escape allowed directories
```

### Why This Hybrid Approach?

**Comparison to Rules-Based Alternative** (NOT our approach):
```rust
// Rigid keyword matching (ANTI-PATTERN)
if user_message.contains("EC2") && user_message.contains("not responding") {
    load_skill("aws-ec2-troubleshooting");
    execute_diagnostic_workflow();  // No adaptation
}
```

**Problems with Pure Rules**:
- ❌ Brittle: Fails on "instance down", "server offline", "can't connect"
- ❌ No context awareness: Same response for "EC2 slow" vs "EC2 crashed"
- ❌ No creativity: Can't combine skills or deviate from script

**Advantages of LLM-Managed**:
- ✅ Flexible: Understands synonyms, paraphrasing, context
- ✅ Adaptive: Adjusts to user-provided information
- ✅ Creative: Combines skills, adapts procedures, handles edge cases

**Advantages of Rules-Based Safety**:
- ✅ Secure: LLM cannot escape file system restrictions
- ✅ Predictable: Skill format is guaranteed
- ✅ Auditable: Security boundaries are code-enforced

### Token Economics

**Current System (No Skills)**:
```
System prompt: 2,000 tokens (all instructions inline)
Every agent: 2,000 tokens
```

**With Skills (Progressive Disclosure)**:
```
Startup:
  Base prompt: 800 tokens
  Skill metadata (5 skills): 200 tokens
  Total: 1,000 tokens (-50%)

Simple task (no skill needed):
  Base prompt: 800 tokens only
  Savings: 60%

Specialist task (skill loaded):
  Base: 800 tokens
  Metadata: 200 tokens
  Loaded skill: 5,000 tokens
  Total: 6,000 tokens (only when needed)
```

**Key Insight**: Pay token cost only for specialist knowledge when needed, not every interaction.

---

## Executive Summary

Implement Anthropic's Agent Skills system to enhance the AWS Orchestration Agent with:
1. **Progressive disclosure**: Load skill metadata at startup, full content on-demand
2. **Composability**: Skills stack automatically with agent coordination
3. **Efficiency**: Reduce context bloat while maintaining specialized capabilities
4. **Reusability**: Package AWS expertise into portable SKILL.md files

**Expected Impact**:
- -60% context token usage (progressive disclosure vs. full prompt)
- +40% specialized task performance (domain expertise in skills)
- 10x faster skill iteration (edit SKILL.md vs. recompile agent)

---

## Project Milestones

### Milestone 1: Tool Infrastructure (Week 1-2)
**Goal**: Build file system tools for skill discovery and loading
**Status**: Not Started
**Estimated Effort**: 40 hours

### Milestone 2: Skill System Core (Week 3-4)
**Goal**: Implement skill discovery, loading, and progressive disclosure
**Status**: ✅ COMPLETED (2025-01-29)
**Estimated Effort**: 60 hours
**Actual Effort**: ~12 hours (leveraged existing patterns)

### Milestone 3: Prompt Enhancement (Week 5)
**Goal**: Update Orchestration Agent prompt with skill capabilities
**Status**: ✅ COMPLETED (2025-01-29)
**Estimated Effort**: 20 hours
**Actual Effort**: ~2 hours (restructuring only)

### Milestone 4: Example Skills (Week 6-7)
**Goal**: Create 5-10 AWS domain skills for validation
**Status**: Not Started
**Estimated Effort**: 40 hours

### Milestone 5: Integration & Testing (Week 8)
**Goal**: End-to-end testing with multi-skill scenarios
**Status**: Not Started
**Estimated Effort**: 20 hours

**Total Estimated Effort**: 180 hours (4-5 weeks full-time)

---

## Milestone 1: Tool Infrastructure

### Task 1.1: File Read Tool ⭐ CRITICAL
**Priority**: P0 - Blocking
**Estimated Effort**: 8 hours
**File**: `src/app/agent_framework/tools/file_read.rs`

**Description**:
Create tool for reading file contents from disk. Required for loading SKILL.md files.

**Implementation Details**:
```rust
// Tool: ReadFileTool
// Input: { file_path: String }
// Output: { content: String, size_bytes: usize }

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn description(&self) -> String {
        "Read the contents of a file from the filesystem.

        Input:
        - file_path: Absolute path to file (e.g., '/home/user/.claude/skills/aws-ec2/SKILL.md')

        Output:
        - content: Full file contents as string
        - size_bytes: File size in bytes

        Edge cases:
        - Returns error if file doesn't exist
        - Returns error if file size > 10MB (context protection)
        - Returns error if path is not absolute (security)
        - Returns error if path escapes allowed directories

        Security:
        - Only allows reading from: ~/.claude/skills/, ~/.awsdash/skills/
        - Rejects path traversal attempts (../, symlinks)
        - Never reads: /etc/, /root/, credential files

        Example:
        read_file('/home/user/.claude/skills/aws-ec2/SKILL.md')
        → Returns EC2 skill markdown content".to_string()
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // 1. Parse input
        let file_path = input.get_string("file_path")?;

        // 2. Security validation
        validate_file_path(&file_path)?;  // Absolute, within allowed dirs, no traversal

        // 3. Size check
        let metadata = fs::metadata(&file_path)
            .map_err(|e| ToolError::FileNotFound(e))?;
        if metadata.len() > 10_000_000 {  // 10MB limit
            return Err(ToolError::FileTooLarge);
        }

        // 4. Read content
        let content = fs::read_to_string(&file_path)
            .map_err(|e| ToolError::ReadError(e))?;

        // 5. Return
        Ok(ToolOutput {
            content,
            size_bytes: metadata.len()
        })
    }
}
```

**Testing Requirements**:
- Unit tests: valid path, invalid path, size limits, security boundaries
- Integration test: Read actual SKILL.md from test fixtures
- Edge case test: Path traversal attempts, symlink following

**Acceptance Criteria**:
- [ ] Tool registered in orchestration agent
- [ ] Security validation prevents path traversal
- [ ] Size limit enforced (10MB max)
- [ ] Returns clear error messages for invalid paths
- [ ] Logs file access with tracing

---

### Task 1.2: Directory List Tool
**Priority**: P0 - Blocking
**Estimated Effort**: 6 hours
**File**: `src/app/agent_framework/tools/list_directory.rs`

**Description**:
Create tool for listing directory contents. Required for skill discovery.

**Implementation Details**:
```rust
// Tool: ListDirectoryTool
// Input: { directory_path: String, pattern: Option<String> }
// Output: { files: Vec<FileEntry>, directories: Vec<String> }

pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: DateTime<Utc>,
}

impl Tool for ListDirectoryTool {
    fn name(&self) -> String {
        "list_directory".to_string()
    }

    fn description(&self) -> String {
        "List files and directories in a given path.

        Input:
        - directory_path: Absolute path to directory
        - pattern: Optional glob pattern (e.g., '*.md', 'SKILL.*')

        Output:
        - files: Array of files with metadata
        - directories: Array of subdirectory names

        Security:
        - Only allows listing: ~/.claude/skills/, ~/.awsdash/skills/
        - Rejects path traversal

        Example:
        list_directory('/home/user/.claude/skills/', pattern='SKILL.md')
        → Returns all SKILL.md files in subdirectories".to_string()
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        // Implementation similar to ReadFileTool with directory traversal
    }
}
```

**Testing Requirements**:
- Recursive directory listing
- Glob pattern filtering
- Security boundary enforcement

**Acceptance Criteria**:
- [ ] Tool registered in orchestration agent
- [ ] Supports recursive listing with pattern matching
- [ ] Returns file metadata (size, modified time)
- [ ] Security validation like ReadFileTool

---

### Task 1.3: File Path Validation Library
**Priority**: P0 - Blocking (used by 1.1, 1.2)
**Estimated Effort**: 4 hours
**File**: `src/app/agent_framework/tools/file_security.rs`

**Description**:
Create shared security validation for file operations.

**Implementation Details**:
```rust
// Security validation utilities

pub fn validate_file_path(path: &str) -> Result<PathBuf, SecurityError> {
    let path = PathBuf::from(path);

    // 1. Must be absolute
    if !path.is_absolute() {
        return Err(SecurityError::RelativePath);
    }

    // 2. Canonicalize (resolves symlinks, .., etc.)
    let canonical = path.canonicalize()
        .map_err(|_| SecurityError::InvalidPath)?;

    // 3. Check allowed directories
    let allowed_dirs = get_allowed_directories();
    if !is_within_allowed_dirs(&canonical, &allowed_dirs) {
        return Err(SecurityError::OutsideAllowedDirectory);
    }

    // 4. Check blocklist (credentials, system files)
    if is_sensitive_path(&canonical) {
        return Err(SecurityError::SensitiveFile);
    }

    Ok(canonical)
}

fn get_allowed_directories() -> Vec<PathBuf> {
    vec![
        dirs::home_dir().unwrap().join(".claude/skills"),
        dirs::home_dir().unwrap().join(".awsdash/skills"),
        // Add more as needed
    ]
}

fn is_sensitive_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/.aws/credentials") ||
    path_str.contains("/.ssh/") ||
    path_str.contains("/etc/passwd") ||
    // ... more sensitive paths
}
```

**Testing Requirements**:
- Test all security boundaries
- Test path traversal attempts
- Test symlink resolutionagentskill

**Acceptance Criteria**:
- [ ] Blocks path traversal
- [ ] Blocks sensitive file access
- [ ] Allows only skills directories
- [ ] Comprehensive unit tests

---

### Task 1.4: Tool Registration
**Priority**: P0 - Blocking
**Estimated Effort**: 2 hours
**File**: `src/app/agent_framework/tools_registry.rs`

**Description**:
Register new file tools in orchestration agent.

**Implementation Details**:
```rust
// Add to tools_registry.rs

pub fn read_file_tool() -> Box<dyn Tool> {
    Box::new(ReadFileTool::new())
}

pub fn list_directory_tool() -> Box<dyn Tool> {
    Box::new(ListDirectoryTool::new())
}

// Update orchestration_agent.rs builder
agent_builder
    .add_tool(read_file_tool())
    .add_tool(list_directory_tool())
    // ... existing tools
```

**Acceptance Criteria**:
- [ ] Tools available in orchestration agent
- [ ] Tools NOT available in task agents (security boundary)
- [ ] Tools documented in system prompt

---

## Milestone 2: Skill System Core

### Task 2.1: Skill Discovery Service ✅ COMPLETED
**Priority**: P0 - Blocking
**Estimated Effort**: 12 hours
**Actual Effort**: 3 hours
**File**: `src/app/agent_framework/skills/discovery.rs`

**Description**:
Implement skill discovery system that scans skill directories and extracts metadata.

**Implementation Details**:
```rust
// Skill metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub directory_path: PathBuf,
    pub skill_md_path: PathBuf,
    pub additional_files: Vec<String>,  // forms.md, reference.md, etc.
}

pub struct SkillDiscoveryService {
    skill_directories: Vec<PathBuf>,
    discovered_skills: RwLock<Vec<SkillMetadata>>,
}

impl SkillDiscoveryService {
    pub fn new() -> Self {
        Self {
            skill_directories: vec![
                dirs::home_dir().unwrap().join(".claude/skills"),
                dirs::home_dir().unwrap().join(".awsdash/skills"),
            ],
            discovered_skills: RwLock::new(Vec::new()),
        }
    }

    /// Scan all skill directories and discover skills
    pub fn discover_skills(&self) -> Result<Vec<SkillMetadata>, SkillError> {
        let mut skills = Vec::new();

        for skill_dir in &self.skill_directories {
            if !skill_dir.exists() {
                continue;
            }

            // Find all SKILL.md files
            for entry in WalkDir::new(skill_dir)
                .max_depth(2)  // skill_dir/skill_name/SKILL.md
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == "SKILL.md" {
                    if let Ok(metadata) = self.extract_metadata(entry.path()) {
                        skills.push(metadata);
                    }
                }
            }
        }

        // Cache discovered skills
        *self.discovered_skills.write().unwrap() = skills.clone();

        Ok(skills)
    }

    /// Extract name and description from SKILL.md YAML frontmatter
    fn extract_metadata(&self, skill_md_path: &Path) -> Result<SkillMetadata, SkillError> {
        let content = fs::read_to_string(skill_md_path)?;

        // Parse YAML frontmatter (between --- delimiters)
        let frontmatter = extract_yaml_frontmatter(&content)?;

        let name = frontmatter.get("name")
            .ok_or(SkillError::MissingName)?
            .as_str()
            .ok_or(SkillError::InvalidName)?
            .to_string();

        let description = frontmatter.get("description")
            .ok_or(SkillError::MissingDescription)?
            .as_str()
            .ok_or(SkillError::InvalidDescription)?
            .to_string();

        let directory_path = skill_md_path.parent().unwrap().to_path_buf();

        // Find additional files in skill directory
        let additional_files = fs::read_dir(&directory_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension() == Some("md".as_ref()))
            .filter(|e| e.file_name() != "SKILL.md")
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        Ok(SkillMetadata {
            name,
            description,
            directory_path,
            skill_md_path: skill_md_path.to_path_buf(),
            additional_files,
        })
    }
}

fn extract_yaml_frontmatter(content: &str) -> Result<HashMap<String, serde_yaml::Value>, SkillError> {
    // Extract content between --- delimiters
    if !content.starts_with("---\n") {
        return Err(SkillError::NoFrontmatter);
    }

    let parts: Vec<&str> = content.splitn(3, "---\n").collect();
    if parts.len() < 3 {
        return Err(SkillError::InvalidFrontmatter);
    }

    let yaml_str = parts[1];
    let frontmatter: HashMap<String, serde_yaml::Value> = serde_yaml::from_str(yaml_str)?;

    Ok(frontmatter)
}
```

**Testing Requirements**:
- Test YAML parsing with valid/invalid frontmatter
- Test skill directory scanning
- Test metadata extraction
- Test additional file discovery

**Acceptance Criteria**:
- [x] Discovers all SKILL.md files in configured directories
- [x] Extracts name and description from YAML frontmatter
- [x] Handles missing/invalid frontmatter gracefully
- [x] Caches discovered skills for performance
- [x] Logs discovery process

---

### Task 2.2: Skill Loading Service ✅ COMPLETED
**Priority**: P0 - Blocking
**Estimated Effort**: 8 hours
**Actual Effort**: 2 hours
**File**: `src/app/agent_framework/skills/loader.rs`

**Description**:
Implement on-demand skill loading system.

**Implementation Details**:
```rust
pub struct SkillLoader {
    discovery_service: Arc<SkillDiscoveryService>,
    loaded_skills: RwLock<HashMap<String, LoadedSkill>>,
}

#[derive(Debug, Clone)]
pub struct LoadedSkill {
    pub metadata: SkillMetadata,
    pub content: String,
    pub loaded_at: DateTime<Utc>,
}

impl SkillLoader {
    /// Load full skill content (SKILL.md)
    pub fn load_skill(&self, skill_name: &str) -> Result<LoadedSkill, SkillError> {
        // Check cache first
        {
            let loaded = self.loaded_skills.read().unwrap();
            if let Some(skill) = loaded.get(skill_name) {
                return Ok(skill.clone());
            }
        }

        // Find skill metadata
        let discovered = self.discovery_service.discovered_skills.read().unwrap();
        let metadata = discovered.iter()
            .find(|s| s.name == skill_name)
            .ok_or(SkillError::SkillNotFound(skill_name.to_string()))?
            .clone();

        // Load SKILL.md content
        let content = fs::read_to_string(&metadata.skill_md_path)
            .map_err(|e| SkillError::LoadError(e))?;

        let loaded = LoadedSkill {
            metadata,
            content,
            loaded_at: Utc::now(),
        };

        // Cache
        self.loaded_skills.write().unwrap().insert(skill_name.to_string(), loaded.clone());

        Ok(loaded)
    }

    /// Load additional skill file (forms.md, reference.md, etc.)
    pub fn load_skill_file(&self, skill_name: &str, filename: &str) -> Result<String, SkillError> {
        let metadata = self.get_skill_metadata(skill_name)?;
        let file_path = metadata.directory_path.join(filename);

        fs::read_to_string(&file_path)
            .map_err(|e| SkillError::LoadError(e))
    }
}
```

**Acceptance Criteria**:
- [x] Loads SKILL.md on demand
- [x] Caches loaded skills to avoid re-reading
- [x] Supports loading additional files (forms.md, etc.)
- [x] Returns clear errors for missing skills

---

### Task 2.3: Global Skill Manager ✅ COMPLETED
**Priority**: P0 - Blocking
**Estimated Effort**: 6 hours
**Actual Effort**: 2 hours
**File**: `src/app/agent_framework/skills/manager.rs`

**Description**:
Create global singleton for skill management.

**Implementation Details**:
```rust
// Global skill manager (similar to GLOBAL_AWS_CLIENT pattern)
static GLOBAL_SKILL_MANAGER: RwLock<Option<Arc<SkillManager>>> = RwLock::new(None);

pub struct SkillManager {
    discovery: Arc<SkillDiscoveryService>,
    loader: Arc<SkillLoader>,
}

impl SkillManager {
    pub fn new() -> Self {
        let discovery = Arc::new(SkillDiscoveryService::new());
        let loader = Arc::new(SkillLoader::new(discovery.clone()));

        Self { discovery, loader }
    }

    /// Initialize at application startup
    pub fn initialize() -> Result<(), SkillError> {
        let manager = Arc::new(SkillManager::new());

        // Discover skills
        manager.discovery.discover_skills()?;

        // Set global
        *GLOBAL_SKILL_MANAGER.write().unwrap() = Some(manager);

        Ok(())
    }

    pub fn get_all_skill_metadata(&self) -> Vec<SkillMetadata> {
        self.discovery.discovered_skills.read().unwrap().clone()
    }
}

pub fn get_global_skill_manager() -> Option<Arc<SkillManager>> {
    GLOBAL_SKILL_MANAGER.read().unwrap().clone()
}
```

**Acceptance Criteria**:
- [x] Singleton pattern like other global managers
- [x] Initialized at application startup
- [x] Thread-safe access via RwLock

---

### Task 2.4: Skill Invocation Tool ✅ COMPLETED
**Priority**: P0 - Blocking
**Estimated Effort**: 10 hours
**Actual Effort**: 3 hours
**File**: `src/app/agent_framework/tools/invoke_skill.rs`

**Description**:
Create tool that orchestration agent uses to load skills.

**Implementation Details**:
```rust
// Tool: InvokeSkillTool
// Input: { skill_name: String, load_additional_files: Option<Vec<String>> }
// Output: { content: String, additional_files: HashMap<String, String> }

pub struct InvokeSkillTool;

impl Tool for InvokeSkillTool {
    fn name(&self) -> String {
        "invoke_skill".to_string()
    }

    fn description(&self) -> String {
        "Load a skill to gain specialized knowledge for a task.

        Available skills are listed in your system prompt. When you recognize
        a task matches a skill's description, invoke the skill to load its
        full instructions and capabilities.

        Input:
        - skill_name: Name of skill to load (from available skills list)
        - load_additional_files: Optional array of additional files to load
          (e.g., ['forms.md', 'reference.md'])

        Output:
        - content: Full SKILL.md content with instructions
        - additional_files: Map of filename → content for requested files

        Example:
        invoke_skill('aws-ec2-troubleshooting')
        → Loads EC2 troubleshooting procedures and best practices

        invoke_skill('aws-s3-security', load_additional_files=['checklist.md'])
        → Loads S3 security skill + security checklist".to_string()
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let skill_name = input.get_string("skill_name")?;
        let additional_files = input.get_string_array("load_additional_files")
            .unwrap_or_default();

        let manager = get_global_skill_manager()
            .ok_or(ToolError::SkillSystemNotInitialized)?;

        // Load main skill
        let skill = manager.loader.load_skill(&skill_name)
            .map_err(|e| ToolError::SkillLoadError(e))?;

        // Load additional files
        let mut additional_content = HashMap::new();
        for filename in additional_files {
            match manager.loader.load_skill_file(&skill_name, &filename) {
                Ok(content) => {
                    additional_content.insert(filename, content);
                }
                Err(e) => {
                    // Log but don't fail the entire operation
                    warn!("Failed to load additional file {}: {}", filename, e);
                }
            }
        }

        Ok(ToolOutput {
            content: skill.content,
            additional_files: additional_content,
        })
    }
}
```

**Acceptance Criteria**:
- [x] Loads skills by name
- [x] Supports loading additional files
- [x] Returns clear error messages
- [x] Logs skill invocations

---

### Task 2.5: Skill Metadata Injection ✅ COMPLETED
**Priority**: P0 - Blocking
**Estimated Effort**: 8 hours
**Actual Effort**: 2 hours
**File**: `src/app/agent_framework/agents/orchestration_agent.rs`

**Description**:
Inject skill metadata into orchestration agent system prompt at startup.

**Implementation Details**:
```rust
// Modify OrchestrationAgent::create_system_prompt()

pub fn create_system_prompt() -> String {
    let base_prompt = "You are the AWS Orchestration Agent...";

    // Get skill metadata from global manager
    let skill_metadata = match get_global_skill_manager() {
        Some(manager) => manager.get_all_skill_metadata(),
        None => Vec::new(),
    };

    // Build skills section
    let skills_section = if skill_metadata.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nAVAILABLE SKILLS:\nYou have access to specialized skills. When a task matches a skill, use invoke_skill to load it.\n\n{}",
            skill_metadata.iter()
                .map(|s| format!("- {}: {}", s.name, s.description))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    format!("{}{}", base_prompt, skills_section)
}
```

**Anthropic Pattern**:
```
Example system prompt injection:
"You are an AWS agent...

AVAILABLE SKILLS:
- aws-ec2-troubleshooting: Diagnose and resolve EC2 instance issues including networking, storage, and performance
- aws-lambda-optimization: Analyze and optimize Lambda function performance, cold starts, and costs
- aws-s3-security: Audit S3 bucket configurations for security best practices and compliance
- aws-cloudwatch-analysis: Investigate CloudWatch logs and metrics to identify patterns and anomalies
- aws-iam-policy-review: Review IAM policies for least privilege and security vulnerabilities

When you recognize a task matching a skill, use invoke_skill('skill-name') to load specialized instructions."
```

**Acceptance Criteria**:
- [x] Skills section added to system prompt
- [x] All discovered skills listed with name and description
- [x] Skills section only included if skills discovered
- [x] Updated on each agent creation (dynamic discovery)

---

### Task 2.6: Skill Refresh Mechanism ✅ COMPLETED
**Priority**: P1 - Important
**Estimated Effort**: 4 hours
**Actual Effort**: Built into manager (0.5 hours)
**File**: `src/app/agent_framework/skills/manager.rs`

**Description**:
Allow refreshing skill discovery without restart.

**Implementation Details**:
```rust
impl SkillManager {
    /// Refresh skill discovery (called by UI or periodically)
    pub fn refresh(&self) -> Result<usize, SkillError> {
        let discovered = self.discovery.discover_skills()?;
        let count = discovered.len();

        // Clear loaded skill cache
        self.loader.loaded_skills.write().unwrap().clear();

        info!("Refreshed {} skills", count);
        Ok(count)
    }
}

// Add refresh tool (optional)
pub struct RefreshSkillsTool;

impl Tool for RefreshSkillsTool {
    fn name(&self) -> String {
        "refresh_skills".to_string()
    }

    fn description(&self) -> String {
        "Refresh the list of available skills.

        Call this after installing new skills or modifying existing ones.

        Output: Number of skills discovered".to_string()
    }

    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput, ToolError> {
        let manager = get_global_skill_manager()
            .ok_or(ToolError::SkillSystemNotInitialized)?;

        let count = manager.refresh()?;

        Ok(ToolOutput {
            skills_discovered: count,
            message: format!("Refreshed {} skills", count),
        })
    }
}
```

**Acceptance Criteria**:
- [x] Refresh rediscovers skills without restart
- [x] Clears loaded skill cache
- [x] Returns count of discovered skills

---

## Milestone 3: Prompt Enhancement

### Task 3.1: Update System Prompt ✅ COMPLETED
**Priority**: P0 - Blocking
**Estimated Effort**: 8 hours
**Actual Effort**: 2 hours
**File**: `src/app/agent_framework/agents/orchestration_agent.rs`

**Description**:
Update orchestration agent prompt per PROMPT_ANALYSIS_REPORT.md recommendations and add skill system.

**Implementation Details**:

Based on Anthropic best practices and current analysis, restructure prompt from 184 lines to ~130 lines with embedded LLM-managed intelligence philosophy:

```rust
pub fn create_system_prompt() -> String {
    format!(
r#"# AWS Orchestration Agent

You are an AWS Orchestration Agent that delegates infrastructure tasks to specialized task agents. You have autonomy to recognize patterns, select appropriate skills, and adapt workflows to user context.

## Extended Thinking: Plan Before Acting

Before taking action, analyze:
1. **Task Type**: Simple lookup? Multi-step analysis? Complex investigation?
2. **Skill Match**: Does this match an available skill's description?
   - Use semantic understanding, not keyword matching
   - Skills are guidance, not rigid scripts - adapt to user context
3. **AWS Context**: Account ID, region, resource identifiers needed?
4. **Delegation Strategy**: Use tools directly or create task agent?
5. **Parallelization**: Can multiple task agents run simultaneously?

## Available Skills

{}

**How to Use Skills**:
- Recognize semantic match between user intent and skill description
- invoke_skill('skill-name') loads full procedures and best practices
- Adapt skill guidance to user's specific context
- Combine multiple skills if task requires multiple domains
- Don't force-fit skills; if no match, proceed without

**Examples of Skill Recognition**:
- "EC2 won't start" → matches aws-ec2-troubleshooting (semantic: availability issue)
- "Lambda is slow" → matches aws-lambda-optimization (semantic: performance issue)
- "Check S3 buckets" → NO skill match (simple list, not security audit)

## Available Tools

**Skill System**:
- `invoke_skill(skill_name)`: Load specialized knowledge on-demand
- `read_file(file_path)`: Read skill files and additional resources
- `list_directory(path)`: Discover available skills

**Agent Coordination**:
- `create_task(description, account, region)`: Spawn specialized task agents
- `TodoWrite(todos)`: Track multi-step task progress (USE PROACTIVELY)
- `TodoRead()`: Query current task state

**AWS Context** (no API calls):
- `aws_find_account(filter)`: Search AWS accounts by name/ID
- `aws_find_region(filter)`: Search AWS regions by name

## Workflow Pattern

**Standard Flow**:
1. **Think**: Check skill match, assess complexity
2. **Skill**: If match, `invoke_skill` to load expertise
3. **Plan**: Use `TodoWrite` for multi-step tasks
4. **Context**: Gather account/region with `aws_find_account` / `aws_find_region`
5. **Delegate**: `create_task` with clear objective and context
6. **Monitor**: Update todos, report results

**Example Flow (With Skill)**:
```
User: "My EC2 instance isn't responding"
Think: "not responding" = availability → matches "aws-ec2-troubleshooting"
invoke_skill('aws-ec2-troubleshooting')
[Skill loads: Check status → network → storage]
Adapt: User didn't specify region, need to ask
aws_find_account("production")
Response: "Loaded EC2 troubleshooting skill. Which region is the instance in?"
[User provides region]
create_task("Diagnose EC2 instance availability using troubleshooting procedures", account, region)
```

**Example Flow (No Skill Needed)**:
```
User: "List Lambda functions"
Think: Simple list operation, no skill needed
aws_find_account(), aws_find_region()
create_task("List all Lambda functions", account, region)
```

## Effort Scaling Guidelines

Help the agent understand expected scale:

**Simple Tasks** (1 agent, 3-5 tool calls, <1 min):
- "List EC2 instances in us-east-1"
- "Find Lambda function named 'api-handler'"
- "Get S3 bucket policy for 'my-bucket'"

**Analysis Tasks** (2-3 agents, 10-15 calls each, 2-5 min):
- "Find Lambda errors and identify patterns"
- "Analyze CloudWatch alarms for critical services"
- "Review security group configurations"

**Complex Investigations** (5+ agents, 20+ calls each, 5-15 min):
- "Audit S3 security across all accounts"
- "Investigate EC2 performance degradation"
- "Comprehensive IAM policy review"

## Task Delegation Specifications

Each `create_task` must provide:
- **Objective**: Clear, specific goal
- **Output Format**: Expected structure/format
- **Tool Priorities**: Which tools to use first
- **Boundaries**: Scope limits, what NOT to include

**Good Delegation Example**:
```
create_task(
  "Find Lambda functions with error rate >5% in last 24h.
   Output: Table with function name, error rate, top 3 error messages.
   Tools: aws_list_resources(Lambda), aws_describe_log_groups, aws_get_log_events.
   Boundaries: Only production account, ignore test functions.",
  account_id, region
)
```

## Output Format

**Tool Results Priority**:
1. Present tool results immediately (no line limit)
2. Add 1-2 sentence summary only if helpful
3. No preamble ("Based on...") or postamble ("Let me know...")

**Examples**:
- After `invoke_skill`: "Loaded EC2 troubleshooting skill with 15 diagnostic procedures."
- After `aws_find_account`: "Found 3 accounts: Production (123456789012), Staging (234567890123), Dev (345678901234)"
- After `create_task`: "Started task agent to analyze Lambda errors. Results will appear shortly."

**Avoid**:
- ❌ "Based on the information provided..."
- ❌ "Let me know if you need anything else..."
- ❌ "I'll help you with that. First, let me..."

## Error Handling

**Empty Results**:
- Don't just say "no results found"
- Ask for clarification with specific suggestions
- Example: "No EC2 instances found. Did you mean a different region? Available: us-east-1, us-west-2, eu-west-1"

**Tool Failures**:
- Explain what failed and why
- Suggest alternatives
- Example: "Skill 'aws-rds-optimization' not found. Available skills: aws-ec2-troubleshooting, aws-lambda-optimization, aws-s3-security"

**Missing Context**:
- Ask specifically with options
- Example: "Which account? Found: Production (123), Staging (456), Dev (789)"

**Skill Not Found**:
- List available skills with descriptions
- Suggest proceeding without skill if applicable

## Security and Safety

- **Destructive Operations**: Refuse without explicit confirmation
- **Credentials**: Never expose, log, or request AWS credentials
- **Security Practices**: Only defensive security (auditing, analysis, compliance)
- **AWS Best Practices**: Follow AWS Well-Architected Framework principles

## TodoWrite Requirements

Use `TodoWrite` PROACTIVELY for:
- Multi-step tasks (3+ steps)
- Complex investigations
- Tasks requiring multiple task agents
- User explicitly provides list of tasks

Mark todos complete IMMEDIATELY after finishing, don't batch.

{}
"#,
        generate_skills_list(),
        generate_examples()
    )
}

fn generate_skills_list() -> String {
    let manager = match get_global_skill_manager() {
        Some(m) => m,
        None => return "No skills available (skill system not initialized)".to_string(),
    };

    let metadata = manager.get_all_skill_metadata();
    if metadata.is_empty() {
        return "No skills installed. Install skills to ~/.claude/skills/ or ~/.awsdash/skills/".to_string();
    }

    metadata.iter()
        .map(|s| format!("- **{}**: {}", s.name, s.description))
        .collect::<Vec<_>>()
        .join("\n")
}
```

**Acceptance Criteria**:
- [x] Reduced from 245 to 132 lines (46% reduction)
- [x] No contradictory instructions
- [x] Includes skill system integration
- [x] Extended thinking prompts included ("Plan Before Acting")
- [x] Workflow patterns with concrete examples
- [x] Error handling section included
- [x] All tests passing

---

### Task 3.2: Add Tool Descriptions
**Priority**: P0 - Blocking
**Estimated Effort**: 4 hours
**File**: `src/app/agent_framework/agents/orchestration_agent.rs`

**Description**:
Enhance tool descriptions in prompt with Anthropic-style comprehensive documentation.

**Implementation per Task 1.1-1.4 tool descriptions**.

**Acceptance Criteria**:
- [ ] Each tool has: purpose, inputs, outputs, edge cases, examples
- [ ] Tool boundaries clearly defined
- [ ] Security constraints specified

---

### Task 3.3: Create Skill Usage Examples
**Priority**: P1 - Important
**Estimated Effort**: 6 hours
**File**: `src/app/agent_framework/agents/orchestration_agent.rs`

**Description**:
Add concrete examples of skill invocation to prompt.

**Implementation Details**:
```rust
fn generate_examples() -> String {
    r#"
## Example Interactions

### Example 1: Using a Skill for Specialized Task
User: "My EC2 instance isn't responding"
Agent thinks: This matches aws-ec2-troubleshooting skill
Agent: [invokes invoke_skill('aws-ec2-troubleshooting')]
Agent: [reads loaded skill with diagnostic procedures]
Agent: "Loaded EC2 troubleshooting skill. Following the diagnostic checklist:
       1. First, I need your account and region
       2. Then I'll check instance status, security groups, and network ACLs
       What account and region is the instance in?"

### Example 2: Skill with Additional Resources
User: "Review IAM policies for security issues"
Agent thinks: This matches aws-iam-policy-review skill, might need checklist
Agent: [invokes invoke_skill('aws-iam-policy-review', load_additional_files=['checklist.md'])]
Agent: "Loaded IAM policy review skill with security checklist.
       I'll need the AWS account to audit. Which account?"

### Example 3: No Matching Skill, Create Task Agent
User: "Find CloudTrail events for user john@example.com"
Agent thinks: No specific skill, but straightforward task
Agent: [calls aws_find_account, aws_find_region]
Agent: [calls create_task with CloudTrail search description]
Agent: "Started task agent to search CloudTrail events. Searching Production account..."
"#.to_string()
}
```

**Acceptance Criteria**:
- [ ] 3-5 concrete examples showing skill usage
- [ ] Examples demonstrate thinking process
- [ ] Shows when to use skills vs. direct tools
- [ ] Examples are AWS-domain specific

---

## Milestone 4: Example Skills

### Task 4.1: AWS EC2 Troubleshooting Skill
**Priority**: P0 - Validation
**Estimated Effort**: 6 hours
**File**: `~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md`

**Description**:
Create comprehensive EC2 troubleshooting skill with diagnostic procedures.

**Implementation Details**:
```markdown
---
name: aws-ec2-troubleshooting
description: Diagnose and resolve EC2 instance issues including networking, storage, performance, and connectivity
---

# AWS EC2 Troubleshooting Skill

## Purpose
This skill provides systematic diagnostic procedures for EC2 instance issues.

## When to Use This Skill
- Instance not responding to SSH/RDP
- Instance showing unexpected behavior
- Performance degradation
- Networking connectivity issues
- Storage problems

## Diagnostic Workflow

### 1. Gather Instance Information
Before investigating, collect:
- Account ID
- Region
- Instance ID
- Instance type
- VPC ID
- Subnet ID

Tools to use:
- aws_find_account: Get account ID
- aws_find_region: Get region
- aws_describe_resource: Get instance details

### 2. Check Instance Status
Create task agent to check:
- Instance state (running, stopped, terminated)
- Status checks (system, instance)
- Console output for boot errors
- System logs for kernel issues

### 3. Network Diagnostics
Create task agent to verify:
- Security group rules (inbound/outbound)
- Network ACLs
- Route table entries
- Elastic IP association
- DNS resolution

### 4. Storage Diagnostics
If storage issues suspected:
- EBS volume status
- Volume attachments
- Disk space utilization (via CloudWatch)
- I/O performance metrics

### 5. Performance Analysis
For performance issues:
- CPU utilization (CloudWatch)
- Memory pressure (CloudWatch Agent)
- Network throughput
- EBS I/O metrics

## Task Agent Delegation

**For each diagnostic area**, create separate task agent:
```
create_task(
  task_description="Check EC2 instance {instance-id} status checks and console output",
  account_id="{account}",
  region="{region}"
)
```

**Parallel execution**: Run network and storage diagnostics simultaneously.

## Expected Outputs

Each diagnostic should return:
- Status: OK / WARNING / CRITICAL
- Findings: List of issues discovered
- Recommendations: Specific remediation steps

## Common Issues and Solutions

### Issue: Cannot connect via SSH
Checks:
1. Instance state = running
2. Security group allows port 22 from your IP
3. Network ACL allows inbound/outbound
4. Route table has internet gateway route
5. SSH key is correct

### Issue: Instance status check failed
Checks:
1. System status: AWS infrastructure issue → contact AWS support
2. Instance status: Software/config issue → check console output

### Issue: High CPU utilization
Checks:
1. Identify process via CloudWatch logs
2. Check if instance type appropriate for workload
3. Review CloudWatch alarms

## Additional Resources
For specific scenarios, see:
- networking.md: Network troubleshooting details
- storage.md: EBS and storage diagnostics
- performance.md: Performance tuning guide
```

**Create additional files**:
- `networking.md`: Deep dive on security groups, NACLs, routing
- `storage.md`: EBS troubleshooting procedures
- `performance.md`: Performance optimization techniques

**Acceptance Criteria**:
- [ ] SKILL.md has valid YAML frontmatter
- [ ] Contains systematic diagnostic workflow
- [ ] Specifies when to delegate to task agents
- [ ] Includes common issue patterns
- [ ] References additional files appropriately

---

### Task 4.2: AWS Lambda Optimization Skill
**Priority**: P1 - Validation
**Estimated Effort**: 5 hours
**File**: `~/.awsdash/skills/aws-lambda-optimization/SKILL.md`

**Description**:
Create Lambda performance optimization skill.

**Structure**:
```markdown
---
name: aws-lambda-optimization
description: Analyze and optimize Lambda function performance, cold starts, memory allocation, and costs
---

# AWS Lambda Optimization Skill

## Optimization Categories
1. Cold Start Reduction
2. Memory Allocation Tuning
3. Concurrency Configuration
4. Cost Optimization
5. Error Rate Reduction

## Analysis Workflow
[Similar structure to EC2 skill]

## Task Agent Delegation Patterns
[Specific patterns for Lambda analysis]
```

**Acceptance Criteria**:
- [ ] Covers cold starts, memory, concurrency, cost
- [ ] Provides specific metrics to analyze
- [ ] Includes cost calculation examples

---

### Task 4.3: AWS S3 Security Audit Skill
**Priority**: P1 - Validation
**Estimated Effort**: 5 hours
**File**: `~/.awsdash/skills/aws-s3-security/SKILL.md`

**Description**:
Create S3 security audit skill with compliance checklist.

**Structure**:
```markdown
---
name: aws-s3-security
description: Audit S3 bucket configurations for security best practices, encryption, access controls, and compliance
---

# AWS S3 Security Audit Skill

## Security Categories
1. Bucket Access Controls
2. Encryption Configuration
3. Versioning and Lifecycle
4. Logging and Monitoring
5. Public Access Settings

## Additional Files
- checklist.md: Complete security checklist
- compliance.md: Compliance requirements (HIPAA, PCI, SOC2)
```

**Acceptance Criteria**:
- [ ] Security audit procedures defined
- [ ] Checklist in separate file
- [ ] Compliance mappings included

---

### Task 4.4: AWS CloudWatch Analysis Skill
**Priority**: P2 - Nice to Have
**Estimated Effort**: 4 hours
**File**: `~/.awsdash/skills/aws-cloudwatch-analysis/SKILL.md`

**Description**:
Log and metric analysis patterns.

---

### Task 4.5: AWS IAM Policy Review Skill
**Priority**: P2 - Nice to Have
**Estimated Effort**: 4 hours
**File**: `~/.awsdash/skills/aws-iam-policy-review/SKILL.md`

**Description**:
IAM policy security review and least privilege checking.

---

### Task 4.6: Skill Installation Documentation
**Priority**: P1 - Important
**Estimated Effort**: 4 hours
**File**: `docs/technical/agent-skills-system.md`

**Description**:
Document skill installation and creation.

**Topics**:
- Where to install skills (`~/.claude/skills`, `~/.awsdash/skills`)
- SKILL.md file format and requirements
- YAML frontmatter structure
- Creating custom skills
- Skill discovery and refresh
- Troubleshooting skills not appearing

**Acceptance Criteria**:
- [ ] Complete user guide for skill installation
- [ ] Examples of creating new skills
- [ ] Troubleshooting section

---

## Milestone 5: Integration & Testing

### Task 5.1: End-to-End Skill Test
**Priority**: P0 - Validation
**Estimated Effort**: 8 hours
**File**: `tests/agent_framework/skill_integration_test.rs`

**Description**:
Integration test exercising full skill workflow.

**Test Scenarios**:
1. Agent discovers skills on startup
2. User asks EC2-related question
3. Agent recognizes skill match
4. Agent invokes skill
5. Agent follows skill procedures
6. Agent creates appropriate task agents
7. Agent reports results

**Acceptance Criteria**:
- [ ] Test passes end-to-end
- [ ] Validates skill discovery
- [ ] Validates skill loading
- [ ] Validates agent follows skill guidance

---

### Task 5.2: Multi-Skill Scenario Test
**Priority**: P1 - Validation
**Estimated Effort**: 6 hours

**Description**:
Test agent using multiple skills in single session.

**Test Scenario**:
User: "Check my EC2 instance performance and review S3 bucket security"
Expected: Agent loads both skills and delegates appropriately

---

### Task 5.3: Skill Error Handling Test
**Priority**: P1 - Validation
**Estimated Effort**: 4 hours

**Description**:
Test error scenarios:
- Skill not found
- Invalid SKILL.md format
- Missing frontmatter
- File read failures

---

### Task 5.4: Performance Benchmarking
**Priority**: P1 - Validation
**Estimated Effort**: 4 hours

**Description**:
Measure context token reduction with skills.

**Metrics**:
- Baseline: Current prompt tokens
- With skills: Prompt + skill metadata only
- Skill loaded: Prompt + metadata + SKILL.md
- Expected: 60% reduction in baseline, 20% increase when loaded

---

## Dependencies and Risks

### External Dependencies
- **serde_yaml**: YAML frontmatter parsing
- **walkdir**: Directory traversal for skill discovery
- **dirs**: Platform-specific skill directory paths

### Risks and Mitigations

**Risk 1**: Skills bloat context window
- **Mitigation**: Progressive disclosure - only load when needed
- **Mitigation**: Size limits on SKILL.md (10MB max)
- **Mitigation**: Agent learns when NOT to load skills

**Risk 2**: Security - malicious skills
- **Mitigation**: Path validation prevents directory traversal
- **Mitigation**: Skills only loaded from trusted directories
- **Mitigation**: No code execution in MVP (future feature)
- **Mitigation**: Skill audit/review process

**Risk 3**: Skill discovery performance
- **Mitigation**: Cache discovered skills
- **Mitigation**: Lazy refresh (manual or periodic)
- **Mitigation**: Limit directory depth (max 2 levels)

**Risk 4**: Agent doesn't use skills
- **Mitigation**: Clear skill descriptions in metadata
- **Mitigation**: Examples in system prompt
- **Mitigation**: Reinforcement in task planning section

---

## Future Enhancements (Post-MVP)

### Phase 2: Code Execution
- Add code execution tool (Python, Bash)
- Skills can include scripts for deterministic operations
- Example: S3 security audit script that generates report

### Phase 3: Skill Marketplace
- Community skill repository
- Skill versioning and updates
- Skill ratings and reviews

### Phase 4: Agent-Created Skills
- Agent observes successful task patterns
- Agent suggests creating skill from pattern
- Agent generates SKILL.md from experience

### Phase 5: MCP Integration
- Implement Model Context Protocol server
- Skills exposed as MCP resources
- Interoperability with other MCP clients

---

## Success Criteria

### MVP Complete When:
- [ ] All P0 tasks completed
- [ ] 3+ example skills created and tested
- [ ] Orchestration agent successfully invokes skills
- [ ] End-to-end integration test passes
- [ ] Documentation complete
- [ ] Context token usage reduced by 50%+

### Quality Gates:
- [ ] Security review passed (path validation, file access)
- [ ] Performance benchmarks met (60% token reduction)
- [ ] User documentation complete
- [ ] All tests passing
- [ ] Code review approved

---

## Implementation Order

**Week 1-2: Foundation**
1. Task 1.3: File security validation (shared dependency)
2. Task 1.1: File read tool
3. Task 1.2: Directory list tool
4. Task 1.4: Tool registration

**Week 3: Core System**
5. Task 2.1: Skill discovery service
6. Task 2.2: Skill loading service
7. Task 2.3: Global skill manager

**Week 4: Integration**
8. Task 2.4: Skill invocation tool
9. Task 2.5: Skill metadata injection
10. Task 2.6: Skill refresh mechanism

**Week 5: Prompt**
11. Task 3.1: Update system prompt
12. Task 3.2: Tool descriptions
13. Task 3.3: Skill usage examples

**Week 6-7: Skills**
14. Task 4.1: EC2 troubleshooting skill
15. Task 4.2: Lambda optimization skill
16. Task 4.3: S3 security skill
17. Task 4.6: Installation documentation

**Week 8: Testing**
18. Task 5.1: End-to-end test
19. Task 5.2: Multi-skill test
20. Task 5.3: Error handling test
21. Task 5.4: Performance benchmarking

---

## Document Summary

**This is the SINGLE SOURCE OF TRUTH for Agent Skills implementation.**

This document contains:
1. **Architecture Philosophy** (Lines 14-140): LLM-managed intelligence vs rules-based safety
2. **5 Milestones with Detailed Tasks** (Lines 158-1550): Complete implementation roadmap
3. **Enhanced Orchestration Agent Prompt** (Lines 937-1117): Complete rewrite with skill integration
4. **Risk Mitigation Strategies** (Lines 1540-1563): Security, performance, adoption
5. **Implementation Order** (Lines 1610-1649): Week-by-week execution plan

**Key Architectural Decisions**:
- **90% LLM-Managed**: Skill selection, application, orchestration (semantic understanding)
- **10% Rules-Based**: Security boundaries, file format, discovery mechanics (hard constraints)
- **Progressive Disclosure**: Metadata at startup (cheap), full content on-demand (expensive but targeted)
- **Token Economics**: 60% reduction in baseline context, 6,000 tokens when specialist skills loaded

**File Locations**:
- Implementation: `src/app/agent_framework/` (Rust source code)
- Skills: `~/.claude/skills/` and `~/.awsdash/skills/` (SKILL.md files)
- Tests: `tests/agent_framework/` (integration tests)
- Documentation: `docs/technical/agent-skills-system.md` (user guide)

**Current Status**: Planning phase (Week 0)
**Next Step**: Begin Milestone 1, Task 1.3 (file security validation - shared dependency)

---

**End of TODO Document**
**Total Estimated Effort**: 180 hours (4-5 weeks full-time)
**Priority**: High - Aligns with Anthropic best practices
**Expected ROI**: 60% context reduction + 40% task performance improvement
