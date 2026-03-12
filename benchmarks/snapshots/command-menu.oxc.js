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
  const $ = _c(48);
  const { items, placeholder, emptyMessage, onOpenChange } = t0;
  if ($[0] !== items || $[1] !== placeholder || $[2] !== emptyMessage || $[3] !== onOpenChange) {
    $[0] = items;
    $[1] = placeholder;
    $[2] = emptyMessage;
    $[3] = onOpenChange;
  } else {
  }
  const t176 = useState;
  const t177 = "";
  const t178 = t176(t177);
  let query;
  let setQuery;
  if ($[4] !== query || $[5] !== setQuery) {
    $[4] = query;
    $[5] = setQuery;
  } else {
  }
  ([query, setQuery] = t178);
  const t182 = useState;
  const t183 = 0;
  const t184 = t182(t183);
  let activeIndex;
  let setActiveIndex;
  if ($[6] !== activeIndex || $[7] !== setActiveIndex) {
    $[6] = activeIndex;
    $[7] = setActiveIndex;
  } else {
  }
  ([activeIndex, setActiveIndex] = t184);
  let inputRef;
  if ($[8] !== inputRef) {
    $[8] = inputRef;
  } else {
  }
  const t189 = useRef;
  const t190 = null;
  const t191 = t189(t190);
  inputRef = t191;
  let listRef;
  if ($[9] !== listRef) {
    $[9] = listRef;
  } else {
  }
  const t194 = useRef;
  const t195 = null;
  const t196 = t194(t195);
  listRef = t196;
  const t198 = useEffect;
  const t199 = () => {
    const t1 = inputRef;
    const t2 = t1.current;
    const t3 = t2.focus();
    const t4 = undefined;
    return t4;
  };
  const t200 = [];
  const t201 = t198(t199, t200);
  let filteredItems;
  if ($[10] !== filteredItems) {
    $[10] = filteredItems;
  } else {
  }
  let groupedItems;
  if ($[11] !== useMemo || $[12] !== items || $[13] !== query || $[14] !== filteredItems || $[15] !== groupedItems) {
    const t203 = useMemo;
    const t204 = () => {
      const t1 = query;
      const t2 = !t1;
      if (t2) {
        const t4 = items;
        return t4;
      } else {
      }
      let q;
      const t8 = query;
      const t9 = t8.toLowerCase();
      q = t9;
      const t12 = items;
      const t13 = (item) => {
        let t1;
        const t4 = item;
        const t5 = t4.label;
        const t6 = t5.toLowerCase();
        const t8 = q;
        const t9 = t6.includes(t8);
        t1 = t9;
        let t11;
        const t14 = item;
        const t15 = t14.description;
        t11 = t15;
        const t18 = item;
        const t19 = t18.description;
        const t20 = t19.toLowerCase();
        const t22 = q;
        const t23 = t20.includes(t22);
        t11 = t23;
        t1 = t11;
        return t1;
      };
      const t14 = t12.filter(t13);
      return t14;
    };
    const t205 = items;
    const t206 = query;
    const t207 = [t205, t206];
    const t208 = t203(t204, t207);
    filteredItems = t208;
    $[11] = useMemo;
    $[12] = items;
    $[13] = query;
    $[14] = filteredItems;
    $[15] = groupedItems;
  } else {
  }
  let flatList;
  if ($[16] !== useMemo || $[17] !== filteredItems || $[18] !== groupedItems || $[19] !== flatList) {
    const t211 = useMemo;
    const t212 = () => {
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
      let t14;
      const t17 = item;
      const t18 = t17.group;
      t14 = t18;
      const t20 = "Actions";
      t14 = t20;
      group = t14;
      const t24 = groups;
      const t26 = group;
      const t27 = t24.has(t26);
      const t28 = !t27;
      if (t28) {
        const t30 = groups;
        const t32 = group;
        const t33 = [];
        const t34 = t30.set(t32, t33);
      } else {
      }
      const t36 = groups;
      const t38 = group;
      const t39 = t36.get(t38);
      const t41 = item;
      const t42 = t39.push(t41);
    };
    const t213 = filteredItems;
    const t214 = [t213];
    const t215 = t211(t212, t214);
    groupedItems = t215;
    $[16] = useMemo;
    $[17] = filteredItems;
    $[18] = groupedItems;
    $[19] = flatList;
  } else {
  }
  let handleSelect;
  if ($[20] !== useMemo || $[21] !== groupedItems || $[22] !== flatList || $[23] !== handleSelect) {
    const t218 = useMemo;
    const t219 = () => {
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
    };
    const t220 = groupedItems;
    const t221 = [t220];
    const t222 = t218(t219, t221);
    flatList = t222;
    $[20] = useMemo;
    $[21] = groupedItems;
    $[22] = flatList;
    $[23] = handleSelect;
  } else {
  }
  let handleKeyDown;
  if ($[24] !== useCallback || $[25] !== onOpenChange || $[26] !== handleSelect || $[27] !== handleKeyDown) {
    const t225 = useCallback;
    const t226 = (item) => {
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
    const t227 = onOpenChange;
    const t228 = [t227];
    const t229 = t225(t226, t228);
    handleSelect = t229;
    $[24] = useCallback;
    $[25] = onOpenChange;
    $[26] = handleSelect;
    $[27] = handleKeyDown;
  } else {
  }
  if ($[28] !== useCallback || $[29] !== flatList || $[30] !== activeIndex || $[31] !== handleSelect || $[32] !== onOpenChange || $[33] !== handleKeyDown) {
    const t232 = useCallback;
    const t233 = (e) => {
      const t2 = e;
      const t3 = t2.key;
      const t4 = "ArrowDown";
      const t5 = "ArrowUp";
      const t6 = "Enter";
      const t7 = "Escape";
      switch (t3) {
        case t4:
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
          const t42 = undefined;
          return t42;
        case t5:
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
        case t6:
          const t23 = e;
          const t24 = t23.preventDefault();
          const t26 = flatList;
          const t28 = activeIndex;
          const t29 = t26[t28];
          if (t29) {
            const t31 = handleSelect;
            const t33 = flatList;
            const t35 = activeIndex;
            const t36 = t33[t35];
            const t37 = t31(t36);
          } else {
          }
        case t7:
          const t39 = onOpenChange;
          const t40 = false;
          const t41 = t39(t40);
      }
    };
    const t234 = flatList;
    const t235 = activeIndex;
    const t236 = handleSelect;
    const t237 = onOpenChange;
    const t238 = [t234, t235, t236, t237];
    const t239 = t232(t233, t238);
    handleKeyDown = t239;
    $[28] = useCallback;
    $[29] = flatList;
    $[30] = activeIndex;
    $[31] = handleSelect;
    $[32] = onOpenChange;
    $[33] = handleKeyDown;
  } else {
  }
  const t241 = useEffect;
  const t242 = () => {
    const t1 = setActiveIndex;
    const t2 = 0;
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  const t243 = query;
  const t244 = [t243];
  const t245 = t241(t242, t244);
  const t246 = "div";
  const t247 = "w-full max-w-md bg-white border rounded-lg shadow-xl";
  const t248 = handleKeyDown;
  const t249 = "div";
  const t250 = "flex items-center border-b px-3";
  const t251 = "span";
  const t252 = "text-gray-400";
  const t253 = "⌘";
  const t254 = _jsx(t251, { className: t252, children: t253 });
  const t255 = "input";
  const t256 = inputRef;
  const t257 = query;
  const t258 = (e) => {
    const t2 = setQuery;
    const t4 = e;
    const t5 = t4.target;
    const t6 = t5.value;
    const t7 = t2(t6);
    return t7;
  };
  const t259 = placeholder;
  const t260 = "flex-1 px-2 py-3 outline-none";
  const t261 = _jsx(t255, { ref: t256, value: t257, onChange: t258, placeholder: t259, className: t260 });
  const t262 = _jsxs(t249, { className: t250, children: [t254, t261] });
  const t263 = "div";
  const t264 = listRef;
  const t265 = "max-h-72 overflow-y-auto p-1";
  const t266 = filteredItems;
  const t267 = t266.length;
  const t268 = 0;
  const t269 = t267 === t268;
  let t152;
  if (t269) {
    let t291;
    if ($[34] !== groupedItems || $[35] !== t152 || $[36] !== query || $[37] !== t241 || $[38] !== t242 || $[39] !== handleKeyDown || $[40] !== inputRef || $[41] !== query || $[42] !== placeholder || $[43] !== listRef || $[44] !== filteredItems || $[45] !== t268 || $[46] !== emptyMessage) {
      const t292 = "p";
      const t293 = "py-6 text-center text-sm text-gray-500";
      const t294 = emptyMessage;
      const t295 = _jsx(t292, { className: t293, children: t294 });
      t152 = t295;
      $[47] = t291;
      $[34] = groupedItems;
      $[35] = t152;
      $[36] = query;
      $[37] = t241;
      $[38] = t242;
      $[39] = handleKeyDown;
      $[40] = inputRef;
      $[41] = query;
      $[42] = placeholder;
      $[43] = listRef;
      $[44] = filteredItems;
      $[45] = t268;
      $[46] = emptyMessage;
    } else {
      t291 = $[47];
    }
  } else {
    const t271 = Array;
    const t272 = groupedItems;
    const t273 = t272.entries();
    const t274 = t271.from(t273);
    const t275 = (t0) => {
      let group;
      let groupItems;
      ([group, groupItems] = t0);
      const t6 = "div";
      const t8 = group;
      const t9 = "div";
      const t10 = "px-2 py-1.5 text-xs font-semibold text-gray-500";
      const t12 = group;
      const t13 = _jsx(t9, { className: t10, children: t12 });
      const t15 = groupItems;
      const t16 = (item) => {
        let index;
        const t4 = flatList;
        const t6 = item;
        const t7 = t4.indexOf(t6);
        index = t7;
        const t9 = "button";
        const t11 = item;
        const t12 = t11.id;
        const t13 = () => {
          const t1 = handleSelect;
          const t3 = item;
          const t4 = t1(t3);
          return t4;
        };
        const t15 = index;
        const t17 = activeIndex;
        const t18 = t15 === t17;
        let t19;
        if (t18) {
          const t21 = "bg-blue-50 text-blue-700";
          t19 = t21;
        } else {
          const t23 = "hover:bg-gray-50";
          t19 = t23;
        }
        const t25 = `w-full text-left px-2 py-1.5 rounded text-sm flex justify-between ${t19}`;
        const t26 = "div";
        const t27 = "span";
        const t29 = item;
        const t30 = t29.label;
        const t31 = _jsx(t27, { children: t30 });
        let t32;
        const t35 = item;
        const t36 = t35.description;
        t32 = t36;
        const t38 = "span";
        const t39 = "ml-2 text-gray-400";
        const t41 = item;
        const t42 = t41.description;
        const t43 = _jsx(t38, { className: t39, children: t42 });
        t32 = t43;
        const t45 = _jsxs(t26, { children: [t31, t32] });
        let t46;
        const t49 = item;
        const t50 = t49.shortcut;
        t46 = t50;
        const t52 = "kbd";
        const t53 = "text-xs bg-gray-100 px-1 rounded";
        const t55 = item;
        const t56 = t55.shortcut;
        const t57 = _jsx(t52, { className: t53, children: t56 });
        t46 = t57;
        const t59 = _jsxs(t9, { key: t12, onClick: t13, className: t25, children: [t45, t46] });
        return t59;
      };
      const t17 = t15.map(t16);
      const t18 = _jsxs(t6, { key: t8, children: [t13, t17] });
      return t18;
    };
    const t276 = t274.map(t275);
    t152 = t276;
  }
  const t290 = _jsx(t263, { ref: t264, className: t265, children: t152 });
  t291 = _jsxs(t246, { className: t247, onKeyDown: t248, children: [t262, t290] });
  return t291;
}

