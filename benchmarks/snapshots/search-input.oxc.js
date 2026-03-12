import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';

interface SearchInputProps {
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
}

export function SearchInput(t0) {
  const $ = _c(4);
  const t71 = useState;
  const t72 = "";
  const t73 = t71(t72);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t74 = Discriminant(4) */
    /* t75 = Discriminant(4) */
  } else {
  }
  /* t76 = Discriminant(6) */
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t77 = Discriminant(4) */
  } else {
  }
  const t78 = useRef;
  const t79 = null;
  const t80 = t78(t79);
  const timerRef = t80;
  const t82 = useEffect;
  /* t83 = Discriminant(28) */
  const t84 = [];
  const t85 = t82(t83, t84);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t86 = Discriminant(4) */
  } else {
  }
  const t87 = useCallback;
  /* t88 = Discriminant(28) */
  const t89 = onSearch;
  const t90 = debounceMs;
  const t91 = [t89, t90];
  const t92 = t87(t88, t91);
  const handleChange = t92;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t94 = Discriminant(4) */
  } else {
  }
  const t95 = useCallback;
  /* t96 = Discriminant(28) */
  const t97 = onSearch;
  const t98 = [t97];
  const t99 = t95(t96, t98);
  const handleClear = t99;
  const t101 = "div";
  const t102 = "relative";
  const t103 = "input";
  const t104 = "text";
  const t105 = value;
  const t106 = handleChange;
  const t107 = placeholder;
  const t108 = "w-full px-3 py-2 border rounded";
  const t109 = <t103 type={t104} value={t105} onChange={t106} placeholder={t107} className={t108} />;
}

