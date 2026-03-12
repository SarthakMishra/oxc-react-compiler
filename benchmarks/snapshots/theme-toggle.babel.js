import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';
import { jsx as _jsx } from "react/jsx-runtime";
export function ThemeToggle() {
  const $ = _c(4);
  const [theme, setTheme] = useState("light");
  let t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t0 = () => {
      setTheme(_temp);
    };
    $[0] = t0;
  } else {
    t0 = $[0];
  }
  const toggle = t0;
  const t1 = theme === "dark" ? "bg-gray-800 text-white" : "bg-white text-black";
  const t2 = theme === "dark" ? "\u2600\uFE0F" : "\uD83C\uDF19";
  let t3;
  if ($[1] !== t1 || $[2] !== t2) {
    t3 = /*#__PURE__*/_jsx("button", {
      onClick: toggle,
      className: t1,
      children: t2
    });
    $[1] = t1;
    $[2] = t2;
    $[3] = t3;
  } else {
    t3 = $[3];
  }
  return t3;
}
function _temp(t) {
  return t === "light" ? "dark" : "light";
}