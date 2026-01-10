/// JavaScript Deobfuscator - Rename variables using heuristics
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use regex::Regex;

/// Variable usage context for determining meaningful names
#[derive(Debug, Clone)]
struct VariableContext {
    name: String,
    // Usage patterns
    is_function_param: bool,
    is_callback: bool,
    is_dom_element: bool,
    is_array_like: bool,
    is_object_like: bool,
    is_promise: bool,
    is_event: bool,
    is_error: bool,
    is_config: bool,
    is_data: bool,

    // Operations performed
    has_push_call: bool,
    has_map_call: bool,
    has_filter_call: bool,
    has_foreach_call: bool,
    has_length_access: bool,
    has_then_call: bool,
    has_catch_call: bool,
    has_query_selector: bool,
    has_add_event_listener: bool,

    // Common patterns
    used_in_return: bool,
    used_in_condition: bool,
    assigned_from: Vec<String>,
}

impl VariableContext {
    fn new(name: String) -> Self {
        Self {
            name,
            is_function_param: false,
            is_callback: false,
            is_dom_element: false,
            is_array_like: false,
            is_object_like: false,
            is_promise: false,
            is_event: false,
            is_error: false,
            is_config: false,
            is_data: false,
            has_push_call: false,
            has_map_call: false,
            has_filter_call: false,
            has_foreach_call: false,
            has_length_access: false,
            has_then_call: false,
            has_catch_call: false,
            has_query_selector: false,
            has_add_event_listener: false,
            used_in_return: false,
            used_in_condition: false,
            assigned_from: Vec::new(),
        }
    }
}

pub struct JSDeobfuscator {
    code: String,
    variable_contexts: HashMap<String, VariableContext>,
    renamed_vars: HashMap<String, String>,
    verbose: bool,
}

impl JSDeobfuscator {
    pub fn new(code: String) -> Self {
        Self {
            code,
            variable_contexts: HashMap::new(),
            renamed_vars: HashMap::new(),
            verbose: true,
        }
    }

    pub fn with_verbosity(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let code = fs::read_to_string(path)?;
        Ok(Self::new(code))
    }

    /// Main deobfuscation pipeline
    pub fn deobfuscate(&mut self) -> String {
        if self.verbose {
            println!("[*] Step 1: Analyzing variable usage patterns...");
        }
        self.analyze_variables();

        if self.verbose {
            println!("[*] Step 2: Generating meaningful names using heuristics...");
        }
        self.generate_names();

        if self.verbose {
            println!("[*] Step 3: Applying renames...");
        }
        let result = self.apply_renames();

        if self.verbose {
            println!("[+] Deobfuscation complete! Renamed {} variables", self.renamed_vars.len());
        }
        result
    }

    /// Analyze all variables and their usage patterns
    fn analyze_variables(&mut self) {
        // Find all single-letter, dollar sign, and short minified variable names
        // Matches: a, x, H, $, a1, x2, H0, _a, _H, GaD (3 chars with caps)
        let patterns = vec![
            r"\b([a-z])\b",                    // Single lowercase: a, x
            r"\b([A-Z])\b",                    // Single uppercase: H, A, L
            r"\$",                              // Dollar sign: $
            r"\b([a-z][0-9])\b",               // Lowercase + digit: a1, x2
            r"\b([A-Z][0-9])\b",               // Uppercase + digit: H0, A1
            r"\b(_[a-zA-Z])\b",                // Underscore + letter: _a, _H
            r"\b([A-Z][a-z]+[A-Z][a-z]*)\b",   // CamelCase minified: GaD, KaD, UaD
        ];

        for pattern in patterns {
            let var_re = Regex::new(pattern).unwrap();
            for caps in var_re.captures_iter(&self.code) {
                let var_name = caps.get(0).map(|m| m.as_str()).unwrap_or("$").to_string();

                if !self.variable_contexts.contains_key(&var_name) && var_name.len() <= 3 {
                    let mut ctx = VariableContext::new(var_name.clone());
                    self.analyze_variable_usage(&var_name, &mut ctx);
                    self.variable_contexts.insert(var_name, ctx);
                }
            }
        }
    }

