import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';

export function ThemeToggle() {
  const $ = _c(6);
  let t2;
  let t30;
  let t15;
  let t29;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = "light";
    $[0] = t2;
  } else {
    t2 = $[0];
  }
  let theme;
  let setTheme;
  ([theme, setTheme] = useState(t2));
  let toggle;
  let t11 = () => {
    let t2 = (t) => {
      let t4;
      if (t === "light") {
        t4 = "dark";
      } else {
        t4 = "light";
      }
      return t4;
    };
    let t3 = setTheme(t2);
    return undefined;
  };
  let t13 = useCallback(t11, []);
  if ($[1] !== t13) {
    t30 = t13;
    t15 = "button";
    $[1] = t13;
    $[2] = t30;
    $[3] = t15;
  } else {
    t30 = $[2];
    t15 = $[3];
  }
  toggle = t30;
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
  if ($[4] !== t13) {
    t29 = <button onClick={toggle} className={t20}>{t26}</button>;
    $[4] = t13;
    $[5] = t29;
  } else {
    t29 = $[5];
  }
  return t29;
}

