#![allow(dead_code)]
//! Shared hook identification utilities.

use oxc_ast::ast::{CallExpression, Expression};

/// Returns `true` if `name` looks like a React hook (starts with "use" followed by an uppercase
/// letter or is exactly "use").
pub fn is_hook_name(name: &str) -> bool {
    if name == "use" {
        return true;
    }
    if let Some(rest) = name.strip_prefix("use") {
        rest.starts_with(|c: char| c.is_ascii_uppercase())
    } else {
        false
    }
}

/// Returns `true` if `name` looks like a React component (starts with an uppercase letter).
pub fn is_component_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Extract the callee name from a `CallExpression`.
///
/// Handles:
/// - `useEffect(...)` → `Some("useEffect")`
/// - `React.useEffect(...)` → `Some("useEffect")`
/// - Anything else → `None`
pub fn get_callee_name<'a>(call: &'a CallExpression<'a>) -> Option<&'a str> {
    match &call.callee {
        Expression::Identifier(ident) => Some(ident.name.as_str()),
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

/// Returns `true` if the call expression is calling a React hook.
pub fn is_hook_call<'a>(call: &'a CallExpression<'a>) -> bool {
    get_callee_name(call).is_some_and(is_hook_name)
}

/// Returns `true` if the call expression looks like a `setState` call.
///
/// Matches names that start with "set" followed by an uppercase letter, which is the
/// conventional naming for state setters returned by `useState`.
/// e.g. `setCount`, `setValue`, `setIsOpen`
pub fn is_set_state_call<'a>(call: &'a CallExpression<'a>) -> bool {
    get_callee_name(call).is_some_and(is_set_state_name)
}

/// Returns `true` if `name` looks like a setState function (starts with "set" + uppercase).
pub fn is_set_state_name(name: &str) -> bool {
    if let Some(rest) = name.strip_prefix("set") {
        rest.starts_with(|c: char| c.is_ascii_uppercase())
    } else {
        false
    }
}

/// Returns `true` if the call expression is calling an effect hook
/// (`useEffect`, `useLayoutEffect`, `useInsertionEffect`).
pub fn is_effect_hook_call<'a>(call: &'a CallExpression<'a>) -> bool {
    get_callee_name(call)
        .is_some_and(|n| n == "useEffect" || n == "useLayoutEffect" || n == "useInsertionEffect")
}

/// Returns `true` if the call expression is `useRef(...)`.
pub fn is_use_ref_call<'a>(call: &'a CallExpression<'a>) -> bool {
    get_callee_name(call).is_some_and(|n| n == "useRef")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hook_name() {
        assert!(is_hook_name("useState"));
        assert!(is_hook_name("useEffect"));
        assert!(is_hook_name("useMyCustomHook"));
        assert!(is_hook_name("use"));
        assert!(!is_hook_name("useless"));
        assert!(!is_hook_name("notAHook"));
        assert!(!is_hook_name(""));
    }

    #[test]
    fn test_is_component_name() {
        assert!(is_component_name("MyComponent"));
        assert!(is_component_name("App"));
        assert!(!is_component_name("myComponent"));
        assert!(!is_component_name("app"));
        assert!(!is_component_name(""));
    }

    #[test]
    fn test_is_set_state_name() {
        assert!(is_set_state_name("setCount"));
        assert!(is_set_state_name("setValue"));
        assert!(is_set_state_name("setIsOpen"));
        assert!(!is_set_state_name("set"));
        assert!(!is_set_state_name("setup"));
        assert!(!is_set_state_name("setting"));
        assert!(!is_set_state_name("notSetState"));
    }
}
