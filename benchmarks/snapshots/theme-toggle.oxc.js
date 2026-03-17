import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';

export function ThemeToggle() {
  const $ = _c(8);
  let t2;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t2;
  } else {
    t2 = $[0];
  }
  let t30;
  let t29;
  let t11;
  let t12;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t11 = () => {
      const t2 = (t) => {
        let t4;
        if (t === "light") {
          t4 = "dark";
        } else {
          t4 = "light";
        }
        return t4;
      };
      const t3 = setTheme(t2);
      return undefined;
    };
    t12 = [];
    $[1] = t29;
    $[2] = t30;
    $[3] = t11;
    $[4] = t12;
  } else {
    t29 = $[1];
    t30 = $[2];
    t11 = $[3];
    t12 = $[4];
  }
  const toggle = t30;
  const t13 = useCallback(t11, t12);
  let t31;
  let t15;
  if ($[5] !== t13) {
    t31 = t13;
    t15 = "button";
    $[5] = t13;
    $[6] = t31;
    $[7] = t15;
  } else {
    t31 = $[6];
    t15 = $[7];
  }
  const toggle = t31;
  let t20;
  if (theme === "dark") {
    t20 = "bg-gray-800 text-white";
  } else {
    t20 = "bg-white text-black";
  }
  let t26;
  if (theme === "dark") {
    t26 = "☀️";
  } else {
    t26 = "🌙";
  }
  return <t15 onClick={toggle} className={t20}>{t26}</t15>;
}

