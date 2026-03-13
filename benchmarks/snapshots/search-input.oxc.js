import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';

interface SearchInputProps {
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
}

export function SearchInput(t0) {
  const $ = _c(11);
  const { onSearch, placeholder, debounceMs } = t0;
  const timerRef = useRef(null);
  let handleChange;
  if ($[0] !== useEffect) {
    const t97 = () => {
      const t0 = () => {
        if (timerRef.current) {
          const t7 = clearTimeout(timerRef.current);
        }
        return undefined;
      };
      return t0;
    };
    const t99 = useEffect(t97, []);
    $[0] = useEffect;
  }
  let handleClear;
  if ($[1] !== debounceMs || $[2] !== onSearch || $[3] !== useCallback) {
    const t102 = (e) => {
      const newVal = e.target.value;
      const t12 = setValue(newVal);
      if (timerRef.current) {
        const t20 = clearTimeout(timerRef.current);
      }
      const t22 = () => {
        const t4 = onSearch(newVal);
        return undefined;
      };
      timerRef.current = setTimeout(t22, debounceMs);
      return undefined;
    };
    handleChange = useCallback(t102, [onSearch, debounceMs]);
    $[1] = debounceMs;
    $[2] = onSearch;
    $[3] = useCallback;
  }
  const t110 = () => {
    const t3 = setValue("");
    const t7 = onSearch("");
    if (timerRef.current) {
      const t15 = clearTimeout(timerRef.current);
    }
    return undefined;
  };
  handleClear = useCallback(t110, [onSearch]);
  let t139;
  if ($[4] !== handleChange || $[5] !== onSearch || $[6] !== placeholder || $[7] !== useCallback || $[8] !== value || $[9] !== value) {
    t67 = value;
    $[4] = handleChange;
    $[5] = onSearch;
    $[6] = placeholder;
    $[7] = useCallback;
    $[8] = value;
    $[9] = value;
    $[10] = t139;
  } else {
    t139 = $[10];
  }
  t67 = <button onClick={handleClear} className="absolute right-2 top-2 text-gray-400">\n          ×\n        </button>;
  return <div className="relative"><input type="text" value={value} onChange={handleChange} placeholder={placeholder} className="w-full px-3 py-2 border rounded" />{t67}</div>;
}

