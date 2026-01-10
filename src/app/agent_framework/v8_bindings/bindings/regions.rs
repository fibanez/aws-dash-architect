//! Region-related function bindings
//!
//! Provides JavaScript access to AWS region information.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Region information exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    /// AWS Region code (e.g., "us-east-1")
    pub code: String,

    /// Human-readable region name
    pub name: String,
}

/// Get all AWS regions
pub fn get_regions() -> Vec<RegionInfo> {
    crate::app::aws_regions::AWS_REGIONS
        .iter()
        .map(|code| RegionInfo {
            code: code.to_string(),
            name: region_code_to_name(code),
        })
        .collect()
}

/// Convert region code to human-readable name
fn region_code_to_name(code: &str) -> String {
    match code {
        "us-east-1" => "US East (N. Virginia)",
        "us-east-2" => "US East (Ohio)",
        "us-west-1" => "US West (N. California)",
        "us-west-2" => "US West (Oregon)",
        "af-south-1" => "Africa (Cape Town)",
        "ap-east-1" => "Asia Pacific (Hong Kong)",
        "ap-south-1" => "Asia Pacific (Mumbai)",
        "ap-south-2" => "Asia Pacific (Hyderabad)",
        "ap-northeast-1" => "Asia Pacific (Tokyo)",
        "ap-northeast-2" => "Asia Pacific (Seoul)",
        "ap-northeast-3" => "Asia Pacific (Osaka)",
        "ap-southeast-1" => "Asia Pacific (Singapore)",
        "ap-southeast-2" => "Asia Pacific (Sydney)",
        "ap-southeast-3" => "Asia Pacific (Jakarta)",
        "ap-southeast-4" => "Asia Pacific (Melbourne)",
        "ca-central-1" => "Canada (Central)",
        "eu-central-1" => "Europe (Frankfurt)",
        "eu-central-2" => "Europe (Zurich)",
        "eu-west-1" => "Europe (Ireland)",
        "eu-west-2" => "Europe (London)",
        "eu-west-3" => "Europe (Paris)",
        "eu-north-1" => "Europe (Stockholm)",
        "eu-south-1" => "Europe (Milan)",
        "eu-south-2" => "Europe (Spain)",
        "me-central-1" => "Middle East (UAE)",
        "me-south-1" => "Middle East (Bahrain)",
        "sa-east-1" => "South America (São Paulo)",
        "us-gov-east-1" => "AWS GovCloud (US-East)",
        "us-gov-west-1" => "AWS GovCloud (US-West)",
        _ => code, // Fallback to code if unknown
    }
    .to_string()
}

/// Register region-related functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register listRegions() function
    let list_regions_fn = v8::Function::new(scope, list_regions_callback)
        .expect("Failed to create listRegions function");

    let fn_name =
        v8::String::new(scope, "listRegions").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), list_regions_fn.into());

    Ok(())
}

