# Intelligent Task Parsing

## Overview

The intelligent task parser uses LLM (Claude) to analyze task files and automatically:
- Detect hierarchical task structures
- Identify dependencies between tasks
- Assign priorities and complexity estimates
- Determine optimal execution strategies
- Generate detailed execution plans

This provides semantic understanding beyond simple text pattern matching.

## Quick Start

### Basic Usage

```bash
# Auto-detect parser (uses intelligent parser for .claude/tasks.md)
aca --task-file .claude/tasks.md --verbose

# Explicitly use intelligent parser
aca --task-file tasks.md --use-intelligent-parser

# Add context hints to improve analysis
aca --task-file tasks.md --use-intelligent-parser \
    --context "full-stack web application" \
    --context "3 month timeline"
```

### Dump Execution Plan

```bash
# Generate and examine execution plan without executing
aca --task-file .claude/tasks.md --dry-run --dump-plan plan.json

# View the generated plan
cat plan.json

# Or use TOML format
aca --task-file tasks.md --dry-run --dump-plan plan.toml
```

## CLI Flags

### Parser Selection

- `--use-intelligent-parser`: Force use of LLM-based parser
- `--force-naive-parser`: Force use of simple text-based parser
- Default: Auto-detect (intelligent for `--tasks`, naive for `--task-file`)

### Context Hints

```bash
--context "hint"  # Can be used multiple times
```

Examples:
```bash
aca --tasks project.md \
    --use-intelligent-parser \
    --context "React frontend" \
    --context "Node.js backend" \
    --context "team of 5 developers" \
    --context "6 month project"
```

### Plan Dumping

```bash
--dump-plan FILE  # Dump execution plan to JSON or TOML
```

Format is determined by file extension:
- `.json` → JSON format
- `.toml` → TOML format

## Examples

### Example 1: Simple Task List

**Input** (`tasks.md`):
```markdown
# Web Application

## Phase 1: Backend
- Set up database
- Create API endpoints
- Add authentication

## Phase 2: Frontend
- Build UI components
- Implement routing
- Connect to API
```

**Command**:
```bash
aca --task-file tasks.md --use-intelligent-parser --dry-run --dump-plan plan.json
```

**Output** (`plan.json`):
```json
{
  "metadata": {
    "name": "Tasks from tasks.md",
    "description": "Execution plan generated from intelligent LLM analysis (6 tasks)",
    "tags": ["llm-analyzed", "intelligent-parser"]
  },
  "execution_mode": "Sequential",
  "task_specs": [
    {
      "title": "Phase 1: Backend",
      "description": "Set up backend infrastructure",
      "metadata": {
        "priority": "Critical",
        "estimated_complexity": "Complex"
      }
    },
    {
      "title": "Set up database",
      "description": "Configure and initialize database schema",
      "metadata": {
        "priority": "High",
        "estimated_complexity": "Moderate"
      }
    }
    // ... more tasks
  ]
}
```

### Example 2: Complex Multi-Phase Project

**Input** (`.claude/tasks.md`):
```markdown
# E-Commerce Platform

## Phase 1: Infrastructure (Critical Priority)
- [ ] Set up PostgreSQL database
- [ ] Configure Redis cache
- [ ] Set up CI/CD pipeline

## Phase 2: Backend Services (depends on Phase 1)
- [ ] Implement user service
- [ ] Implement product catalog service
- [ ] Implement order processing service

## Phase 3: Frontend (can start after user service)
- [ ] Build customer-facing app
- [ ] Build admin dashboard

## Phase 4: Testing & Deployment (after all above)
- [ ] Write integration tests
- [ ] Load testing
- [ ] Production deployment
```

**Command**:
```bash
export ANTHROPIC_API_KEY=your_key

aca --task-file .claude/tasks.md \
    --use-intelligent-parser \
    --context "e-commerce platform" \
    --context "microservices architecture" \
    --context "6 month timeline" \
    --verbose \
    --dry-run
```

