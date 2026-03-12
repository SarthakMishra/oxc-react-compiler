import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';

export function ThemeToggle() {
  const $ = _c(11);
  const t38 = useState;
  const t39 = "light";
  const t40 = t38(t39);
  let theme;
  let setTheme;
  if ($[0] !== theme || $[1] !== setTheme) {
    $[0] = theme;
    $[1] = setTheme;
  } else {
  }
  ([theme, setTheme] = t40);
  let toggle;
  if ($[2] !== toggle) {
    $[2] = toggle;
  } else {
  }
  const t45 = useCallback;
  const t46 = () => {
    const t1 = setTheme;
    const t2 = (t) => {
      const t2 = t;
      const t3 = "light";
      const t4 = t2 === t3;
      if (t4) {
        const t5 = "dark";
      } else {
        const t6 = "light";
      }
      return t7;
    };
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  const t47 = [];
  const t48 = t45(t46, t47);
  toggle = t48;
  const t50 = "button";
  const t51 = toggle;
  const t52 = theme;
  const t53 = "dark";
  const t54 = t52 === t53;
  if (t54) {
  } else {
  }
  if ($[3] !== theme || $[4] !== t58) {
    const t57 = theme;
    const t58 = "dark";
    const t59 = t57 === t58;
    $[3] = theme;
    $[4] = t58;
  } else {
  }
  if (t59) {
  } else {
  }
  let t63;
  if ($[5] !== toggle || $[6] !== theme || $[7] !== t53 || $[8] !== t28 || $[9] !== t35) {
    t63 = _jsx(t50, { onClick: t51, className: t28, children: t35 });
    $[10] = t63;
    $[5] = toggle;
    $[6] = theme;
    $[7] = t53;
    $[8] = t28;
    $[9] = t35;
  } else {
    t63 = $[10];
  }
  return t63;
}