    /// Analyze how a specific variable is used
    fn analyze_variable_usage(&self, var_name: &str, ctx: &mut VariableContext) {
        // Array operations
        if self.code.contains(&format!("{}.push(", var_name)) {
            ctx.has_push_call = true;
            ctx.is_array_like = true;
        }
        if self.code.contains(&format!("{}.map(", var_name)) {
            ctx.has_map_call = true;
            ctx.is_array_like = true;
        }
        if self.code.contains(&format!("{}.filter(", var_name)) {
            ctx.has_filter_call = true;
            ctx.is_array_like = true;
        }
        if self.code.contains(&format!("{}.forEach(", var_name)) {
            ctx.has_foreach_call = true;
            ctx.is_array_like = true;
        }
        if self.code.contains(&format!("{}.length", var_name)) {
            ctx.has_length_access = true;
        }

        // Promise operations
        if self.code.contains(&format!("{}.then(", var_name)) {
            ctx.has_then_call = true;
            ctx.is_promise = true;
        }
        if self.code.contains(&format!("{}.catch(", var_name)) {
            ctx.has_catch_call = true;
            ctx.is_promise = true;
        }

        // DOM operations
        if self.code.contains(&format!("{}.querySelector", var_name)) {
            ctx.has_query_selector = true;
            ctx.is_dom_element = true;
        }
        if self.code.contains(&format!("{}.addEventListener(", var_name)) {
            ctx.has_add_event_listener = true;
            ctx.is_dom_element = true;
        }
        if self.code.contains(&format!("{}.createElement", var_name)) {
            ctx.is_dom_element = true;
        }
        if self.code.contains(&format!("{}.getElementById", var_name)) {
            ctx.is_dom_element = true;
        }

        // Common patterns
        if self.code.contains(&format!("return {}", var_name)) {
            ctx.used_in_return = true;
        }
        if self.code.contains(&format!("if({})", var_name)) ||
           self.code.contains(&format!("if ({})", var_name)) {
            ctx.used_in_condition = true;
        }

        // Check for error patterns
        if self.code.contains(&format!("catch({})", var_name)) ||
           self.code.contains(&format!("catch ({})", var_name)) {
            ctx.is_error = true;
        }

        // Check for event patterns
        if self.code.contains(&format!("function({}", var_name)) &&
           (self.code.contains("event") || self.code.contains("Event")) {
            ctx.is_event = true;
        }

        // Check for data/response patterns
        let assignment_re = Regex::new(&format!(r"var\s+{}\s*=\s*(\w+)", var_name)).unwrap();
        if let Some(caps) = assignment_re.captures(&self.code) {
            if let Some(source) = caps.get(1) {
                ctx.assigned_from.push(source.as_str().to_string());

                // Infer types from assignment source
                let source_str = source.as_str().to_lowercase();
                if source_str.contains("response") || source_str.contains("data") {
                    ctx.is_data = true;
                }
                if source_str.contains("config") || source_str.contains("options") {
                    ctx.is_config = true;
                }
            }
        }

        // Check if it's a callback (function passed as argument)
        let callback_re = Regex::new(&format!(r"\w+\([^)]*function\s*\([^)]*{}\s*\)", var_name)).unwrap();
        if callback_re.is_match(&self.code) {
            ctx.is_callback = true;
        }
    }

    /// Generate meaningful names based on context
    fn generate_names(&mut self) {
        let mut name_counters: HashMap<String, usize> = HashMap::new();

        for (var_name, ctx) in &self.variable_contexts {
            let new_name = self.infer_name(ctx, &mut name_counters);

            // Only rename if the new name is more meaningful
            if new_name != *var_name && var_name.len() <= 2 {
                self.renamed_vars.insert(var_name.clone(), new_name);
            }
        }
    }

