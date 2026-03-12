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
  const $ = _c(22);
  const { onSearch, placeholder, debounceMs } = t0;
  if ($[0] !== onSearch || $[1] !== placeholder || $[2] !== debounceMs) {
    $[0] = onSearch;
    $[1] = placeholder;
    $[2] = debounceMs;
  } else {
  }
  const t85 = useState;
  const t86 = "";
  const t87 = t85(t86);
  let value;
  let setValue;
  if ($[3] !== value || $[4] !== setValue) {
    $[3] = value;
    $[4] = setValue;
  } else {
  }
  ([value, setValue] = t87);
  let timerRef;
  if ($[5] !== timerRef) {
    $[5] = timerRef;
  } else {
  }
  const t92 = useRef;
  const t93 = null;
  const t94 = t92(t93);
  timerRef = t94;
  const t96 = useEffect;
  const t97 = () => {
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
  const t98 = [];
  const t99 = t96(t97, t98);
  let handleChange;
  if ($[6] !== handleChange) {
    $[6] = handleChange;
  } else {
  }
  let handleClear;
  if ($[7] !== useCallback || $[8] !== onSearch || $[9] !== debounceMs || $[10] !== handleChange || $[11] !== handleClear) {
    const t101 = useCallback;
    const t102 = (e) => {
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
    const t103 = onSearch;
    const t104 = debounceMs;
    const t105 = [t103, t104];
    const t106 = t101(t102, t105);
    handleChange = t106;
    $[7] = useCallback;
    $[8] = onSearch;
    $[9] = debounceMs;
    $[10] = handleChange;
    $[11] = handleClear;
  } else {
  }
  const t109 = useCallback;
  const t110 = () => {
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
  const t111 = onSearch;
  const t112 = [t111];
  const t113 = t109(t110, t112);
  handleClear = t113;
  const t115 = "div";
  const t116 = "relative";
  const t117 = "input";
  const t118 = "text";
  const t119 = value;
  const t120 = handleChange;
  const t121 = placeholder;
  const t122 = "w-full px-3 py-2 border rounded";
  const t123 = _jsx(t117, { type: t118, value: t119, onChange: t120, placeholder: t121, className: t122 });
  let t67;
  let t139;
  if ($[12] !== handleClear || $[13] !== t67 || $[14] !== useCallback || $[15] !== onSearch || $[16] !== handleClear || $[17] !== value || $[18] !== handleChange || $[19] !== placeholder || $[20] !== value) {
    const t125 = value;
    t67 = t125;
    $[21] = t139;
    $[12] = handleClear;
    $[13] = t67;
    $[14] = useCallback;
    $[15] = onSearch;
    $[16] = handleClear;
    $[17] = value;
    $[18] = handleChange;
    $[19] = placeholder;
    $[20] = value;
  } else {
    t139 = $[21];
  }
  const t127 = "button";
  const t128 = handleClear;
  const t129 = "absolute right-2 top-2 text-gray-400";
  const t130 = "\n          ×\n        ";
  const t131 = _jsx(t127, { onClick: t128, className: t129, children: t130 });
  t67 = t131;
  t139 = _jsxs(t115, { className: t116, children: [t123, t67] });
  return t139;
}

