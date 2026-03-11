#![allow(dead_code)]

use super::object_shape::{
    CallKind, FunctionSignature, ObjectShape, ParamEffect, PropertyShape, ShapeId, ShapeRegistry,
};
use rustc_hash::FxHashMap;

/// Result of global registration — holds ShapeIds for key globals.
#[derive(Debug)]
pub struct GlobalRegistry {
    pub array_shape: ShapeId,
    pub object_shape: ShapeId,
    pub math_shape: ShapeId,
    pub json_shape: ShapeId,
    pub console_shape: ShapeId,
    pub string_shape: ShapeId,
    pub number_shape: ShapeId,
    pub promise_shape: ShapeId,
    pub react_shape: ShapeId,
    /// Maps global names to their shape IDs
    pub globals: FxHashMap<String, ShapeId>,
}

/// Register all built-in global shapes into the registry.
pub fn register_globals(registry: &mut ShapeRegistry) -> GlobalRegistry {
    let mut globals = FxHashMap::default();

    // Helper: create a pure function signature
    let pure_fn = |params: Vec<ParamEffect>| FunctionSignature {
        params,
        return_shape: ShapeId::NONE,
        call_kind: CallKind::Pure,
        no_alias: true,
    };

    // Helper: create a function that reads its arguments
    let read_fn = |n: usize| FunctionSignature {
        params: vec![ParamEffect::Read; n],
        return_shape: ShapeId::NONE,
        call_kind: CallKind::Normal,
        no_alias: true,
    };

    // Helper: create a hook function signature
    let hook_fn = |params: Vec<ParamEffect>| FunctionSignature {
        params,
        return_shape: ShapeId::NONE,
        call_kind: CallKind::Hook,
        no_alias: true,
    };

    // Helper: create an impure function signature
    let impure_fn = || FunctionSignature {
        params: vec![],
        return_shape: ShapeId::NONE,
        call_kind: CallKind::Impure,
        no_alias: true,
    };

    // --- Array ---
    let mut array = ObjectShape::new();
    // Mutating methods
    for method in
        &["push", "pop", "shift", "unshift", "splice", "sort", "reverse", "fill", "copyWithin"]
    {
        let mut sig = FunctionSignature {
            params: vec![ParamEffect::Read],
            return_shape: ShapeId::NONE,
            call_kind: CallKind::Normal,
            no_alias: true,
        };
        // push/unshift/splice take multiple args
        sig.params = vec![ParamEffect::Read; 3];
        array.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    // Non-mutating methods
    for method in &[
        "map",
        "filter",
        "reduce",
        "forEach",
        "find",
        "findIndex",
        "some",
        "every",
        "includes",
        "indexOf",
        "lastIndexOf",
        "flat",
        "flatMap",
        "slice",
        "concat",
        "join",
        "entries",
        "keys",
        "values",
        "at",
        "toString",
    ] {
        array.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    array
        .properties
        .insert("length".to_string(), PropertyShape { value_shape: ShapeId::NONE, writable: true });
    let array_shape = registry.register_shape(array);
    globals.insert("Array".to_string(), array_shape);

    // --- Object ---
    let mut object = ObjectShape::new();
    for method in &[
        "keys",
        "values",
        "entries",
        "assign",
        "freeze",
        "isFrozen",
        "create",
        "defineProperty",
        "defineProperties",
        "getOwnPropertyDescriptor",
        "getOwnPropertyNames",
        "getPrototypeOf",
        "is",
        "hasOwn",
    ] {
        object.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    let object_shape = registry.register_shape(object);
    globals.insert("Object".to_string(), object_shape);

    // --- Math ---
    let mut math = ObjectShape::new();
    for method in &[
        "abs", "ceil", "floor", "round", "max", "min", "pow", "sqrt", "log", "log2", "log10",
        "sin", "cos", "tan", "atan2", "sign", "trunc", "cbrt", "hypot", "fround", "clz32", "imul",
    ] {
        math.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    // Math.random is impure
    math.properties.insert(
        "random".to_string(),
        PropertyShape { value_shape: ShapeId::NONE, writable: false },
    );
    // Constants
    for constant in &["PI", "E", "LN2", "LN10", "LOG2E", "LOG10E", "SQRT2", "SQRT1_2"] {
        math.properties.insert(
            constant.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    let math_shape = registry.register_shape(math);
    globals.insert("Math".to_string(), math_shape);

    // --- JSON ---
    let mut json = ObjectShape::new();
    json.properties
        .insert("parse".to_string(), PropertyShape { value_shape: ShapeId::NONE, writable: false });
    json.properties.insert(
        "stringify".to_string(),
        PropertyShape { value_shape: ShapeId::NONE, writable: false },
    );
    let json_shape = registry.register_shape(json);
    globals.insert("JSON".to_string(), json_shape);

    // --- console ---
    let mut console = ObjectShape::new();
    for method in &[
        "log",
        "warn",
        "error",
        "info",
        "debug",
        "trace",
        "dir",
        "table",
        "time",
        "timeEnd",
        "timeLog",
        "assert",
        "count",
        "countReset",
        "group",
        "groupEnd",
        "groupCollapsed",
        "clear",
    ] {
        console.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    let console_shape = registry.register_shape(console);
    globals.insert("console".to_string(), console_shape);

    // --- String prototype ---
    let mut string = ObjectShape::new();
    for method in &[
        "charAt",
        "charCodeAt",
        "codePointAt",
        "concat",
        "endsWith",
        "includes",
        "indexOf",
        "lastIndexOf",
        "localeCompare",
        "match",
        "matchAll",
        "normalize",
        "padEnd",
        "padStart",
        "repeat",
        "replace",
        "replaceAll",
        "search",
        "slice",
        "split",
        "startsWith",
        "substring",
        "toLocaleLowerCase",
        "toLocaleUpperCase",
        "toLowerCase",
        "toUpperCase",
        "trim",
        "trimEnd",
        "trimStart",
        "at",
        "toString",
        "valueOf",
    ] {
        string.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    string.properties.insert(
        "length".to_string(),
        PropertyShape { value_shape: ShapeId::NONE, writable: false },
    );
    let string_shape = registry.register_shape(string);
    globals.insert("String".to_string(), string_shape);

    // --- Number ---
    let mut number = ObjectShape::new();
    for method in &[
        "toFixed",
        "toPrecision",
        "toExponential",
        "toString",
        "valueOf",
        "parseInt",
        "parseFloat",
        "isFinite",
        "isInteger",
        "isNaN",
        "isSafeInteger",
    ] {
        number.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    for constant in &[
        "MAX_VALUE",
        "MIN_VALUE",
        "MAX_SAFE_INTEGER",
        "MIN_SAFE_INTEGER",
        "POSITIVE_INFINITY",
        "NEGATIVE_INFINITY",
        "EPSILON",
        "NaN",
    ] {
        number.properties.insert(
            constant.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    let number_shape = registry.register_shape(number);
    globals.insert("Number".to_string(), number_shape);

    // --- Promise ---
    let mut promise = ObjectShape::new();
    for method in
        &["then", "catch", "finally", "all", "allSettled", "any", "race", "resolve", "reject"]
    {
        promise.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    let promise_shape = registry.register_shape(promise);
    globals.insert("Promise".to_string(), promise_shape);

    // --- React ---
    let mut react = ObjectShape::new();
    // React namespace methods
    for method in &[
        "createElement",
        "cloneElement",
        "createContext",
        "forwardRef",
        "memo",
        "lazy",
        "startTransition",
        "use",
    ] {
        react.properties.insert(
            method.to_string(),
            PropertyShape { value_shape: ShapeId::NONE, writable: false },
        );
    }
    // React.Children
    react.properties.insert(
        "Children".to_string(),
        PropertyShape { value_shape: ShapeId::NONE, writable: false },
    );
    let react_shape = registry.register_shape(react);
    globals.insert("React".to_string(), react_shape);

    // --- React hooks as standalone globals ---
    // (These are typically imported, but some setups have them as globals)
    for hook in &[
        "useState",
        "useReducer",
        "useRef",
        "useEffect",
        "useLayoutEffect",
        "useInsertionEffect",
        "useMemo",
        "useCallback",
        "useContext",
        "useTransition",
        "useDeferredValue",
        "useId",
        "useSyncExternalStore",
        "useImperativeHandle",
        "useDebugValue",
    ] {
        // Register a minimal shape for each hook
        let shape = ObjectShape::new();
        let id = registry.register_shape(shape);
        globals.insert(hook.to_string(), id);
    }

    // --- DOM globals (impure) ---
    for name in &[
        "document",
        "window",
        "navigator",
        "location",
        "history",
        "localStorage",
        "sessionStorage",
        "fetch",
        "XMLHttpRequest",
    ] {
        let shape = ObjectShape::new();
        let id = registry.register_shape(shape);
        globals.insert(name.to_string(), id);
    }

    // --- Other globals ---
    for name in &[
        "Date",
        "RegExp",
        "Map",
        "Set",
        "WeakMap",
        "WeakSet",
        "Symbol",
        "Proxy",
        "Reflect",
        "Intl",
        "Error",
        "TypeError",
        "RangeError",
        "globalThis",
        "undefined",
        "NaN",
        "Infinity",
        "parseInt",
        "parseFloat",
        "isNaN",
        "isFinite",
        "encodeURI",
        "decodeURI",
        "encodeURIComponent",
        "decodeURIComponent",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
        "queueMicrotask",
        "structuredClone",
        "atob",
        "btoa",
    ] {
        let shape = ObjectShape::new();
        let id = registry.register_shape(shape);
        globals.insert(name.to_string(), id);
    }

    // Suppress unused variable warnings
    let _ = &pure_fn;
    let _ = &read_fn;
    let _ = &hook_fn;
    let _ = &impure_fn;

    GlobalRegistry {
        array_shape,
        object_shape,
        math_shape,
        json_shape,
        console_shape,
        string_shape,
        number_shape,
        promise_shape,
        react_shape,
        globals,
    }
}

/// Check if a name is a known React hook.
pub fn is_hook_name(name: &str) -> bool {
    // Convention: hooks start with "use" followed by an uppercase letter
    if name.len() < 4 {
        return false;
    }
    name.starts_with("use") && name.as_bytes().get(3).is_some_and(u8::is_ascii_uppercase)
}

/// Check if a name is a known React component (PascalCase).
pub fn is_component_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}
