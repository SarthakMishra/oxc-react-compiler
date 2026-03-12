import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';

export function ThemeToggle() {
  const $ = _c(11);
  const t44 = useState;
  const t45 = "light";
  const t46 = t44(t45);
  let theme;
  let setTheme;
  if ($[0] !== theme || $[1] !== setTheme) {
    $[0] = theme;
    $[1] = setTheme;
  } else {
  }
  ([theme, setTheme] = t46);
  let toggle;
  if ($[2] !== toggle) {
    $[2] = toggle;
  } else {
  }
  const t51 = useCallback;
  const t52 = () => {
    const t1 = setTheme;
    const t2 = (t) => {
      const t2 = t;
      const t3 = "light";
      const t4 = t2 === t3;
      let t5;
      if (t4) {
        const t7 = "dark";
        t5 = t7;
      } else {
        const t9 = "light";
        t5 = t9;
      }
      return t5;
    };
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  const t53 = [];
  const t54 = t51(t52, t53);
  toggle = t54;
  const t56 = "button";
  const t57 = toggle;
  const t58 = theme;
  const t59 = "dark";
  const t60 = t58 === t59;
  let t26;
  if (t60) {
    const t79 = "bg-gray-800 text-white";
    t26 = t79;
  } else {
    const t81 = "bg-white text-black";
    t26 = t81;
  }
  if ($[3] !== theme || $[4] !== t67) {
    const t66 = theme;
    const t67 = "dark";
    const t68 = t66 === t67;
    $[3] = theme;
    $[4] = t67;
  } else {
  }
  let t36;
  if (t68) {
    const t77 = "☀️";
    t36 = t77;
  } else {
    const t70 = "🌙";
    t36 = t70;
  }
  let t76;
  if ($[5] !== toggle || $[6] !== theme || $[7] !== t59 || $[8] !== t26 || $[9] !== t36) {
    t76 = _jsx(t56, { onClick: t57, className: t26, children: t36 });
    $[10] = t76;
    $[5] = toggle;
    $[6] = theme;
    $[7] = t59;
    $[8] = t26;
    $[9] = t36;
  } else {
    t76 = $[10];
  }
  return t76;
}

