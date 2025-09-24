# Reference Files

This directory contains reference documents that can be included in tasks for additional context.

## 📖 Reference Documents

### `memory_leak_analysis.md`
**Use case:** Detailed technical analysis for debugging tasks

A comprehensive analysis document that demonstrates how to provide structured technical information for complex debugging tasks. Contains:
- Issue description and symptoms
- Investigation findings and root cause analysis
- Recommended solutions with priority levels
- Testing and validation plans

**Referenced by:** `../task-inputs/task_list.md`

### `caching_strategy.txt`
**Use case:** Implementation specification and requirements

A detailed technical specification showing how to document implementation requirements for system architecture tasks. Includes:
- Multi-layer architecture overview
- Detailed implementation specifications
- Configuration and monitoring requirements
- Performance and operational considerations

**Referenced by:** `../task-inputs/task_list.md`

## 🔗 How Reference Files Work

Reference files are automatically included when tasks reference them using the `->` syntax:

```markdown
- [ ] Fix memory leak in data processor -> ../references/memory_leak_analysis.md
```

When ACA processes this task, it will:
1. Read the main task description
2. Load the referenced file content
3. Include the reference content in the task context
4. Process the combined information as a single task

## 📝 Creating Effective Reference Files

### Technical Analysis Documents
Structure for debugging and investigation:
```markdown
# [Issue Name]

## Issue Description
Brief overview of the problem

## Symptoms Observed
- Observable behaviors
- Error conditions
- Performance impacts

## Investigation Findings
### Root Cause Analysis
### Code Locations Affected

## Recommended Solutions
1. Immediate fixes
2. Long-term improvements

## Testing Plan
Verification steps
```

### Implementation Specifications
Structure for feature requirements:
```markdown
# [Feature/System Name]

## Overview
High-level description

## Requirements
- Functional requirements
- Non-functional requirements

## Architecture
- System design
- Component interactions

## Implementation Details
- Technical specifications
- Configuration requirements

## Monitoring & Operations
- Performance metrics
- Operational procedures
```

### Best Practices

1. **Be Specific**: Include exact file paths, line numbers, function names
2. **Provide Context**: Explain the business impact and technical implications
3. **Include Examples**: Show code snippets, configuration examples, test cases
4. **Structure Information**: Use headings and lists for easy scanning
5. **Cross-Reference**: Link related files and documentation

### File Formats
Reference files can be in any UTF-8 text format:
- `.md` - Markdown (recommended for structured documents)
- `.txt` - Plain text (good for simple specifications)
- `.json` - Configuration examples
- `.yaml/.yml` - Configuration files
- Any other text format

## 🎯 Usage Patterns

### Analysis-Driven Development
Create analysis documents for:
- Bug investigations
- Performance bottlenecks
- Security assessments
- Code reviews

### Specification-Driven Implementation
Create specification documents for:
- New feature requirements
- Architecture decisions
- API designs
- Integration requirements

### Documentation-Driven Maintenance
Create reference docs for:
- Operational procedures
- Troubleshooting guides
- Configuration references
- Best practices