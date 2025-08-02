# DOCS_TODO.md - Documentation Implementation Guide

## 🏗️ AWS Dash: Unified Desktop Environment for Architecting Compliant AWS Solutions

**Project Vision**: AWS Dash is not just a CloudFormation tool - it's a comprehensive unified desktop environment designed for architecting compliant AWS solutions. The application provides an integrated workspace that combines:

- **CloudFormation Template Management**: Visual editing, validation, and deployment
- **AWS Resource Explorer**: Interactive browsing and management of AWS resources
- **Identity Center Integration**: Seamless AWS SSO authentication and role management
- **Project Management**: Organized workspace for complex multi-service architectures
- **Compliance Frameworks**: Built-in support for AWS best practices and compliance standards
- **Visualization Tools**: Interactive graphs and diagrams for architecture understanding

This unified approach enables architects and developers to design, validate, and deploy AWS solutions within a single, cohesive desktop environment rather than juggling multiple tools and browser tabs.

## 📋 Documentation Philosophy & Approach

**CORE PRINCIPLES:**
- 📚 **Progressive Disclosure**: Start with "How to Use" → progress to "How it Works" → finally "How to Extend"
- 🎯 **Multi-Audience Support**: Serve both application users and developers with distinct pathways
- ✅ **Unified Documentation**: In-source rustdoc comments + Markdown extended docs in single pass
- 🔄 **Continuous Validation**: Every commit triggers doc review, full reviews before releases
- 📏 **Cognitive Load Management**: Consistent structure reduces mental overhead
- ✍️ **User-Focused Writing**: Documentation that speaks directly to users' needs and goals


## ✍️ Documentation Writing Style Guidelines

### Voice and Tone
- **User-obsessed**: Frame benefits from the user's perspective ("You'll be able to..." rather than "The library provides...")
- **Confident but not boastful**: State features and benefits matter-of-factly
- **Conversational yet professional**: Use accessible language without being overly casual
- **Action-oriented**: Emphasize what users can do or achieve
- **Trustworthy**: Provide clear, straightforward communication without marketing fluff

### Markdown Formatting Guidelines
- **Standard emphasis**: Use `**text**` for bold, `*text*` for italic
- **Consistent formatting**: Follow GitHub Flavored Markdown standards
- **Cross-reference format**: Use `[Display Text](file-name.md)` for internal links
- **Source code links**: Use `[Link Description](../src/module/mod.rs)` to link Markdown to source files
- **Code blocks**: Use triple backticks with language specification: ```rust
- 
### Writing Principles
- **Clarity above all**: Choose simple words over complex ones, use short sentences when possible
- **Scannable format**: Use bullet points for features, bold text for key information
- **Front-loaded benefits**: Place the most important information first
- **Specific over vague**: "Processes 10,000 records per second" not "high performance"
- **Active voice**: "Parse the configuration" not "The configuration can be parsed"

### Common Phrases and Patterns
- "Developers who use this often combine it with..."
- "Commonly used together with..."
- Start sentences with action verbs: "Create," "Build," "Transform," "Parse"
- Use "you" and "your" frequently to personalize the experience
- "This enables you to..." rather than "This feature allows..."

### Structural Elements
- Concise headlines that include key attributes or benefits
- Feature bullets that start with capital letters but don't end with periods
- Clear hierarchy of information (most to least important)
- Consistent formatting across all documentation
- Code examples immediately after concepts

### What to Avoid
- Exclamation points (!)
- Superlatives and hyperbole ("amazing," "incredible," "best ever")
- Industry jargon without explanation
- Long paragraphs in API descriptions
- Passive voice constructions
- Marketing language or sales pitch tone
- Assumptions about user's knowledge level without context



---

## 📊 Documentation Quality Checklist

After each full review, verify:

- [ ] Every major internal component has rustdoc documentation
- [ ] Every test has explanatory comment
- [ ] Every development script is thoroughly documented
- [ ] Every unsafe block has safety documentation
- [ ] Error handling patterns are documented
- [ ] System interaction patterns are documented
- [ ] Architecture diagrams are up to date
- [ ] Cross-references are valid
- [ ] No outdated information remains


