import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by shadcn/ui command/combobox component
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';

interface CommandItem {
  id: string;
  label: string;
  description?: string;
  group?: string;
  shortcut?: string;
  onSelect: () => void;
}

interface CommandMenuProps {
  items: CommandItem[];
  placeholder?: string;
  emptyMessage?: string;
  onOpenChange?: (open: boolean) => void;
}

export function CommandMenu(t0) {
  const $ = _c(11);
  const t159 = useState;
  const t160 = "";
  const t161 = t159(t160);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t162 = Discriminant(4) */
    /* t163 = Discriminant(4) */
  } else {
  }
  /* t164 = Discriminant(6) */
  const t165 = useState;
  const t166 = 0;
  const t167 = t165(t166);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t168 = Discriminant(4) */
    /* t169 = Discriminant(4) */
  } else {
  }
  /* t170 = Discriminant(6) */
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t171 = Discriminant(4) */
  } else {
  }
  const t172 = useRef;
  const t173 = null;
  const t174 = t172(t173);
  const inputRef = t174;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t176 = Discriminant(4) */
  } else {
  }
  const t177 = useRef;
  const t178 = null;
  const t179 = t177(t178);
  const listRef = t179;
  const t181 = useEffect;
  /* t182 = Discriminant(28) */
  const t183 = [];
  const t184 = t181(t182, t183);
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    /* t185 = Discriminant(4) */
  } else {
  }
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    const t186 = useMemo;
    /* t187 = Discriminant(28) */
    const t188 = items;
    const t189 = query;
    const t190 = [t188, t189];
    const t191 = t186(t187, t190);
    const filteredItems = t191;
    /* t193 = Discriminant(4) */
  } else {
  }
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    const t194 = useMemo;
    /* t195 = Discriminant(28) */
    const t196 = filteredItems;
    const t197 = [t196];
    const t198 = t194(t195, t197);
    const groupedItems = t198;
    /* t200 = Discriminant(4) */
  } else {
  }
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    const t201 = useMemo;
    /* t202 = Discriminant(28) */
    const t203 = groupedItems;
    const t204 = [t203];
    const t205 = t201(t202, t204);
    const flatList = t205;
    /* t207 = Discriminant(4) */
  } else {
  }
  const t208 = useCallback;
  /* t209 = Discriminant(28) */
  const t210 = onOpenChange;
  const t211 = [t210];
  const t212 = t208(t209, t211);
  const handleSelect = t212;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    /* t214 = Discriminant(4) */
  } else {
  }
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    const t215 = useCallback;
    /* t216 = Discriminant(28) */
    const t217 = flatList;
    const t218 = activeIndex;
    const t219 = handleSelect;
    const t220 = onOpenChange;
    const t221 = [t217, t218, t219, t220];
    const t222 = t215(t216, t221);
    const handleKeyDown = t222;
  } else {
  }
  const t224 = useEffect;
  /* t225 = Discriminant(28) */
  if ($[10] === Symbol.for("react.memo_cache_sentinel")) {
    const t226 = query;
    const t227 = [t226];
    const t228 = t224(t225, t227);
    const t229 = "div";
    const t230 = "w-full max-w-md bg-white border rounded-lg shadow-xl";
    const t231 = handleKeyDown;
    const t232 = "div";
    const t233 = "flex items-center border-b px-3";
    const t234 = "span";
    const t235 = "text-gray-400";
    /* t236 = Discriminant(8) */
    const t237 = <t234 className={t235}>{t236}</t234>;
    const t238 = "input";
    const t239 = inputRef;
    const t240 = query;
    /* t241 = Discriminant(28) */
    const t242 = placeholder;
    const t243 = "flex-1 px-2 py-3 outline-none";
    const t244 = <t238 ref={t239} value={t240} onChange={t241} placeholder={t242} className={t243} />;
    const t245 = <t232 className={t233}>{t237}{t244}</t232>;
    const t246 = "div";
    const t247 = listRef;
    const t248 = "max-h-72 overflow-y-auto p-1";
    const t249 = filteredItems;
    const t250 = t249.length;
    const t251 = 0;
    const t252 = t250 === t251;
  } else {
  }
}

