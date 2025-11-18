# Agent Initialization & Skill System Flow

This document provides a high-level pseudocode view of how agents are initialized and how the skill system integrates into the agent workflow.

---

## 1. Application Startup (One-Time Initialization)

```pseudocode
main():
    DashApp.new(creation_context):
        // PHASE 1: Skill System Initialization (BEFORE agents)
        initialize_skills():
            skill_manager = GlobalSkillManager.new()
            discovered_count = skill_manager.discover_skills():
                for directory in [~/.awsdash/skills/, ~/.claude/skills/]:
                    for file in walk_directory(directory, max_depth=2):
                        if file.name == "SKILL.md":
                            metadata = extract_yaml_frontmatter(file):
                                parse_yaml("""
                                    ---
                                    name: aws-ec2-troubleshooting
                                    description: Diagnose and resolve EC2 instance issues...
                                    ---
                                """)
                            skill_metadata_cache.add(metadata)

            set_global_skill_manager(skill_manager)
            log("✅ Skill system initialized: {} skills discovered", discovered_count)

        // PHASE 2: AWS Identity & Agent Framework (AFTER skills)
        // (Happens when user logs into AWS Identity Center)

    return app


// Later: User logs into AWS Identity Center
on_aws_login(aws_identity):
    // PHASE 3: Agent Manager Initialization
    agent_manager = AgentManager.new(aws_identity)
    set_global_aws_identity(aws_identity)
    log("🚀 AgentManager initialized")
```

**Key Point**: Skills are discovered at startup, **before** any agents are created. This ensures skill metadata is available immediately when agents are initialized.

---

## 2. Agent Creation (Per-Agent Initialization)

```pseudocode
user_clicks_launch_agent(name, description, model_id):
    agent_manager.launch_agent(name, description, model_id):
        // Step 1: Create agent instance
        agent_id = generate_uuid()
        agent_instance = AgentInstance.new(agent_id, metadata):
            // Initialize per-agent logger
            logger = AgentLogger.new(agent_id, name)
            logger.log_agent_created(metadata)

            // Agent starts with empty state
            agent = None  // Will be created on first message
            messages = []
            status = Running

        agent_registry.insert(agent_id, agent_instance)
        return agent_id
```

**Key Point**: Agent instances are lightweight shells. The actual LLM-based agent is created lazily on first message.

---

## 3. User Input Processing (Where Skills Come In)

