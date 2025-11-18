# Quick Test Guide - Skill System

## Setup (30 seconds)
```bash
cd /home/fernando/Documents/code/aws-dash-public/worktrees/agent-framework
cargo build
./scripts/sync-skills.sh
```

**Note**: Skill system now initializes automatically at app startup (see `src/app/dashui/app/initialization.rs:25`)

## Test Input & Expected Results

### ✅ Test 1: Skill Recognition - Should Invoke Skill
**Input**: `"My EC2 instance isn't responding to SSH"`

**Expected Agent Behavior**:
1. Recognizes EC2 troubleshooting scenario
2. Calls `invoke_skill('aws-ec2-troubleshooting')`
3. Loads skill (8835 bytes)
4. Asks for account, region, instance ID

**Expected Response**:
```
I've loaded the EC2 troubleshooting skill with diagnostic procedures.
To diagnose why your instance isn't responding to SSH, I need:
1. Which AWS account?
2. Which region?
3. Instance ID?
```

**Log Pattern** (in `~/.local/share/awsdash/logs/awsdash.log`):
```
🎯 invoke_skill executing: aws-ec2-troubleshooting
✅ loaded skill (8835 bytes)
```

---

### ❌ Test 2: No Skill Match - Should NOT Invoke
**Input**: `"List all EC2 instances in us-east-1"`

**Expected Agent Behavior**:
1. Recognizes simple list operation
2. Does NOT invoke skill
3. Calls `create_task` directly

**Expected Response**:
```
I'll list EC2 instances in us-east-1.
[gathers account/region]
Started task agent to list instances.
```

**Log Pattern**:
```
❌ NO invoke_skill calls
✅ create_task called
```

---

### ✅ Test 3: Semantic Matching
**Input**: `"EC2 won't start"`

**Expected**: Agent recognizes this matches EC2 troubleshooting (availability issue)

**Input**: `"Instance is slow"`

**Expected**: Agent recognizes this matches EC2 troubleshooting (performance issue)

---

## What to Monitor

### Terminal 1: Run Application
```bash
cargo run
# OR
./target/debug/awsdash
```

### Terminal 2: Watch Logs
```bash
tail -f ~/.local/share/awsdash/logs/awsdash.log | grep -E "(invoke_skill|Skill|skill)"
```

Look for:
- `🚀 Initializing skill system`
- `✅ Skill system initialized: 1 skills`
- `🎯 invoke_skill executing`
- `✅ loaded skill (8835 bytes)`

---

## Quick Verification

### Before starting:
```bash
# Verify skill exists
ls -lh ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md
# Should show: ~8.6K file

# Verify YAML frontmatter
head -5 ~/.awsdash/skills/aws-ec2-troubleshooting/SKILL.md
# Should show:
# ---
# name: aws-ec2-troubleshooting
# description: Diagnose and resolve EC2 instance issues...
# ---
```

---

## Success Checklist

During your test, verify:
- [ ] Application starts without errors
- [ ] Log shows "Skill system initialized: 1 skills"
- [ ] When you say "EC2 instance isn't responding", agent loads skill
- [ ] When you say "List EC2 instances", agent does NOT load skill
- [ ] Agent's responses reference the skill content (diagnostic procedures)

---

## If Something Goes Wrong

### No skills discovered
```bash
./scripts/sync-skills.sh
```

### Can't find logs
```bash
mkdir -p ~/.local/share/awsdash/logs
```

### Agent doesn't invoke skill
- Check if system prompt includes skills section
- Review agent's reasoning in logs
- Verify skill description is clear

---

## Test Results Template

```
## Test Session: [Date/Time]

### Test 1: EC2 Troubleshooting Recognition
Input: "My EC2 instance isn't responding to SSH"
Result: ✅ / ❌
Notes:

### Test 2: No Skill Match
Input: "List all EC2 instances"
Result: ✅ / ❌
Notes:

### Test 3: Semantic Matching
Input: "EC2 won't start"
Result: ✅ / ❌
Notes:

### Logs Review
- Skills discovered:
- invoke_skill calls:
- Errors/Warnings:
- System prompt includes skills: ✅ / ❌

### Overall Assessment
- Skill system working: ✅ / ❌
- Agent recognition accurate: ✅ / ❌
- Ready for next skill: ✅ / ❌
```
