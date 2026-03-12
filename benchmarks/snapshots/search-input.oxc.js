import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';

interface SearchInputProps {
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
}

export function SearchInput(t0) {
  const $ = _c(21);
  const { onSearch, placeholder, debounceMs } = t0;
  if ($[0] !== onSearch || $[1] !== placeholder || $[2] !== debounceMs) {
    $[0] = onSearch;
    $[1] = placeholder;
    $[2] = debounceMs;
  } else {
  }
  const t82 = useState;
  const t83 = "";
  const t84 = t82(t83);
  let value;
  let setValue;
  if ($[3] !== value || $[4] !== setValue) {
    $[3] = value;
    $[4] = setValue;
  } else {
  }
  ([value, setValue] = t84);
  let timerRef;
  if ($[5] !== timerRef) {
    $[5] = timerRef;
  } else {
  }
  const t89 = useRef;
  const t90 = null;
  const t91 = t89(t90);
  timerRef = t91;
  const t93 = useEffect;
  const t94 = () => {
    const t0 = () => {
      const t1 = timerRef;
      const t2 = t1.current;
      if (t2) {
        const t3 = clearTimeout;
        const t5 = timerRef;
        const t6 = t5.current;
        const t7 = t3(t6);
      } else {
      }
      const t8 = undefined;
      return t8;
    };
    return t0;
  };
  const t95 = [];
  const t96 = t93(t94, t95);
  let handleChange;
  if ($[6] !== handleChange) {
    $[6] = handleChange;
  } else {
  }
  let handleClear;
  if ($[7] !== useCallback || $[8] !== onSearch || $[9] !== debounceMs || $[10] !== handleChange || $[11] !== handleClear) {
    const t98 = useCallback;
    const t99 = (e) => {
      let newVal;
      const t4 = e;
      const t5 = t4.target;
      const t6 = t5.value;
      newVal = t6;
      const t9 = setValue;
      const t11 = newVal;
      const t12 = t9(t11);
      const t14 = timerRef;
      const t15 = t14.current;
      if (t15) {
        const t16 = clearTimeout;
        const t18 = timerRef;
        const t19 = t18.current;
        const t20 = t16(t19);
      } else {
      }
      const t21 = setTimeout;
      const t22 = () => {
        const t1 = onSearch;
        const t3 = newVal;
        const t4 = t1(t3);
        const t5 = undefined;
        return t5;
      };
      const t24 = debounceMs;
      const t25 = t21(t22, t24);
      const t27 = timerRef;
      t27.current = t25;
      const t29 = undefined;
      return t29;
    };
    const t100 = onSearch;
    const t101 = debounceMs;
    const t102 = [t100, t101];
    const t103 = t98(t99, t102);
    handleChange = t103;
    $[7] = useCallback;
    $[8] = onSearch;
    $[9] = debounceMs;
    $[10] = handleChange;
    $[11] = handleClear;
  } else {
  }
  const t106 = useCallback;
  const t107 = () => {
    const t1 = setValue;
    const t2 = "";
    const t3 = t1(t2);
    const t5 = onSearch;
    const t6 = "";
    const t7 = t5(t6);
    const t9 = timerRef;
    const t10 = t9.current;
    if (t10) {
      const t11 = clearTimeout;
      const t13 = timerRef;
      const t14 = t13.current;
      const t15 = t11(t14);
    } else {
    }
    const t16 = undefined;
    return t16;
  };
  const t108 = onSearch;
  const t109 = [t108];
  const t110 = t106(t107, t109);
  handleClear = t110;
  const t112 = "div";
  const t113 = "relative";
  const t114 = "input";
  const t115 = "text";
  const t116 = value;
  const t117 = handleChange;
  const t118 = placeholder;
  const t119 = "w-full px-3 py-2 border rounded";
  const t120 = _jsx(t114, { type: t115, value: t116, onChange: t117, placeholder: t118, className: t119 });
  const t122 = "button";
  if ($[12] !== handleClear) {
    const t123 = handleClear;
    $[12] = handleClear;
  } else {
  }
  const t124 = "absolute right-2 top-2 text-gray-400";
  const t125 = "\n          ×\n        ";
  let t132;
  if ($[13] !== useCallback || $[14] !== onSearch || $[15] !== handleClear || $[16] !== value || $[17] !== handleChange || $[18] !== placeholder || $[19] !== t75) {
    t132 = _jsxs(t112, { className: t113, children: [t120, t75] });
    $[20] = t132;
    $[13] = useCallback;
    $[14] = onSearch;
    $[15] = handleClear;
    $[16] = value;
    $[17] = handleChange;
    $[18] = placeholder;
    $[19] = t75;
  } else {
    t132 = $[20];
  }
  return t132;
}

