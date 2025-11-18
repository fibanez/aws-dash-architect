# Skill System Manual Test Plan

## Test Environment Setup

### Prerequisites
1. Build the application: `cargo build`
2. Sync skills: `./scripts/sync-skills.sh`
3. Verify skill exists: `ls ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md`

### Log File Location
- **Application logs**: `~/.local/share/awsdash/logs/awsdash.log`
- Monitor with: `tail -f ~/.local/share/awsdash/logs/awsdash.log`

## Test Scenario 1: Skill Discovery at Startup

### Expected Behavior
When the application starts, the skill system should:
1. Initialize the global skill manager
2. Discover skills in `~/.awsdash/skills/` directory
3. Find the `aws-ec2-troubleshooting` skill
4. Parse YAML frontmatter and extract metadata

### What to Look For in Logs
```
🚀 Initializing skill system
🔍 Discovering skills in: /home/{user}/.awsdash/skills
📁 Found SKILL.md: aws-ec2-troubleshooting
📝 Extracted metadata: name=aws-ec2-troubleshooting, description=Diagnose and resolve EC2 instance issues...
✅ Skill system initialized: 1 skills discovered
```

### Test Steps
1. Clear existing logs: `rm ~/.local/share/awsdash/logs/awsdash.log`
2. Start the application
3. Check logs for skill discovery messages

### Expected Results
- ✅ Log shows "Skill system initialized: 1 skills discovered"
- ✅ Skill metadata extracted correctly
- ✅ No errors in discovery process

---

## Test Scenario 2: Skill Metadata in System Prompt

### Expected Behavior
When the orchestration agent is created:
1. `create_system_prompt()` is called
2. Global skill manager is accessed
3. Skill metadata is retrieved
4. Skills section is injected into prompt with name and description

### What to Look For in Logs
```
🤖 Global model access: ✅ Available
🎯 Creating orchestration agent with model: {model_id}
📝 System prompt generated with 1 skills
```

### Test Steps
1. Create an orchestration agent (when you start a conversation)
2. Check logs for system prompt generation

### Expected Results
- ✅ System prompt includes "AVAILABLE SKILLS" section
- ✅ Lists: "aws-ec2-troubleshooting: Diagnose and resolve EC2 instance issues..."
- ✅ No errors during prompt generation

---

## Test Scenario 3: Agent Recognizes Skill Match

### User Input
```
"My EC2 instance isn't responding to SSH"
```

### Expected Agent Behavior
The agent should:
1. **Analyze** the request (Extended Thinking)
   - Task Type: Complex troubleshooting
   - Skill Match: YES - "isn't responding" matches EC2 troubleshooting
2. **Decide** to invoke skill
3. **Call** `invoke_skill('aws-ec2-troubleshooting')`

### What to Look For in Logs
```
🎯 invoke_skill executing with parameters: {"skill_name": "aws-ec2-troubleshooting"}
🔍 Global skill manager access: ✅ Available
📂 Loading skill: aws-ec2-troubleshooting
✅ invoke_skill completed: loaded skill 'aws-ec2-troubleshooting' (8835 bytes, 0 additional files)
```

### Expected Agent Response
The agent should respond with something like:
```
I've loaded the EC2 troubleshooting skill with diagnostic procedures.
To help diagnose why your instance isn't responding to SSH, I need:
1. Which AWS account is the instance in?
2. Which region?
3. What's the instance ID?
```

### Expected Results
- ✅ Agent recognizes semantic match ("isn't responding" → troubleshooting)
- ✅ Agent invokes skill proactively
- ✅ Skill loads successfully (8835 bytes)
- ✅ Agent uses skill content to guide diagnostic questions

---

## Test Scenario 4: Agent Does NOT Invoke Skill (Correct Negative)

### User Input
```
"List all EC2 instances in us-east-1"
```

### Expected Agent Behavior
The agent should:
1. **Analyze** the request
   - Task Type: Simple list operation
   - Skill Match: NO - this is not troubleshooting/diagnostics
2. **Decide** NOT to invoke skill
3. **Call** `create_task` directly

### What to Look For in Logs
- ❌ NO `invoke_skill` calls
- ✅ `create_task` call for listing instances

### Expected Agent Response
```
I'll list EC2 instances in us-east-1.
[calls aws_find_account]
Found account: Production (123456789012)
[calls create_task]
Started task agent to list EC2 instances.
```

### Expected Results
- ✅ Agent correctly identifies this doesn't need troubleshooting skill
- ✅ No skill loaded
- ✅ Direct delegation to task agent

