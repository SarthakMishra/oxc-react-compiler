import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';

interface SearchInputProps {
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
}

export function SearchInput(t0) {
  const $ = _c(33);
  const { onSearch, placeholder, debounceMs } = t0;
  let t7;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t7;
  } else {
    t7 = $[0];
  }
  let t57;
  let t15;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t15 = null;
    $[1] = t57;
    $[2] = t15;
  } else {
    t57 = $[1];
    t15 = $[2];
  }
  const timerRef = t57;
  const t16 = useRef(t15);
  let t58;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    t58 = t16;
    $[3] = t58;
  } else {
    t58 = $[3];
  }
  const timerRef = t58;
  let t19;
  let t20;
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t19 = () => {
      const t0 = () => {
        if (timerRef.current) {
          const t6 = clearTimeout(timerRef.current);
        }
        return undefined;
      };
      return t0;
    };
    t20 = [];
    $[4] = t19;
    $[5] = t20;
  } else {
    t19 = $[4];
    t20 = $[5];
  }
  const t21 = useEffect(t19, t20);
  let handleChange;
  const t26 = (e) => {
    let newVal;
    newVal = e.target.value;
    const t8 = setValue(newVal);
    if (timerRef.current) {
      const t15 = clearTimeout(timerRef.current);
    }
    const t17 = () => {
      const t4 = onSearch(newVal);
      return undefined;
    };
    timerRef.current = setTimeout(t17, debounceMs);
    return undefined;
  };
  const t30 = useCallback(t26, [onSearch, debounceMs]);
  let t60;
  let t59;
  let t35;
  let t37;
  if ($[6] !== t30 || $[7] !== onSearch) {
    t59 = t30;
    t35 = () => {
      const t3 = setValue("");
      const t7 = onSearch("");
      if (timerRef.current) {
        const t14 = clearTimeout(timerRef.current);
      }
      return undefined;
    };
    t37 = [onSearch];
    $[6] = t30;
    $[7] = onSearch;
    $[8] = t59;
    $[9] = t60;
    $[10] = t35;
    $[11] = t37;
  } else {
    t59 = $[8];
    t60 = $[9];
    t35 = $[10];
    t37 = $[11];
  }
  handleChange = t59;
  const handleClear = t60;
  const t38 = useCallback(t35, t37);
  let t49;
  let t61;
  let t40;
  let t41;
  let t48;
  if ($[12] !== t38 || $[13] !== debounceMs || $[14] !== handleClear || $[15] !== onSearch || $[16] !== placeholder) {
    t61 = t38;
    t40 = "div";
    t41 = "relative";
    t48 = <input type="text" value={value} onChange={handleChange} placeholder={placeholder} className="w-full px-3 py-2 border rounded" />;
    $[12] = t38;
    $[13] = debounceMs;
    $[14] = handleClear;
    $[15] = onSearch;
    $[16] = placeholder;
    $[17] = t61;
    $[18] = t40;
    $[19] = t41;
    $[20] = t48;
    $[21] = t49;
  } else {
    t61 = $[17];
    t40 = $[18];
    t41 = $[19];
    t48 = $[20];
    t49 = $[21];
  }
  const handleClear = t61;
  let t56;
  let t62;
  let t26;
  let t29;
  let value;
  if ($[22] !== t38 || $[23] !== debounceMs || $[24] !== handleChange || $[25] !== onSearch || $[26] !== placeholder) {
    t49 = value;
    $[22] = t38;
    $[23] = debounceMs;
    $[24] = handleChange;
    $[25] = onSearch;
    $[26] = placeholder;
    $[27] = t49;
    $[28] = t56;
    $[29] = t62;
    $[30] = t26;
    $[31] = t29;
    $[32] = value;
  } else {
    t49 = $[27];
    t56 = $[28];
    t62 = $[29];
    t26 = $[30];
    t29 = $[31];
    value = $[32];
  }
  handleChange = t62;
  t49 = <button onClick={handleClear} className="absolute right-2 top-2 text-gray-400">\n          ×\n        </button>;
  return <t40 className={t41}>{t48}{t49}</t40>;
}

