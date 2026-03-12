
/// Top-level plugin options controlling the compiler behavior.
#[derive(Debug, Clone)]
pub struct PluginOptions {
    pub compilation_mode: CompilationMode,
    pub output_mode: OutputMode,
    pub target: ReactTarget,
    pub gating: Option<GatingConfig>,
    pub panic_threshold: PanicThreshold,
    pub sources: Option<SourceFilter>,
}

impl Default for PluginOptions {
    fn default() -> Self {
        Self {
            compilation_mode: CompilationMode::Infer,
            output_mode: OutputMode::Client,
            target: ReactTarget::React19,
            gating: None,
            panic_threshold: PanicThreshold::CriticalErrors,
            sources: None,
        }
    }
}

/// Determines how the compiler finds functions to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilationMode {
    /// Infer whether to compile based on heuristics (default)
    Infer,
    /// Only compile functions with explicit annotations ("use memo", "use no memo")
    Annotation,
    /// Compile all top-level functions that look like components/hooks
    Syntax,
    /// Compile everything
    All,
}

/// Controls what the compiler outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputMode {
    /// Normal client-side compilation with memoization
    Client,
    /// Client-side without memoization (benchmarking/testing the raw transform)
    ClientNoMemo,
    /// Server-side rendering mode (different memoization strategy)
    SSR,
    /// Lint-only mode: run analysis, collect errors, skip codegen
    Lint,
}

/// Target React version for generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactTarget {
    React17,
    React18,
    React19,
}

/// Controls when the compiler bails out due to errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanicThreshold {
    /// Bail on any error
    AllErrors,
    /// Bail only on critical errors (invariant violations)
    CriticalErrors,
    /// Never bail, collect all errors
    None,
}

/// Configuration for gating compiled output behind a feature flag.
#[derive(Debug, Clone)]
pub struct GatingConfig {
    /// Import source for the gating function
    pub import_source: String,
    /// Function name to use as a gating check
    pub function_name: String,
}

/// Filter for which source files to compile.
#[derive(Debug, Clone)]
pub struct SourceFilter {
    /// Glob patterns for files to include
    pub include: Vec<String>,
    /// Glob patterns for files to exclude
    pub exclude: Vec<String>,
}

impl PluginOptions {
    /// Parse options from a JSON-like key-value map.
    pub fn from_map(map: &std::collections::HashMap<String, String>) -> Self {
        let mut opts = Self::default();

        if let Some(mode) = map.get("compilationMode") {
            opts.compilation_mode = match mode.as_str() {
                "all" => CompilationMode::All,
                "syntax" => CompilationMode::Syntax,
                "annotation" => CompilationMode::Annotation,
                _ => CompilationMode::Infer,
            };
        }

        if let Some(output) = map.get("outputMode") {
            opts.output_mode = match output.as_str() {
                "ssr" => OutputMode::SSR,
                "lint" => OutputMode::Lint,
                "client-no-memo" => OutputMode::ClientNoMemo,
                _ => OutputMode::Client,
            };
        }

        if let Some(target) = map.get("target") {
            opts.target = match target.as_str() {
                "17" | "react17" => ReactTarget::React17,
                "18" | "react18" => ReactTarget::React18,
                _ => ReactTarget::React19,
            };
        }

        if let Some(threshold) = map.get("panicThreshold") {
            opts.panic_threshold = match threshold.as_str() {
                "all" | "ALL_ERRORS" => PanicThreshold::AllErrors,
                "none" | "NONE" => PanicThreshold::None,
                _ => PanicThreshold::CriticalErrors,
            };
        }

        opts
    }
}

