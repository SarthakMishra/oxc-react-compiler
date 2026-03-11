#![allow(dead_code)]

use crate::hir::types::HIR;

/// Optimize method calls on props to avoid unnecessary memoization.
///
/// Detects `props.onClick()` patterns and converts:
///   `MethodCall { receiver: props, property: "onClick", args }`
/// into:
///   `PropertyLoad { object: props, property: "onClick" }` (new instruction)
///   `CallExpression { callee: <loaded_prop>, args }`
///
/// This allows the property load and the call to be in different reactive
/// scopes, improving memoization granularity. A method call on props forces
/// the entire call into a single scope, but splitting it lets the property
/// load be shared.
///
/// Requirements:
/// - The receiver must be typed as a component's `props` parameter
/// - SSA and type inference must have run before this pass
///
/// For now, this is a no-op pass. Full implementation requires integration
/// with the type inference pass to identify which identifiers refer to props.
pub fn optimize_props_method_calls(hir: &mut HIR) {
    // To implement this fully, we need:
    // 1. Type information to identify which places are "props"
    // 2. An IdGenerator to create new instruction IDs and identifier IDs
    //    for the intermediate PropertyLoad
    // 3. Careful insertion of the new instruction before the transformed call
    //
    // Sketch of the algorithm:
    //
    // for each block:
    //   for each instruction:
    //     if MethodCall { receiver, property, args } where receiver is props:
    //       - create new identifier for the property load result
    //       - create PropertyLoad { object: receiver, property }
    //       - replace MethodCall with CallExpression { callee: new_id, args }
    //       - insert PropertyLoad before the CallExpression
    let _ = hir;
}