**Output**:
```
🤖 Using intelligent LLM-based task parser
🔍 Analyzing tasks with Claude...

✅ Analysis complete!

📁 Created execution plan: 12 tasks, Intelligent execution mode
  📋 Plan: Tasks from .claude/tasks.md
  📝 Description: Execution plan generated from intelligent LLM analysis (12 tasks)
  🎯 Tasks: 12
      1. Phase 1: Infrastructure
      2.   Set up PostgreSQL database
      3.   Configure Redis cache
      4.   Set up CI/CD pipeline
      5. Phase 2: Backend Services
      6.   Implement user service
      7.   Implement product catalog service
      8.   Implement order processing service
      9. Phase 3: Frontend
     10.   Build customer-facing app
     11.   Build admin dashboard
     12. Phase 4: Testing & Deployment

🔍 Dry run mode - execution plan would be processed but won't actually run
```

### Example 3: Examine Execution Plan Structure

```bash
# Generate plan
aca --task-file .claude/tasks.md \
    --use-intelligent-parser \
    --dry-run \
    --dump-plan my-plan.json

# Examine with jq
jq '.task_specs[] | {title, priority: .metadata.priority}' my-plan.json

# Output:
# {"title": "Phase 1: Infrastructure", "priority": "Critical"}
# {"title": "Set up PostgreSQL database", "priority": "High"}
# ...
```

## Benefits Over Naive Parser

| Feature | Naive Parser | Intelligent Parser |
|---------|-------------|-------------------|
| **Task Detection** | Text patterns only | Semantic understanding |
| **Hierarchy** | Flat list | Parent-child relationships |
| **Dependencies** | None | Auto-detected |
| **Priorities** | All equal | Context-aware |
| **Complexity** | Unknown | Estimated per task |
| **Execution Strategy** | Sequential only | Sequential/Parallel/Intelligent |
| **Time Estimates** | Generic | Per-task estimates |

## When to Use

### Use Intelligent Parser When:

- ✅ Complex multi-phase projects
- ✅ Tasks with implicit dependencies
- ✅ Need priority/complexity estimates
- ✅ Want optimal execution strategy
- ✅ Multiple related tasks in hierarchies

### Use Naive Parser When:

- ✅ Single simple task
- ✅ No dependencies between tasks
- ✅ LLM API unavailable
- ✅ Want faster processing (no API call)
- ✅ Explicitly structured TOML config

## Configuration

### Environment Variables

```bash
# Required for intelligent parser
export ANTHROPIC_API_KEY=sk-ant-...

# Optional: Enable debug logging
export RUST_LOG=aca=debug
```

### Auto-Detection Logic

```
If --force-naive-parser:
    → Use naive parser

Else if --use-intelligent-parser:
    → Use intelligent parser

Else (auto-detect):
    If --task-file:
        → Use naive parser (single task)
    If --tasks:
        → Use intelligent parser (task list)
    If --config:
        → Use naive parser (TOML config)
```

## Performance

| Operation | Time | Cost |
|-----------|------|------|
| Naive parsing | <10ms | Free |
| Intelligent parsing (first) | 2-5s | $0.01-0.05 |
| Intelligent parsing (cached) | <1ms | Free |

**Note**: Intelligent parser caches results, so re-parsing the same file is instant.

## Troubleshooting

### "ANTHROPIC_API_KEY environment variable required"

Set your API key:
```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

### "Intelligent parsing failed"

Claude returned unexpected response. Try:
1. Reduce file complexity
2. Add more context hints
3. Check API key validity
4. Use `--force-naive-parser` as fallback

### Rate Limiting

Default limits: 5 requests/minute, 10000 tokens/minute

If you hit limits, wait or use naive parser for some files.

## Advanced Usage

### Combining with Dry Run

```bash
# Analyze and review before executing
aca --task-file project.md \
    --use-intelligent-parser \
    --dry-run \
    --dump-plan review.json \
    --verbose

# Review the plan
cat review.json

# If satisfied, execute
aca --task-file project.md --use-intelligent-parser
```

### Custom Context for Better Analysis

```bash
aca --task-file tasks.md \
    --use-intelligent-parser \
    --context "technology: Rust, PostgreSQL, React" \
    --context "timeline: 3 months" \
    --context "team: 2 backend, 1 frontend developer" \
    --context "priority: security and performance" \
    --context "existing: legacy Python API to migrate from"
```

## See Also

- [Task Management](./task-management.md)
- [Execution Plans](./execution-plans.md)
- [Configuration Guide](./configuration.md)
- [Examples](../../examples/intelligent-parsing/README.md)