impl GatingConfig {
    /// Generate the gating wrapper code.
    pub fn generate_wrapper(&self, compiled_code: &str) -> String {
        format!(
            "import {{ {} }} from \"{}\";\n\
             if ({}()) {{\n{}\n}}",
            self.function_name, self.import_source, self.function_name, compiled_code
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_options() {
        let opts = PluginOptions::default();
        assert_eq!(opts.compilation_mode, CompilationMode::Infer);
        assert_eq!(opts.output_mode, OutputMode::Client);
        assert_eq!(opts.target, ReactTarget::React19);
        assert_eq!(opts.panic_threshold, PanicThreshold::CriticalErrors);
        assert!(opts.gating.is_none());
        assert!(opts.sources.is_none());
    }

    #[test]
    fn from_map_empty() {
        let map = HashMap::new();
        let opts = PluginOptions::from_map(&map);
        assert_eq!(opts.compilation_mode, CompilationMode::Infer);
        assert_eq!(opts.output_mode, OutputMode::Client);
        assert_eq!(opts.target, ReactTarget::React19);
        assert_eq!(opts.panic_threshold, PanicThreshold::CriticalErrors);
    }

    #[test]
    fn compilation_mode_parsing() {
        let cases = [
            ("all", CompilationMode::All),
            ("syntax", CompilationMode::Syntax),
            ("annotation", CompilationMode::Annotation),
            ("infer", CompilationMode::Infer),
            ("unknown", CompilationMode::Infer),
            ("ALL", CompilationMode::Infer), // case-sensitive
        ];
        for (input, expected) in cases {
            let map = HashMap::from([("compilationMode".to_string(), input.to_string())]);
            let opts = PluginOptions::from_map(&map);
            assert_eq!(opts.compilation_mode, expected, "input: {input}");
        }
    }

    #[test]
    fn output_mode_parsing() {
        let cases = [
            ("ssr", OutputMode::SSR),
            ("lint", OutputMode::Lint),
            ("client", OutputMode::Client),
            ("unknown", OutputMode::Client),
        ];
        for (input, expected) in cases {
            let map = HashMap::from([("outputMode".to_string(), input.to_string())]);
            let opts = PluginOptions::from_map(&map);
            assert_eq!(opts.output_mode, expected, "input: {input}");
        }
    }

    #[test]
    fn react_target_parsing() {
        let cases = [
            ("17", ReactTarget::React17),
            ("react17", ReactTarget::React17),
            ("18", ReactTarget::React18),
            ("react18", ReactTarget::React18),
            ("19", ReactTarget::React19),
            ("react19", ReactTarget::React19),
            ("unknown", ReactTarget::React19),
        ];
        for (input, expected) in cases {
            let map = HashMap::from([("target".to_string(), input.to_string())]);
            let opts = PluginOptions::from_map(&map);
            assert_eq!(opts.target, expected, "input: {input}");
        }
    }

    #[test]
    fn panic_threshold_parsing() {
        let cases = [
            ("all", PanicThreshold::AllErrors),
            ("ALL_ERRORS", PanicThreshold::AllErrors),
            ("none", PanicThreshold::None),
            ("NONE", PanicThreshold::None),
            ("critical", PanicThreshold::CriticalErrors),
            ("unknown", PanicThreshold::CriticalErrors),
        ];
        for (input, expected) in cases {
            let map = HashMap::from([("panicThreshold".to_string(), input.to_string())]);
            let opts = PluginOptions::from_map(&map);
            assert_eq!(opts.panic_threshold, expected, "input: {input}");
        }
    }

    #[test]
    fn from_map_multiple_keys() {
        let map = HashMap::from([
            ("compilationMode".to_string(), "all".to_string()),
            ("outputMode".to_string(), "ssr".to_string()),
            ("target".to_string(), "react17".to_string()),
            ("panicThreshold".to_string(), "none".to_string()),
        ]);
        let opts = PluginOptions::from_map(&map);
        assert_eq!(opts.compilation_mode, CompilationMode::All);
        assert_eq!(opts.output_mode, OutputMode::SSR);
        assert_eq!(opts.target, ReactTarget::React17);
        assert_eq!(opts.panic_threshold, PanicThreshold::None);
    }

    #[test]
    fn from_map_unknown_keys_ignored() {
        let map = HashMap::from([
            ("unknownKey".to_string(), "value".to_string()),
            ("another".to_string(), "thing".to_string()),
        ]);
        let opts = PluginOptions::from_map(&map);
        // Should produce defaults, not panic
        assert_eq!(opts.compilation_mode, CompilationMode::Infer);
    }

    #[test]
    fn gating_config_wrapper() {
        let config = GatingConfig {
            import_source: "my-flags".to_string(),
            function_name: "isFeatureEnabled".to_string(),
        };
        let wrapper = config.generate_wrapper("  console.log('compiled');");
        assert!(wrapper.contains("import { isFeatureEnabled } from \"my-flags\""));
        assert!(wrapper.contains("if (isFeatureEnabled())"));
        assert!(wrapper.contains("console.log('compiled')"));
    }

    #[test]
    fn source_filter_construction() {
        let filter = SourceFilter {
            include: vec!["src/**/*.tsx".to_string()],
            exclude: vec!["**/*.test.tsx".to_string()],
        };
        assert_eq!(filter.include.len(), 1);
        assert_eq!(filter.exclude.len(), 1);
        assert_eq!(filter.include[0], "src/**/*.tsx");
        assert_eq!(filter.exclude[0], "**/*.test.tsx");
    }
}