```pseudocode
user_sends_message(agent_id, "My EC2 instance isn't responding to SSH"):
    agent_instance = agent_registry.get(agent_id)

    // Add user message to conversation history
    agent_instance.messages.push({
        role: User,
        content: "My EC2 instance isn't responding to SSH"
    })

    // SPAWN BACKGROUND THREAD for agent execution
    spawn_thread():
        // Step 1: Get AWS credentials
        aws_creds = get_aws_credentials_from_identity_center()

        // Step 2: Create agent on first use (LAZY INITIALIZATION)
        if agent_instance.agent is None:
            orchestration_agent = OrchestrationAgent.create():

                // STEP 2A: GENERATE SYSTEM PROMPT WITH SKILLS
                system_prompt = create_system_prompt():
                    base_prompt = """
                    # AWS Orchestration Agent

                    ## Extended Thinking: Plan Before Acting
                    1. Task Type: Simple lookup? Multi-step analysis? Complex investigation?
                    2. **Skill Match**: Does this match an available skill's description?
                       - Use semantic understanding, not keyword matching
                    3. AWS Context: Account ID, region, resource identifiers needed?
                    4. Delegation Strategy: Use tools directly or create task agent?

                    ## Available Tools
                    - invoke_skill(skill_name): Load specialized AWS domain knowledge
                    - create_task(description, account, region): Spawn task agents
                    - aws_find_account(filter), aws_find_region(filter)
                    - TodoWrite, TodoRead, read_file, list_directory

                    ## Workflow Pattern
                    1. Think: Check skill match, assess complexity
                    2. Skill: If semantic match, invoke_skill to load expertise
                    3. Plan: Use TodoWrite for multi-step tasks
                    4. Context: Gather account/region
                    5. Delegate: create_task with clear objective
                    """

                    // INJECT AVAILABLE SKILLS DYNAMICALLY
                    skill_manager = get_global_skill_manager()
                    skill_metadata = skill_manager.get_all_skill_metadata()

                    if skill_metadata is not empty:
                        skills_section = "\n\nAVAILABLE SKILLS:\n"
                        for skill in skill_metadata:
                            skills_section += "- {}: {}\n".format(skill.name, skill.description)

                        skills_section += """
                        Progressive Disclosure: Skills listed by name/description only.
                        Use invoke_skill to load full content when needed.

                        LLM-Managed Intelligence: Use semantic understanding to match
                        user intent to skills. Don't require exact keyword matches.
                        """

                        system_prompt += skills_section

                    return system_prompt

                // STEP 2B: REGISTER TOOLS
                tools = [
                    invoke_skill_tool,      // Load skills on-demand
                    create_task_tool,       // Spawn task agents
                    aws_find_account_tool,  // Search accounts
                    aws_find_region_tool,   // Search regions
                    todo_write_tool,        // Task tracking
                    todo_read_tool,         // Task queries
                    read_file_tool,         // Read skill files
                    list_directory_tool     // Discover skills
                ]

                // STEP 2C: CREATE LLM AGENT
                agent = Agent.new(
                    model_id: "anthropic.claude-sonnet-4-5",
                    system_prompt: system_prompt,
                    tools: tools,
                    callback_handler: AgentToolCallbackHandler
                )

                return agent

            agent_instance.agent = orchestration_agent
            logger.log_agent_initialized(model_id)

        // Step 3: EXECUTE AGENT WITH USER INPUT
        agent = agent_instance.agent
        user_input = "My EC2 instance isn't responding to SSH"

        logger.log_model_request(system_prompt, user_input, model_id)

        // LLM processes input using system prompt (which includes skill metadata)
        response = agent.execute(user_input):

            // AGENT THINKING (Claude Sonnet Extended Thinking):
            // "User says 'isn't responding' - this is an availability/connectivity issue
            //  Looking at AVAILABLE SKILLS section:
            //  - aws-ec2-troubleshooting: Diagnose and resolve EC2 instance issues including networking...
            //
            //  MATCH FOUND: This is clearly an EC2 troubleshooting scenario
            //  Should invoke skill to get detailed diagnostic procedures"

            // AGENT DECISION: Use invoke_skill tool
            tool_call = {
                tool: "invoke_skill",
                parameters: {
                    skill_name: "aws-ec2-troubleshooting"
                }
            }

            // TOOL EXECUTION (Callback Handler)
            tool_result = invoke_skill_tool.execute(parameters):
                skill_manager = get_global_skill_manager()
                loaded_skill = skill_manager.load_skill("aws-ec2-troubleshooting"):

                    // Check cache first
                    if skill_cache.contains("aws-ec2-troubleshooting"):
                        return skill_cache.get("aws-ec2-troubleshooting")

                    // Load from disk
                    skill_path = ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md
                    skill_content = read_file(skill_path)  // 8835 bytes

                    // Cache for future use
                    skill_cache.insert("aws-ec2-troubleshooting", {
                        metadata: metadata,
                        content: skill_content,
                        loaded_at: now()
                    })

                    return {
                        skill_name: "aws-ec2-troubleshooting",
                        content: skill_content,
                        size_bytes: 8835
                    }

                logger.log_tool_execution("invoke_skill", parameters, result)

                return result

            // AGENT PROCESSES SKILL CONTENT:
            // Now has full diagnostic procedures:
            // - Check instance status (running, stopped, terminated)
            // - Network diagnostics (security groups, NACLs, routes)
            // - Storage diagnostics (EBS volumes)
            // - Performance analysis (CloudWatch metrics)

            // AGENT NEXT STEPS (Following Skill Guidance):
            // Skill says: "Gather Instance Information"
            // - Need: Account ID, Region, Instance ID
            // - Use aws_find_account if not specified

            tool_call = {
                tool: "aws_find_account",
                parameters: {
                    filter: "production"
                }
            }

            // ... agent continues following skill procedures

            final_response = """
            Loaded EC2 troubleshooting skill with diagnostic procedures for
            networking, storage, performance, and connectivity issues.

            To diagnose why your instance isn't responding to SSH, I need:
            1. Which AWS account is the instance in?
            2. Which region?
            3. What's the instance ID?
            """

            return final_response

        // Step 4: Send response back to UI
        send_response_to_ui(response)
        logger.log_model_response(response, token_usage)
```

---

## 4. Skill System Integration Points