---

## Test Scenario 5: Invoke Skill with Additional Files

### User Input
```
"I need detailed network diagnostics for my EC2 instance"
```

### Expected Agent Behavior
The agent should:
1. Invoke skill
2. Request additional files if mentioned in skill
3. Load `networking.md` (if it exists)

### What to Look For in Logs
```
🎯 invoke_skill executing with parameters: {
  "skill_name": "aws-ec2-troubleshooting",
  "load_additional_files": ["networking.md"]
}
⚠️ Failed to load additional file 'networking.md': File not found (this is OK for now)
```

### Expected Results
- ✅ Agent attempts to load additional files
- ⚠️ Graceful handling of missing additional files
- ✅ Main skill still loads successfully

---

## Test Scenario 6: Multiple Skills (Future - when we have more skills)

### User Input
```
"Check my EC2 instance performance and review S3 bucket security"
```

### Expected Agent Behavior
1. Recognize TWO skill matches:
   - EC2 performance → `aws-ec2-troubleshooting`
   - S3 security → `aws-s3-security`
2. Invoke both skills
3. Create parallel task agents

### What to Look For in Logs
```
🎯 invoke_skill: aws-ec2-troubleshooting
🎯 invoke_skill: aws-s3-security
```

### Expected Results
- ✅ Agent loads multiple skills
- ✅ Agent creates multiple task agents in parallel

---

## Verification Checklist

### Before Testing
- [ ] Application builds successfully: `cargo build`
- [ ] Skills synced: `./scripts/sync-skills.sh`
- [ ] Skill file exists: `~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md`
- [ ] Log directory exists: `~/.local/share/awsdash/logs/`

### During Testing
- [ ] Monitor logs: `tail -f ~/.local/share/awsdash/logs/awsdash.log`
- [ ] Watch for skill discovery messages on startup
- [ ] Watch for invoke_skill calls during conversation
- [ ] Check for errors or warnings

### Success Criteria
- [ ] Skill system initializes without errors
- [ ] 1 skill discovered (aws-ec2-troubleshooting)
- [ ] Agent recognizes EC2 troubleshooting scenarios
- [ ] Agent successfully loads skill (8835 bytes)
- [ ] Agent uses skill content to guide diagnostics
- [ ] Agent correctly skips skill for non-troubleshooting requests

---

## Troubleshooting

### Issue: Skill not discovered
**Check**:
1. File exists: `ls ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md`
2. Has YAML frontmatter: `head -5 ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md`
3. Correct format: starts with `---`, has `name:` and `description:`

**Fix**: Run `./scripts/sync-skills.sh`

### Issue: Skill system not initialized
**Check logs for**:
- `Skill system not initialized`
- Missing call to `initialize_skill_system()`

**Fix**: Ensure skill system is initialized at application startup

### Issue: Agent doesn't invoke skill
**Possible reasons**:
1. User request doesn't semantically match skill description
2. Agent prompt doesn't emphasize skill usage enough
3. Skill description too vague

**Check**: Review agent's reasoning in logs

### Issue: invoke_skill fails
**Check logs for**:
- `Skill 'aws-ec2-troubleshooting' not found`
- `Available skills: []`

**Fix**: Verify skill discovery completed successfully

---

## Quick Test Commands

```bash
# 1. Build
cargo build

# 2. Sync skills
./scripts/sync-skills.sh

# 3. Verify skill
cat ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md | head -10

# 4. Clear logs
rm ~/.local/share/awsdash/logs/awsdash.log

# 5. Monitor logs
tail -f ~/.local/share/awsdash/logs/awsdash.log

# 6. Run application
./target/debug/awsdash
```

---

## Expected Log Patterns

### Startup
```
🚀 Initializing skill system
🔍 Discovering skills in: /home/{user}/.awsdash/skills
✅ Skill system initialized: 1 skills discovered
```

### Agent Creation
```
🤖 Creating orchestration agent
📝 Global skill manager access: ✅ Available
📝 System prompt generated with skills section
```

### Skill Invocation
```
🎯 invoke_skill executing with parameters: {"skill_name": "aws-ec2-troubleshooting"}
📂 Loading skill: aws-ec2-troubleshooting
✅ invoke_skill completed: loaded skill 'aws-ec2-troubleshooting' (8835 bytes)
```

### Success Indicators
- ✅ = Success log message
- ⚠️ = Warning (non-fatal)
- ❌ = Error (requires investigation)
