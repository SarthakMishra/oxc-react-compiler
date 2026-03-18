import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';

interface SearchInputProps {
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
}

export function SearchInput(t0) {
  const $ = _c(21);
  let t7;
  let t57;
  let t15;
  let t58;
  let t19;
  let t20;
  let t59;
  let t60;
  let t35;
  let t37;
  let t61;
  let t40;
  let t41;
  let t48;
  let t49;
  let { onSearch, placeholder, debounceMs } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t7 = "";
    $[0] = t7;
  } else {
    t7 = $[0];
  }
  let value;
  let setValue;
  ([value, setValue] = useState(t7));
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t15 = null;
    $[1] = t57;
    $[2] = t15;
  } else {
    t57 = $[1];
    t15 = $[2];
  }
  let timerRef = t57;
  let t16 = useRef(t15);
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    t58 = t16;
    $[3] = t58;
  } else {
    t58 = $[3];
  }
  timerRef = t58;
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t19 = () => {
      let t0 = () => {
        if (timerRef.current) {
          let t6 = clearTimeout(timerRef.current);
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
  let t21 = useEffect(t19, t20);
  let handleChange;
  let t26 = (e) => {
    let newVal;
    newVal = e.target.value;
    let t8 = setValue(newVal);
    if (timerRef.current) {
      let t15 = clearTimeout(timerRef.current);
    }
    let t17 = () => {
      let t4 = onSearch(newVal);
      return undefined;
    };
    timerRef.current = setTimeout(t17, debounceMs);
    return undefined;
  };
  let t30 = useCallback(t26, [onSearch, debounceMs]);
  if ($[6] !== t30 || $[7] !== onSearch) {
    t59 = t30;
    t35 = () => {
      let t3 = setValue("");
      let t7 = onSearch("");
      if (timerRef.current) {
        let t14 = clearTimeout(timerRef.current);
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
  let handleClear = t60;
  let t38 = useCallback(t35, t37);
  if ($[12] !== t38 || $[13] !== onSearch || $[14] !== placeholder) {
    t61 = t38;
    t40 = "div";
    t41 = "relative";
    t48 = <input type="text" value={value} onChange={handleChange} placeholder={placeholder} className="w-full px-3 py-2 border rounded" />;
    $[12] = t38;
    $[13] = onSearch;
    $[14] = placeholder;
    $[15] = t61;
    $[16] = t40;
    $[17] = t41;
    $[18] = t48;
    $[19] = t49;
  } else {
    t61 = $[15];
    t40 = $[16];
    t41 = $[17];
    t48 = $[18];
    t49 = $[19];
  }
  handleClear = t61;
  if ($[20] === Symbol.for("react.memo_cache_sentinel")) {
    t49 = value;
    t49 = <button onClick={handleClear} className="absolute right-2 top-2 text-gray-400">\n          ×\n        </button>;
    return <div className={t41}>{t48}{t49}</div>;
    $[20] = t49;
  } else {
    t49 = $[20];
  }
}