```pseudocode
// KEY INTEGRATION POINTS WHERE SKILLS ARE USED:

// Point 1: STARTUP - Skill Discovery
application_starts():
    initialize_skill_system()  // Discovers all skills, builds metadata cache

// Point 2: AGENT CREATION - System Prompt Injection
create_agent():
    system_prompt = create_system_prompt():
        skill_metadata = get_all_skill_metadata()  // From discovery cache
        inject_skills_into_prompt(skill_metadata)

// Point 3: USER INPUT - Skill Matching (LLM Decision)
agent_processes_input(user_message):
    // LLM reads system prompt, sees AVAILABLE SKILLS section
    // LLM uses semantic understanding to match user intent to skill
    // LLM decides whether to invoke_skill based on:
    //   - Does user request match skill description?
    //   - Is this a troubleshooting/diagnostic/optimization scenario?
    //   - Will skill provide valuable guidance?

// Point 4: TOOL EXECUTION - Skill Loading
invoke_skill_tool.execute("aws-ec2-troubleshooting"):
    skill_manager.load_skill():
        check_cache()  // Returns immediately if cached
        read_from_disk()  // Only on first use
        cache_for_reuse()

// Point 5: SKILL GUIDANCE - Agent Follows Procedures
agent_has_skill_content():
    // Agent now has full skill content (e.g., 8835 bytes of EC2 diagnostics)
    // Agent follows procedures adaptively:
    //   - Gathers required context (account, region, instance ID)
    //   - Creates task agents for diagnostic checks
    //   - Presents findings to user
    //   - Provides remediation recommendations
```

---

## 5. Token Economics: Progressive Disclosure

```pseudocode
// WITHOUT SKILLS (all expertise in system prompt):
system_prompt_size = 50,000 tokens  // EC2 + Lambda + S3 + RDS + ... all domains
per_request_cost = 50,000 tokens * $0.003 = $0.15
total_requests = 1000
total_cost = $150

// WITH SKILLS (metadata only, load on-demand):
base_prompt_size = 5,000 tokens
skill_metadata_size = 500 tokens  // Just names & descriptions
per_request_cost_base = 5,500 tokens * $0.003 = $0.0165

// When skill needed (10% of requests):
skill_invocations = 100
skill_content_size = 3,000 tokens
per_request_cost_with_skill = (5,500 + 3,000) * $0.003 = $0.0255
skill_invocation_cost = 100 * $0.0255 = $2.55

// When skill NOT needed (90% of requests):
simple_requests = 900
simple_request_cost = 900 * $0.0165 = $14.85

total_cost = $2.55 + $14.85 = $17.40

// SAVINGS: $150 - $17.40 = $132.60 (88% reduction)
```

---

## 6. Summary: Skill System Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ APPLICATION STARTUP                                             │
├─────────────────────────────────────────────────────────────────┤
│ 1. initialize_skill_system()                                    │
│    - Scan ~/.awsdash/skills/                                    │
│    - Parse YAML frontmatter (name, description)                 │
│    - Build metadata cache (cheap: ~100 bytes per skill)         │
│    - Log: "✅ Skill system initialized: 1 skills discovered"   │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ AGENT CREATION (First User Message)                            │
├─────────────────────────────────────────────────────────────────┤
│ 2. create_system_prompt()                                       │
│    - Build base prompt (5000 tokens)                            │
│    - Inject skill metadata from cache (500 tokens)              │
│    - Result: Prompt with AVAILABLE SKILLS section               │
│                                                                 │
│ 3. register_tools()                                             │
│    - invoke_skill, create_task, aws_find_*, TodoWrite, etc.     │
│                                                                 │
│ 4. create_agent(system_prompt, tools)                           │
│    - Claude Sonnet 4.5 with skill-aware prompt                  │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ USER INPUT: "My EC2 instance isn't responding"                 │
├─────────────────────────────────────────────────────────────────┤
│ 5. agent.execute(user_input)                                    │
│    LLM Extended Thinking:                                       │
│    → "isn't responding" = availability issue                    │
│    → Check AVAILABLE SKILLS section                             │
│    → Match: aws-ec2-troubleshooting (networking, connectivity)  │
│    → Decision: invoke_skill                                     │
│                                                                 │
│ 6. invoke_skill_tool.execute("aws-ec2-troubleshooting")         │
│    - Load skill content from disk (8835 bytes, 3000 tokens)     │
│    - Cache for future use                                       │
│    - Return full diagnostic procedures to agent                 │
│                                                                 │
│ 7. Agent follows skill guidance:                                │
│    → Gather context: aws_find_account, aws_find_region          │
│    → Ask user for instance ID                                   │
│    → Create task agent for diagnostics                          │
│    → Present findings and recommendations                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Takeaways

1. **Skill Discovery** happens once at app startup (cheap, fast)
2. **Skill Metadata** injected into every agent's system prompt (500 tokens)
3. **Skill Content** loaded only when LLM decides it's needed (3000 tokens)
4. **LLM Decides** when to use skills via semantic understanding
5. **Progressive Disclosure** saves ~88% of token costs vs. all-in-prompt
6. **Skills are Guidance** - agents adapt procedures to user context, not rigid scripts