/// Callback for listRegions() JavaScript function
fn list_regions_callback(
    scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Get all regions
    let regions = get_regions();

    // Serialize to JSON string
    let json_str = match serde_json::to_string(&regions) {
        Ok(json) => json,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("Failed to serialize regions: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Create V8 string from JSON
    let v8_str = match v8::String::new(scope, &json_str) {
        Some(s) => s,
        None => {
            let msg = v8::String::new(scope, "Failed to create V8 string").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON in V8 to create JavaScript array
    let v8_value = match v8::json::parse(scope, v8_str) {
        Some(v) => v,
        None => {
            let msg = v8::String::new(scope, "Failed to parse JSON in V8").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(v8_value);
}

/// Get LLM documentation for region functions
pub fn get_documentation() -> String {
    r#"
### listRegions()

List all AWS regions available in the system.

**Signature:**
```typescript
function listRegions(): RegionInfo[]
```

**Description:**
Returns an array of AWS region objects with their codes and human-readable names.
No credentials or configuration needed.

**Return value structure:**
```json
[
  {
    "code": "us-east-1",
    "name": "US East (N. Virginia)"
  },
  {
    "code": "eu-west-1",
    "name": "Europe (Ireland)"
  },
  {
    "code": "ap-southeast-1",
    "name": "Asia Pacific (Singapore)"
  }
]
```

**Field descriptions:**
- `code` (string): AWS Region code (e.g., "us-east-1", "eu-west-2")
- `name` (string): Human-readable region name

**Example usage:**
```javascript
// Get all regions
const regions = listRegions();
console.log(`Found ${regions.length} regions`);

// Find specific region by code
const usEast = regions.find(r => r.code === 'us-east-1');
if (usEast) {
  console.log(`Region: ${usEast.name}`);
}

// Filter regions by prefix
const usRegions = regions.filter(r => r.code.startsWith('us-'));
console.log(`US regions: ${usRegions.length}`);

// Get all region codes
const regionCodes = regions.map(r => r.code);

// Find regions in Europe
const euRegions = regions.filter(r => r.code.startsWith('eu-'));
euRegions.forEach(r => console.log(`${r.code}: ${r.name}`));

// Check if a specific region exists
const hasApSouth = regions.some(r => r.code === 'ap-south-1');
```

**Edge cases:**
- Always returns a full array of regions (never empty)
- Never returns `null` or `undefined` - always returns an array

**Error handling:**
```javascript
const regions = listRegions();

// Safe to use directly - always returns valid array
const codes = regions.map(r => r.code);
console.log(`Available region codes: ${codes.join(', ')}`);
```
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
    use std::pin::pin;

    #[test]
    fn test_get_regions() {
        let regions = get_regions();

        assert!(!regions.is_empty());
        assert!(regions.len() > 20); // We should have many regions

        // Verify structure
        let region = &regions[0];
        assert!(!region.code.is_empty());
        assert!(!region.name.is_empty());
    }

    #[test]
    fn test_region_code_to_name() {
        assert_eq!(region_code_to_name("us-east-1"), "US East (N. Virginia)");
        assert_eq!(region_code_to_name("eu-west-1"), "Europe (Ireland)");
        assert_eq!(
            region_code_to_name("ap-southeast-1"),
            "Asia Pacific (Singapore)"
        );
    }

    #[test]
    fn test_list_regions_binding() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Register the binding
        register(scope).unwrap();

        // Execute JavaScript that calls listRegions()
        let code = v8::String::new(scope, "listRegions()").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();

        // Should return an array
        assert!(result.is_array());

        // Convert to JavaScript array
        let array = v8::Local::<v8::Array>::try_from(result).unwrap();
        assert!(array.length() > 0);
    }

    #[test]
    fn test_list_regions_javascript_access() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        register(scope).unwrap();

        // Test JavaScript can access region properties
        let code = r#"
            const regions = listRegions();
            const firstRegion = regions[0];
            JSON.stringify({
                count: regions.length,
                firstCode: firstRegion.code,
                firstName: firstRegion.name
            })
        "#;

        let code_str = v8::String::new(scope, code).unwrap();
        let script = v8::Script::compile(scope, code_str, None).unwrap();
        let result = script.run(scope).unwrap();

        let result_str = result.to_string(scope).unwrap();
        let result_json = result_str.to_rust_string_lossy(scope);

        // Verify JavaScript could access properties
        assert!(result_json.contains("count"));
        assert!(result_json.contains("firstCode"));
        assert!(result_json.contains("firstName"));
    }

    #[test]
    fn test_list_regions_filtering() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        register(scope).unwrap();

        // Test JavaScript can filter and map regions
        let code = r#"
            const regions = listRegions();
            const usRegions = regions.filter(r => r.code.startsWith('us-'));
            const regionCodes = regions.map(r => r.code);
            JSON.stringify({
                totalRegions: regions.length,
                usCount: usRegions.length,
                firstCode: regionCodes[0]
            })
        "#;

        let code_str = v8::String::new(scope, code).unwrap();
        let script = v8::Script::compile(scope, code_str, None).unwrap();
        let result = script.run(scope).unwrap();

        let result_str = result.to_string(scope).unwrap();
        let result_json = result_str.to_rust_string_lossy(scope);

        // Verify operations worked
        assert!(result_json.contains("totalRegions"));
        assert!(result_json.contains("usCount"));
        assert!(result_json.contains("firstCode"));
    }

    #[test]
    fn test_documentation_format() {
        let docs = get_documentation();

        // Verify required documentation elements
        assert!(docs.contains("listRegions()"));
        assert!(docs.contains("function listRegions()"));
        assert!(docs.contains("Return value structure:"));
        assert!(docs.contains("```json"));
        assert!(docs.contains("Field descriptions:"));
        assert!(docs.contains("Example usage:"));
        assert!(docs.contains("Edge cases:"));
        assert!(docs.contains("Error handling:"));
    }
}