    /// Infer a meaningful name based on usage context
    fn infer_name(&self, ctx: &VariableContext, counters: &mut HashMap<String, usize>) -> String {
        let base_name = if ctx.is_error {
            "error"
        } else if ctx.is_event {
            "event"
        } else if ctx.is_promise && ctx.is_data {
            "responseData"
        } else if ctx.is_promise {
            "promise"
        } else if ctx.is_callback {
            "callback"
        } else if ctx.is_dom_element && ctx.has_query_selector {
            "element"
        } else if ctx.is_dom_element {
            "node"
        } else if ctx.has_map_call || ctx.has_filter_call {
            "items"
        } else if ctx.is_array_like && ctx.has_foreach_call {
            "list"
        } else if ctx.is_array_like {
            "array"
        } else if ctx.has_then_call {
            "result"
        } else if ctx.is_config {
            "config"
        } else if ctx.is_data {
            "data"
        } else if ctx.used_in_return && !ctx.used_in_condition {
            "result"
        } else if ctx.has_length_access {
            "collection"
        } else {
            // Check assigned_from for hints
            if !ctx.assigned_from.is_empty() {
                let source = &ctx.assigned_from[0];
                if source.contains("fetch") || source.contains("ajax") {
                    "response"
                } else if source.contains("parse") || source.contains("JSON") {
                    "parsed"
                } else {
                    "value"
                }
            } else {
                "value"
            }
        };

        // Add counter if name already used
        let counter = counters.entry(base_name.to_string()).or_insert(0);
        *counter += 1;

        if *counter == 1 {
            base_name.to_string()
        } else {
            format!("{}{}", base_name, counter)
        }
    }

    /// Apply all renames to the code
    fn apply_renames(&self) -> String {
        let mut result = self.code.clone();

        // Sort by length (longest first) to avoid partial replacements
        let mut sorted_renames: Vec<_> = self.renamed_vars.iter().collect();
        sorted_renames.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));

        for (old_name, new_name) in sorted_renames {
            // Use word boundaries to avoid replacing parts of other identifiers
            let pattern = format!(r"\b{}\b", regex::escape(old_name));
            let re = Regex::new(&pattern).unwrap();
            result = re.replace_all(&result, new_name.as_str()).to_string();
        }

        result
    }

    /// Generate a report of renames
    pub fn print_report(&self) {
        println!("\n=== Deobfuscation Report ===\n");
        println!("Variables renamed: {}", self.renamed_vars.len());

        let mut sorted: Vec<_> = self.renamed_vars.iter().collect();
        sorted.sort_by_key(|(_, v)| v.as_str());

        println!("\nRename mappings:");
        for (old, new) in sorted {
            if let Some(ctx) = self.variable_contexts.get(old) {
                let hints = self.get_context_hints(ctx);
                println!("  {} → {} ({})", old, new, hints);
            }
        }
    }

    fn get_context_hints(&self, ctx: &VariableContext) -> String {
        let mut hints = Vec::new();

        if ctx.is_array_like { hints.push("array"); }
        if ctx.is_promise { hints.push("promise"); }
        if ctx.is_dom_element { hints.push("DOM"); }
        if ctx.is_error { hints.push("error"); }
        if ctx.is_callback { hints.push("callback"); }
        if ctx.has_map_call { hints.push("mapped"); }
        if ctx.has_then_call { hints.push("async"); }

        if hints.is_empty() {
            "inferred".to_string()
        } else {
            hints.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_detection() {
        let code = r#"
            function test() {
                var a = [];
                a.push(1);
                a.map(x => x * 2);
                return a;
            }
        "#.to_string();

        let mut deobf = JSDeobfuscator::new(code);
        deobf.deobfuscate();

        // 'a' should be renamed to something array-like
        assert!(deobf.renamed_vars.contains_key("a"));
    }

    #[test]
    fn test_promise_detection() {
        let code = r#"
            var p = fetch(url);
            p.then(r => r.json());
        "#.to_string();

        let mut deobf = JSDeobfuscator::new(code);
        deobf.deobfuscate();

        assert!(deobf.renamed_vars.contains_key("p"));
        assert!(deobf.renamed_vars.contains_key("r"));
    }
}
