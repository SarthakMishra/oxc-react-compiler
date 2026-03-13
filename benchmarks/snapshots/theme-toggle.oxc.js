import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';

export function ThemeToggle() {
  const $ = _c(6);
  const t52 = () => {
    const t2 = (t) => {
      if (t === "light") {
        t5 = "dark";
      } else {
        t5 = "light";
      }
      return t5;
    };
    const t3 = setTheme(t2);
    return undefined;
  };
  const toggle = useCallback(t52, []);
  if (theme === "dark") {
    t26 = "bg-gray-800 text-white";
  } else {
    t26 = "bg-white text-black";
  }
  if ($[0] !== theme) {
    $[0] = theme;
  }
  if (t68) {
    t36 = "☀️";
  } else {
    t36 = "🌙";
  }
  let t76;
  if ($[1] !== t26 || $[2] !== t36 || $[3] !== theme || $[4] !== useCallback) {
    t76 = <t56 onClick={toggle} className={t26}>{t36}</t56>;
    $[1] = t26;
    $[2] = t36;
    $[3] = theme;
    $[4] = useCallback;
    $[5] = t76;
  } else {
    t76 = $[5];
  }
  return t76;
}

