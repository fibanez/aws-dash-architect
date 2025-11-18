# Model Configuration - Bedrock Model Selection

## Component Overview

Provides AI model definitions and configuration for agents. Supports Claude 3.5
(Sonnet, Haiku) and Amazon Nova (Micro, Lite, Pro) models via AWS Bedrock.

**Pattern**: Static configuration with macro-based model selection
**Algorithm**: Match on model_id string to stood::llm::Bedrock enum
**External**: stood::llm::Bedrock, AWS Bedrock API

---

## Major Components

- `ModelConfig` struct - Model metadata (ID, name, provider, description)
- `create_agent_with_model!` macro - Map model_id to stood model enum
- `default_models()` - List of available models

---

## Implementation Patterns

### Pattern: Model Selection via Macro

**Algorithm**: Match expression on model_id string
**External**: stood::llm::Bedrock enum variants

Pseudocode:
  1. create_agent_with_model!(builder, model_id):
     - Match model_id string:
       "anthropic.claude-3-5-sonnet-20241022-v2:0" → Claude35Sonnet
       "anthropic.claude-3-5-haiku-20241022-v1:0" → ClaudeHaiku3
       "amazon.nova-micro-v1:0" → NovaMicro
       "amazon.nova-lite-v1:0" → NovaLite
       "amazon.nova-pro-v1:0" → NovaPro
       Default → ClaudeHaiku3 (fallback)
  2. Calls builder.model(enum_variant)
  3. Returns configured builder

### Pattern: Model Configuration Storage

**Algorithm**: Vec of ModelConfig structs
**External**: serde for serialization

Pseudocode:
  1. ModelConfig fields:
     - model_id: Bedrock API identifier
     - display_name: UI-friendly name
     - provider: "Anthropic" or "Amazon"
     - description: Tooltip text
     - available: bool (feature flags, region support)
  2. default_models() returns hardcoded Vec
  3. UI displays in dropdown/selector
  4. Selected model_id stored in GLOBAL_MODEL_CONFIG

---

## Supported Models

### Anthropic Claude
- **Claude 3.5 Sonnet**: Most capable, complex reasoning
- **Claude 3.5 Haiku**: Fast, efficient, quick tasks

### Amazon Nova
- **Nova Micro**: Lightweight, speed-optimized
- **Nova Lite**: Balanced, general-purpose
- **Nova Pro**: High-performance, complex tasks

---

## External Dependencies

- **stood::llm::Bedrock**: Model enum variants
- **serde**: ModelConfig serialization
- **AWS Bedrock**: Model runtime via stood library

---

## Key Algorithms

### Default Model Selection
Fallback to Claude 3.5 Haiku if model_id unrecognized
Ensures agents always have valid model configuration

### Model Availability
available: bool flag for region/account restrictions
UI can filter models based on availability

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects model_config.rs implementation
