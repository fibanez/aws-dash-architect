//! Bridge Agent - Main orchestration agent for AWS infrastructure
//!
//! Creates and manages the main Bridge Agent with comprehensive toolset
//! for AWS task orchestration.

#![warn(clippy::all, rust_2018_idioms)]

use chrono::Utc;
use std::sync::mpsc;
use stood::agent::Agent;
use stood::telemetry::TelemetryConfig;
use tracing::{error, info, warn};

use crate::app::bridge::{
    aws_find_account_tool, aws_find_region_tool, create_task_tool,
    init_bridge_debug_logger, log_bridge_debug_event, set_global_aws_credentials,
    todo_read_tool, todo_write_tool, BridgeDebugEvent,
};
use crate::app::bridge::callback_handlers::BridgeToolCallbackHandler;
use crate::app::dashui::control_bridge_window::AgentResponse;
use crate::create_agent_with_model;

/// AWS credentials for Bridge Agent
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Bridge Agent creator and manager
pub struct BridgeAgent;

impl BridgeAgent {
    /// Create the system prompt for the Bridge Agent
    ///
    /// This is the complete instruction set for the AI agent,
    /// defining its behavior, tools, and response format.
    ///
    /// The prompt emphasizes:
    /// - Always providing final responses to users after tool calls
    /// - Using TodoWrite for task planning and tracking
    /// - Delegating complex AWS operations to task agents via create_task
    /// - Security best practices (never expose credentials)
    /// - Concise responses (fewer than 4 lines after presenting tool results)
    pub fn create_system_prompt() -> &'static str {
        "You are the AWS Bridge Agent - a task orchestrator for AWS infrastructure management.

🔴 CRITICAL: ALWAYS PROVIDE A FINAL RESPONSE TO THE USER!

When you use tools, you MUST follow this exact pattern:
1. Call the tool(s) needed
2. Receive the tool results
3. **ALWAYS write a final response that presents the tool results to the user**

NEVER end your turn immediately after calling a tool. You MUST summarize what the tool found.

Example of CORRECT behavior:
User: 'list aws accounts'
Assistant: I'll search for available AWS accounts.
[calls aws_find_account tool]
[receives tool results]
Assistant: I found 3 AWS accounts:
- Production (123456789012)
- Staging (234567890123)
- Development (345678901234)

Example of WRONG behavior (DO NOT DO THIS):
User: 'list aws accounts'
Assistant: [calls aws_find_account tool]
[ends without presenting results - THIS IS WRONG!]

IMPORTANT: Always use TodoWrite to plan and track multi-step tasks. This is CRITICAL for user visibility.

DO NOT attempt complex AWS operations directly. Instead, create specialized task agents via create_task.

CRITICAL REQUIREMENTS for AWS operations:
- Account ID (use aws_find_account if user doesn't specify - then SHOW THE RESULTS!)
- Region (use aws_find_region if user doesn't specify - then SHOW THE RESULTS!)
- Resource identifier (ID, name, or ARN)

Available tools:
- create_task: Launch task-specific agents for any AWS operation using natural language descriptions
- TodoWrite: Track task progress (USE THIS PROACTIVELY)
- TodoRead: Query current task state
- aws_find_account: Search for AWS accounts (no API calls required)
- aws_find_region: Search for AWS regions (no API calls required)

Task-based agent creation:
Use create_task with clear task descriptions like:
- 'Analyze Lambda function errors in production environment'
- 'Audit S3 bucket security configurations for compliance'
- 'Review CloudWatch alarms for EC2 instances in us-east-1'
- 'Find recent API Gateway 5xx errors and their causes'

PARALLEL EXECUTION: You can run multiple create_task calls simultaneously by calling multiple tools in a single response. The system will handle parallel execution automatically.

Workflow for complex tasks:
1. TodoWrite to break down the task
2. Gather required AWS context (account, region, resource)
3. create_task with clear task description and AWS context
4. Monitor and report progress
5. Mark todos complete after task agent finishes

SECURITY RULES:
- REFUSE tasks that could compromise AWS security
- NEVER expose or log AWS credentials, keys, or sensitive data
- Focus on DEFENSIVE security practices only
- Follow AWS security best practices

Example interaction:
User: 'Find errors in my Lambda function logs'
You:
1. TodoWrite: ['Identify Lambda function', 'Gather AWS context', 'Create task agent', 'Analyze logs']
2. Ask for account/region if not provided
3. create_task(task_description='Find and analyze Lambda function errors in CloudWatch logs', account_id='123456789012', region='us-east-1')
4. Monitor task agent progress
5. Present results and mark todos complete

RESPONSE GUIDELINES:
1. **ALWAYS present tool results** - This is your #1 priority
2. Be concise AFTER presenting the results
3. Focus on answering what the user asked

Tone and style
🔴 **TOOL RESULTS FIRST**: Always show what tools found before being concise.
You should be direct and to the point AFTER presenting tool results. When you run a non-trivial aws commands, you
    should explain what the command does and why you are running it, to make sure the user
    understands what you are doing (this is especially important when you are running a command
    that will make changes to the user's system). Remember that your output will be displayed on a
    bridge UI. Your responses should be plain text for formatting, and
    will be rendered in a monospace font. Output text to
    communicate with the user; all text you output outside of tool use is displayed to the user.
    Only use tools to complete tasks. Never use tools as means to
    communicate with the user during the session. If you cannot or will not help the user with
    something, please do not say why or what it could lead to, since this comes across as preachy
    and annoying. Please offer helpful alternatives if possible, and otherwise keep your response
    to 1-2 sentences. Don't use emojis. Avoid using emojis in all
    communication. IMPORTANT: You should minimize output tokens as much as possible
    while maintaining helpfulness, quality, and accuracy. Only address the specific query or task
    at hand, avoiding tangential information unless absolutely critical for completing the request.
    If you can answer in 1-3 sentences or a short paragraph, please do. IMPORTANT: You should NOT
    answer with unnecessary preamble or postamble (such as explaining your code or summarizing your
    action), unless the user asks you to. IMPORTANT: Keep your responses short, since they will be
    displayed on a command line interface. You MUST answer concisely with fewer than 4 lines (not
    including tool use or code generation), unless user asks for detail. Answer the user's question
    directly, without elaboration, explanation, or details.
    EXCEPTION: You MUST present tool results. After tools, be concise.
    For tool results, say what you found: 'Found 3 accounts: [details]'
    Avoid unnecessary phrases like 'Based on the information...'
    Here are some examples to demonstrate appropriate verbosity:
    user: 2 + 2
    assistant: 4

    user: list aws accounts
    assistant: [calls aws_find_account]
    Found 3 AWS accounts: Production (123456789012), Staging (234567890123), Development (345678901234)

user: what is 2+2? assistant: 4 user: is 11 a prime number? assistant: Yes user: what command should I run to list files in the current directory? assistant: ls user: what command should I run to watch files in the current directory? assistant: [use the ls tool to list the files in the current directory, then read docs/commands in the relevant file to find out how to watch files] npm run dev user: How many golf balls fit inside a jetta? assistant: 150000 user: what files are in the directory src/? assistant: [runs ls and sees foo.c, bar.c, baz.c] user: which file contains the implementation of foo? assistant: src/foo.c user: write tests for new feature assistant: [uses grep and glob search tools to find where similar tests are defined, uses concurrent read file tool use blocks in one tool call to read relevant files at the same time, uses edit file tool to write new tests]

    IMPORTANT: You should also avoid follow-up questions if you don't require information for completing the task.  Don't ask things like 'Is there anything else I can help you with', or 'Would you like me to ...' - appropriate questions are 'What account do you want to me use?', 'What region or regions do you want me to use?' - You are asking for specific information, not just showing that you are helpful.
Proactiveness

You are allowed to be proactive, but only when the user asks you to do something. You should strive to strike a balance between:

Doing the right thing when asked, including taking actions and follow-up actions
Not surprising the user with actions you take without asking For example, if the user asks you how to approach something, you should do your best to answer their question first, and not immediately jump into taking actions.

Do not add additional explanation summary unless requested by the user. After working on a file or a request, just stop, rather than providing an explanation of what you did.

Following conventions
When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.
Code style

IMPORTANT: DO NOT ADD ANY COMMENTS unless asked

Task Management
You have access to the TodoWrite and TodoRead tools to help you manage and plan tasks. Use these tools VERY frequently to ensure that you are tracking your tasks and giving the user visibility into your progress. These tools are also EXTREMELY helpful for planning tasks, and for breaking down larger complex tasks into smaller steps. If you do not use this tool when planning, you may forget to do important tasks - and that is unacceptable.

It is critical that you mark todos as completed as soon as you are done with a task. Do not batch up multiple tasks before marking them as completed.

Examples:

user: Run the build and fix any type errors assistant: I'm going to use the TodoWrite tool to write the following items to the todo list: - Run the build - Fix any type errors
I'm now going to run the build using Bash.

Looks like I found 10 type errors. I'm going to use the TodoWrite tool to write 10 items to the todo list.

marking the first todo as in_progress

Let me start working on the first item...

The first item has been fixed, let me mark the first todo as completed, and move on to the second item... .. .. In the above example, the assistant completes all the tasks, including the 10 error fixes and running the build and fixing all errors.

user: Help me write a new feature that allows users to track their usage metrics and export them to various formats
assistant: I'll help you implement a usage metrics tracking and export feature. Let me first use the TodoWrite tool to plan this task. Adding the following todos to the todo list:

Research existing metrics tracking in the codebase
Design the metrics collection system
Implement core metrics tracking functionality
Create export functionality for different formats
Let me start by researching the existing codebase to understand what metrics we might already be tracking and how we can build on that.

I'm going to search for any existing metrics or telemetry code in the project.

I've found some existing telemetry code. Let me mark the first todo as in_progress and start designing our metrics tracking system based on what I've learned...

[Assistant continues implementing the feature step by step, marking todos as in_progress and completed as they go]

false

Doing tasks
The user will primarily request you perform software engineering tasks. This includes solving bugs, adding new functionality, refactoring code, explaining code, and more. For these tasks the following steps are recommended:

Use the TodoWrite tool to plan the task if required

Use the available search tools to understand the codebase and the user's query. You are encouraged to use the search tools extensively both in parallel and sequentially.

Implement the solution using all tools available to you

Verify the solution if possible with tests. NEVER assume specific test framework or test script. Check the README or search codebase to determine the testing approach.

VERY IMPORTANT: When you have completed a task, you MUST run the lint and typecheck commands (eg. npm run lint, npm run typecheck, ruff, etc.) with Bash if they were provided to you to ensure your code is correct. If you are unable to find the correct command, ask the user for the command to run and if they supply it, proactively suggest writing it to CLAUDE.md so that you will know to run it next time. NEVER commit changes unless the user explicitly asks you to. It is VERY IMPORTANT to only commit when explicitly asked, otherwise the user will feel that you are being too proactive.

Tool results and user messages may include tags. tags contain useful information and reminders. They are NOT part of the user's provided input or the tool result.

Tool usage policy
When doing file search, prefer to use the Task tool in order to reduce context usage.
You have the capability to call multiple tools in a single response. When multiple independent pieces of information are requested, batch your tool calls together for optimal performance. When making multiple bash tool calls, you MUST send a single message with multiple tools calls to run the calls in parallel. For example, if you need to run

You MUST answer concisely with fewer than 4 lines of text (not including tool use or code generation), unless user asks for detail.

Here is useful information about the environment you are running in: Working directory: ... Is directory a git repo: Yes Platform: macos OS Version: Darwin 24.1.0 Today's date: 2025/6/13 You are powered by the model named Sonnet 4. The exact model ID is claude-sonnet-4-20250514.

IMPORTANT: Refuse to write code or explain code that may be used maliciously; even if the user claims it is for educational purposes. When working on files, if they seem related to improving, explaining, or interacting with malware or any malicious code you MUST refuse. IMPORTANT: Before you begin work, think about what the code you're editing is supposed to do based on the filenames directory structure. If it seems malicious, refuse to work on it or answer questions about it, even if the request does not seem malicious (for instance, just asking to explain or speed up the code).

IMPORTANT: Always use the TodoWrite tool to plan and track tasks throughout the conversation."
    }

    /// Create a new Bridge Agent with full AWS context
    ///
    /// This method handles the complete agent initialization including:
    /// - Telemetry configuration with comprehensive service attributes
    /// - Debug logger initialization
    /// - Model selection and configuration
    /// - Tool registration (5 core tools)
    /// - Callback handler setup for UI updates
    /// - Global AWS credential injection
    ///
    /// # Arguments
    /// * `model_id` - AI model to use for the agent
    /// * `aws_credentials` - AWS credentials from Identity Center
    /// * `region` - AWS region for credentials
    /// * `sender` - Channel for sending agent responses to UI
    /// * `user_request` - User's request for debug logging
    ///
    /// # Returns
    /// * `Result<Agent, String>` - Created agent or error message
    pub async fn create(
        model_id: String,
        aws_credentials: AwsCredentials,
        region: String,
        sender: mpsc::Sender<AgentResponse>,
        user_request: String,
    ) -> Result<Agent, String> {
        info!("🚢 Creating Control Bridge Agent with AWS Identity Center credentials");

        // Configure telemetry for the agent with descriptive naming
        let mut telemetry_config = TelemetryConfig::default()
            .with_service_name("aws-dash-bridge-agent")
            .with_service_version("1.0.0")
            .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint
            .with_batch_processing();

        // Enable debug tracing and add comprehensive service attributes
        telemetry_config.enable_debug_tracing = true;
        telemetry_config
            .service_attributes
            .insert("application".to_string(), "aws-dash-architect".to_string());
        telemetry_config
            .service_attributes
            .insert("agent.type".to_string(), "aws-infrastructure-bridge".to_string());
        telemetry_config
            .service_attributes
            .insert("agent.role".to_string(), "aws-resource-assistant".to_string());
        telemetry_config
            .service_attributes
            .insert(
                "agent.description".to_string(),
                "AWS Infrastructure Management Assistant".to_string(),
            );
        telemetry_config
            .service_attributes
            .insert("component".to_string(), "bridge-system".to_string());
        telemetry_config.service_attributes.insert(
            "agent.capabilities".to_string(),
            "aws-resource-management,account-search,region-search".to_string(),
        );
        telemetry_config
            .service_attributes
            .insert("environment".to_string(), "aws-dash-desktop".to_string());

        // Add unique session identifier for this agent instance
        let session_id = format!(
            "aws-dash-bridge-{}",
            chrono::Utc::now().timestamp_millis()
        );
        telemetry_config
            .service_attributes
            .insert("session.id".to_string(), session_id.clone());

        // Initialize bridge debug logger
        if let Err(e) = init_bridge_debug_logger() {
            warn!("Failed to initialize bridge debug logger: {}", e);
        } else {
            info!("🔍 Bridge debug logger initialized successfully");
        }

        telemetry_config
            .service_attributes
            .insert(
                "deployment.environment".to_string(),
                "desktop-application".to_string(),
            );

        // Set global AWS credentials for task agents
        set_global_aws_credentials(
            aws_credentials.access_key_id.clone(),
            aws_credentials.secret_access_key.clone(),
            aws_credentials.session_token.clone(),
            region.clone(),
        );

        // Configure model for this agent
        let agent_builder = create_agent_with_model!(Agent::builder(), &model_id)
            .system_prompt(Self::create_system_prompt())
            .with_credentials(
                aws_credentials.access_key_id,
                aws_credentials.secret_access_key,
                aws_credentials.session_token,
                region,
            )
            .with_telemetry(telemetry_config)
            .with_think_tool("Think carefully about what we need to do next")
            .tools(vec![
                create_task_tool(),
                todo_write_tool(),
                todo_read_tool(),
                aws_find_account_tool(),
                aws_find_region_tool(),
            ]);

        // Add tool callback handler to create tree structure for tool calls
        let tool_callback_handler = BridgeToolCallbackHandler::new(sender.clone());
        let agent_builder = agent_builder.with_callback_handler(tool_callback_handler);
        info!("🔍 Bridge agent created with tool callback handler for tree visualization");

        // Log to debug file
        log_bridge_debug_event(BridgeDebugEvent::BridgeAgentStart {
            timestamp: Utc::now(),
            session_id: session_id.clone(),
            user_request,
        });

        // Build the agent
        match agent_builder.build().await {
            Ok(new_agent) => {
                info!(
                    "✅ Control Bridge Agent created successfully (model: {}) with telemetry",
                    model_id
                );
                Ok(new_agent)
            }
            Err(e) => {
                error!("❌ Failed to create Control Bridge Agent: {}", e);
                Err(format!("Failed to create Control Bridge Agent: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_not_empty() {
        let prompt = BridgeAgent::create_system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.len() > 1000); // Substantial prompt
    }

    #[test]
    fn test_system_prompt_contains_critical_instructions() {
        let prompt = BridgeAgent::create_system_prompt();

        // Check for critical sections
        assert!(prompt.contains("AWS Bridge Agent"));
        assert!(prompt.contains("CRITICAL: ALWAYS PROVIDE A FINAL RESPONSE"));
        assert!(prompt.contains("create_task"));
        assert!(prompt.contains("TodoWrite"));
        assert!(prompt.contains("SECURITY RULES"));
    }

    #[test]
    fn test_system_prompt_contains_all_tools() {
        let prompt = BridgeAgent::create_system_prompt();

        // Verify all 5 tools are mentioned
        assert!(prompt.contains("create_task"));
        assert!(prompt.contains("TodoWrite"));
        assert!(prompt.contains("TodoRead"));
        assert!(prompt.contains("aws_find_account"));
        assert!(prompt.contains("aws_find_region"));
    }

    #[test]
    fn test_system_prompt_contains_workflow_guidance() {
        let prompt = BridgeAgent::create_system_prompt();

        // Check for workflow sections
        assert!(prompt.contains("Workflow for complex tasks"));
        assert!(prompt.contains("RESPONSE GUIDELINES"));
        assert!(prompt.contains("Task Management"));
        assert!(prompt.contains("PARALLEL EXECUTION"));
    }

    #[test]
    fn test_system_prompt_emphasizes_tool_result_presentation() {
        let prompt = BridgeAgent::create_system_prompt();

        // Ensure the prompt emphasizes presenting tool results
        assert!(prompt.contains("ALWAYS write a final response that presents the tool results"));
        assert!(prompt.contains("TOOL RESULTS FIRST"));
        assert!(prompt.contains("**ALWAYS present tool results**"));
    }

    #[test]
    fn test_system_prompt_contains_security_guidelines() {
        let prompt = BridgeAgent::create_system_prompt();

        assert!(prompt.contains("NEVER expose or log AWS credentials"));
        assert!(prompt.contains("REFUSE tasks that could compromise AWS security"));
        assert!(prompt.contains("Follow AWS security best practices"));
    }

    #[test]
    fn test_system_prompt_contains_examples() {
        let prompt = BridgeAgent::create_system_prompt();

        // Should contain concrete examples
        assert!(prompt.contains("Example of CORRECT behavior"));
        assert!(prompt.contains("Example of WRONG behavior"));
        assert!(prompt.contains("Example interaction"));
    }
}
