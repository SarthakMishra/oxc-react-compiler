import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
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
  const $ = _c(39);
  let items;
  let placeholder;
  let emptyMessage;
  let onOpenChange;
  if ($[0] !== items || $[1] !== placeholder || $[2] !== emptyMessage || $[3] !== onOpenChange) {
    $[0] = items;
    $[1] = placeholder;
    $[2] = emptyMessage;
    $[3] = onOpenChange;
  } else {
  }
  ({ items, placeholder, emptyMessage, onOpenChange } = t0);
  const t173 = useState;
  const t174 = "";
  const t175 = t173(t174);
  let query;
  let setQuery;
  if ($[4] !== query || $[5] !== setQuery) {
    $[4] = query;
    $[5] = setQuery;
  } else {
  }
  ([query, setQuery] = t175);
  const t179 = useState;
  const t180 = 0;
  const t181 = t179(t180);
  let activeIndex;
  let setActiveIndex;
  if ($[6] !== activeIndex || $[7] !== setActiveIndex) {
    $[6] = activeIndex;
    $[7] = setActiveIndex;
  } else {
  }
  ([activeIndex, setActiveIndex] = t181);
  let inputRef;
  if ($[8] !== inputRef) {
    $[8] = inputRef;
  } else {
  }
  const t186 = useRef;
  const t187 = null;
  const t188 = t186(t187);
  inputRef = t188;
  let listRef;
  if ($[9] !== listRef) {
    $[9] = listRef;
  } else {
  }
  const t191 = useRef;
  const t192 = null;
  const t193 = t191(t192);
  listRef = t193;
  const t195 = useEffect;
  const t196 = () => {
    const t1 = inputRef;
    const t2 = t1.current;
    const t3 = t2.focus();
    const t4 = undefined;
    return t4;
  };
  const t197 = [];
  const t198 = t195(t196, t197);
  let filteredItems;
  if ($[10] !== filteredItems) {
    $[10] = filteredItems;
  } else {
  }
  let groupedItems;
  if ($[11] !== useMemo || $[12] !== items || $[13] !== query || $[14] !== filteredItems || $[15] !== groupedItems) {
    const t200 = useMemo;
    const t201 = () => {
      const t1 = query;
      const t2 = !t1;
      const t4 = items;
      return t4;
      let q;
      const t8 = query;
      const t9 = t8.toLowerCase();
      q = t9;
      const t12 = items;
      const t13 = (item) => {
        const t2 = item;
        const t3 = t2.label;
        const t4 = t3.toLowerCase();
        const t6 = q;
        const t7 = t4.includes(t6);
        return t19;
        const t9 = item;
        const t10 = t9.description;
        const t12 = item;
        const t13 = t12.description;
        const t14 = t13.toLowerCase();
        const t16 = q;
        const t17 = t14.includes(t16);
      };
      const t14 = t12.filter(t13);
      return t14;
      const t15 = undefined;
      return t15;
    };
    const t202 = items;
    const t203 = query;
    const t204 = [t202, t203];
    const t205 = t200(t201, t204);
    filteredItems = t205;
    $[11] = useMemo;
    $[12] = items;
    $[13] = query;
    $[14] = filteredItems;
    $[15] = groupedItems;
  } else {
  }
  let flatList;
  if ($[16] !== useMemo || $[17] !== filteredItems || $[18] !== groupedItems || $[19] !== flatList) {
    const t208 = useMemo;
    const t209 = () => {
      let groups;
      const t2 = Map;
      const t3 = new t2();
      groups = t3;
      const t6 = filteredItems;
      const t7 = t6[Symbol.iterator]();
      const t8 = t7.next();
      let item;
      item = t8;
      let group;
      const t41 = groups;
      return t41;
      const t15 = item;
      const t16 = t15.group;
      const t17 = "Actions";
      group = t18;
      const t21 = groups;
      const t23 = group;
      const t24 = t21.has(t23);
      const t25 = !t24;
      const t27 = groups;
      const t29 = group;
      const t30 = [];
      const t31 = t27.set(t29, t30);
      const t33 = groups;
      const t35 = group;
      const t36 = t33.get(t35);
      const t38 = item;
      const t39 = t36.push(t38);
      const t42 = undefined;
      return t42;
    };
    const t210 = filteredItems;
    const t211 = [t210];
    const t212 = t208(t209, t211);
    groupedItems = t212;
    $[16] = useMemo;
    $[17] = filteredItems;
    $[18] = groupedItems;
    $[19] = flatList;
  } else {
  }
  let handleSelect;
  if ($[20] !== useMemo || $[21] !== groupedItems || $[22] !== flatList || $[23] !== handleSelect) {
    const t215 = useMemo;
    const t216 = () => {
      let result;
      const t2 = [];
      result = t2;
      const t5 = groupedItems;
      const t6 = t5.values();
      const t7 = t6[Symbol.iterator]();
      const t8 = t7.next();
      let items;
      items = t8;
      const t13 = result;
      const t15 = items;
      const t16 = t13.push(t15);
      const t18 = result;
      return t18;
      const t19 = undefined;
      return t19;
    };
    const t217 = groupedItems;
    const t218 = [t217];
    const t219 = t215(t216, t218);
    flatList = t219;
    $[20] = useMemo;
    $[21] = groupedItems;
    $[22] = flatList;
    $[23] = handleSelect;
  } else {
  }
  let handleKeyDown;
  if ($[24] !== useCallback || $[25] !== onOpenChange || $[26] !== handleSelect || $[27] !== handleKeyDown) {
    const t222 = useCallback;
    const t223 = (item) => {
      const t2 = item;
      const t3 = t2.onSelect();
      const t5 = setQuery;
      const t6 = "";
      const t7 = t5(t6);
      const t9 = onOpenChange;
      const t10 = false;
      const t11 = t9(t10);
      const t12 = undefined;
      return t12;
    };
    const t224 = onOpenChange;
    const t225 = [t224];
    const t226 = t222(t223, t225);
    handleSelect = t226;
    $[24] = useCallback;
    $[25] = onOpenChange;
    $[26] = handleSelect;
    $[27] = handleKeyDown;
  } else {
  }
  if ($[28] !== useCallback || $[29] !== flatList || $[30] !== activeIndex || $[31] !== handleSelect || $[32] !== onOpenChange || $[33] !== handleKeyDown) {
    const t229 = useCallback;
    const t230 = (e) => {
      const t2 = e;
      const t3 = t2.key;
      const t4 = "ArrowDown";
      const t5 = "ArrowUp";
      const t6 = "Enter";
      const t7 = "Escape";
      const t42 = undefined;
      return t42;
      const t9 = e;
      const t10 = t9.preventDefault();
      const t12 = setActiveIndex;
      const t13 = (i) => {
        const t1 = Math;
        const t3 = i;
        const t4 = 1;
        const t5 = t3 + t4;
        const t7 = flatList;
        const t8 = t7.length;
        const t9 = 1;
        const t10 = t8 - t9;
        const t11 = t1.min(t5, t10);
        return t11;
      };
      const t14 = t12(t13);
      const t16 = e;
      const t17 = t16.preventDefault();
      const t19 = setActiveIndex;
      const t20 = (i) => {
        const t1 = Math;
        const t3 = i;
        const t4 = 1;
        const t5 = t3 - t4;
        const t6 = 0;
        const t7 = t1.max(t5, t6);
        return t7;
      };
      const t21 = t19(t20);
      const t23 = e;
      const t24 = t23.preventDefault();
      const t26 = flatList;
      const t28 = activeIndex;
      const t29 = t26[t28];
      const t39 = onOpenChange;
      const t40 = false;
      const t41 = t39(t40);
      const t31 = handleSelect;
      const t33 = flatList;
      const t35 = activeIndex;
      const t36 = t33[t35];
      const t37 = t31(t36);
    };
    const t231 = flatList;
    const t232 = activeIndex;
    const t233 = handleSelect;
    const t234 = onOpenChange;
    const t235 = [t231, t232, t233, t234];
    const t236 = t229(t230, t235);
    handleKeyDown = t236;
    $[28] = useCallback;
    $[29] = flatList;
    $[30] = activeIndex;
    $[31] = handleSelect;
    $[32] = onOpenChange;
    $[33] = handleKeyDown;
  } else {
  }
  const t238 = useEffect;
  const t239 = () => {
    const t1 = setActiveIndex;
    const t2 = 0;
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  if ($[34] !== query || $[35] !== t238 || $[36] !== t239 || $[37] !== filteredItems || $[38] !== t265) {
    const t240 = query;
    const t241 = [t240];
    const t242 = t238(t239, t241);
    const t243 = "div";
    const t244 = "w-full max-w-md bg-white border rounded-lg shadow-xl";
    const t245 = handleKeyDown;
    const t246 = "div";
    const t247 = "flex items-center border-b px-3";
    const t248 = "span";
    const t249 = "text-gray-400";
    const t250 = "⌘";
    const t251 = _jsx(t248, { className: t249, children: t250 });
    const t252 = "input";
    const t253 = inputRef;
    const t254 = query;
    const t255 = (e) => {
      const t2 = setQuery;
      const t4 = e;
      const t5 = t4.target;
      const t6 = t5.value;
      const t7 = t2(t6);
      return t7;
    };
    const t256 = placeholder;
    const t257 = "flex-1 px-2 py-3 outline-none";
    const t258 = _jsx(t252, { ref: t253, value: t254, onChange: t255, placeholder: t256, className: t257 });
    const t259 = _jsxs(t246, { className: t247, children: [t251, t258] });
    const t260 = "div";
    const t261 = listRef;
    const t262 = "max-h-72 overflow-y-auto p-1";
    const t263 = filteredItems;
    const t264 = t263.length;
    const t265 = 0;
    const t266 = t264 === t265;
    $[34] = query;
    $[35] = t238;
    $[36] = t239;
    $[37] = filteredItems;
    $[38] = t265;
  } else {
  }
}

