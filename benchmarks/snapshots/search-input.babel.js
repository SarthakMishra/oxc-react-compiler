import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function SearchInput(t0) {
  const $ = _c(17);
  const {
    onSearch,
    placeholder: t1,
    debounceMs: t2
  } = t0;
  const placeholder = t1 === undefined ? "Search..." : t1;
  const debounceMs = t2 === undefined ? 300 : t2;
  const [value, setValue] = useState("");
  const timerRef = useRef(null);
  let t3;
  let t4;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t3 = () => () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
    t4 = [];
    $[0] = t3;
    $[1] = t4;
  } else {
    t3 = $[0];
    t4 = $[1];
  }
  useEffect(t3, t4);
  let t5;
  if ($[2] !== debounceMs || $[3] !== onSearch) {
    t5 = e => {
      const newVal = e.target.value;
      setValue(newVal);
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      timerRef.current = setTimeout(() => {
        onSearch(newVal);
      }, debounceMs);
    };
    $[2] = debounceMs;
    $[3] = onSearch;
    $[4] = t5;
  } else {
    t5 = $[4];
  }
  const handleChange = t5;
  let t6;
  if ($[5] !== onSearch) {
    t6 = () => {
      setValue("");
      onSearch("");
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
    $[5] = onSearch;
    $[6] = t6;
  } else {
    t6 = $[6];
  }
  const handleClear = t6;
  let t7;
  if ($[7] !== handleChange || $[8] !== placeholder || $[9] !== value) {
    t7 = /*#__PURE__*/_jsx("input", {
      type: "text",
      value: value,
      onChange: handleChange,
      placeholder: placeholder,
      className: "w-full px-3 py-2 border rounded"
    });
    $[7] = handleChange;
    $[8] = placeholder;
    $[9] = value;
    $[10] = t7;
  } else {
    t7 = $[10];
  }
  let t8;
  if ($[11] !== handleClear || $[12] !== value) {
    t8 = value && /*#__PURE__*/_jsx("button", {
      onClick: handleClear,
      className: "absolute right-2 top-2 text-gray-400",
      children: "\xD7"
    });
    $[11] = handleClear;
    $[12] = value;
    $[13] = t8;
  } else {
    t8 = $[13];
  }
  let t9;
  if ($[14] !== t7 || $[15] !== t8) {
    t9 = /*#__PURE__*/_jsxs("div", {
      className: "relative",
      children: [t7, t8]
    });
    $[14] = t7;
    $[15] = t8;
    $[16] = t9;
  } else {
    t9 = $[16];
  }
  return t9;
}