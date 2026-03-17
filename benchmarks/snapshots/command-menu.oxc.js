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
  const $ = _c(53);
  const { items, placeholder, emptyMessage, onOpenChange } = t0;
  let t8;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t8;
  } else {
    t8 = $[0];
  }
  let t14;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    $[1] = t14;
  } else {
    t14 = $[1];
  }
  let inputRef;
  const t23 = useRef(null);
  let t122;
  let t121;
  let t26;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    t121 = t23;
    t26 = null;
    $[2] = t121;
    $[3] = t122;
    $[4] = t26;
  } else {
    t121 = $[2];
    t122 = $[3];
    t26 = $[4];
  }
  inputRef = t121;
  const listRef = t122;
  const t27 = useRef(t26);
  let t123;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    t123 = t27;
    $[5] = t123;
  } else {
    t123 = $[5];
  }
  const listRef = t123;
  let t30;
  let t31;
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    t30 = () => {
      const t3 = inputRef.current.focus();
      return undefined;
    };
    t31 = [];
    $[6] = t30;
    $[7] = t31;
  } else {
    t30 = $[6];
    t31 = $[7];
  }
  const t32 = useEffect(t30, t31);
  let filteredItems;
  const t37 = () => {
    if (!query) {
      return items;
    }
    let q;
    q = query.toLowerCase();
    const t9 = (item) => {
      let t1;
      t1 = item.label.toLowerCase().includes(q);
      let t8;
      t8 = item.description;
      t8 = item.description.toLowerCase().includes(q);
      t1 = t8;
      return t1;
    };
    return items.filter(t9);
  };
  const t41 = useMemo(t37, [items, query]);
  let t124;
  let t46;
  let t48;
  if ($[8] !== t41) {
    filteredItems = t41;
    t46 = () => {
      let groups;
      groups = new Map();
      const t5 = filteredItems[Symbol.iterator]();
      const t6 = t5.next();
      let item;
      item = t6;
      let group;
      let t9;
      t9 = item.group;
      t9 = "Actions";
      group = t9;
      if (!groups.has(group)) {
        const t20 = groups.set(group, []);
      }
      const t25 = groups.get(group).push(item);
    };
    t48 = [filteredItems];
    $[8] = t41;
    $[9] = filteredItems;
    $[10] = t124;
    $[11] = t46;
    $[12] = t48;
  } else {
    filteredItems = $[9];
    t124 = $[10];
    t46 = $[11];
    t48 = $[12];
  }
  const groupedItems = t124;
  const t49 = useMemo(t46, t48);
  let t125;
  let groupedItems;
  let t54;
  let t56;
  if ($[13] !== t49) {
    groupedItems = t49;
    t54 = () => {
      let result;
      result = [];
      const t5 = groupedItems.values()[Symbol.iterator]();
      const t6 = t5.next();
      let items;
      items = t6;
      const t10 = result.push(items);
    };
    t56 = [groupedItems];
    $[13] = t49;
    $[14] = groupedItems;
    $[15] = t125;
    $[16] = t54;
    $[17] = t56;
  } else {
    groupedItems = $[14];
    t125 = $[15];
    t54 = $[16];
    t56 = $[17];
  }
  const flatList = t125;
  const t57 = useMemo(t54, t56);
  let t127;
  let t126;
  let t63;
  let t65;
  if ($[18] !== t57 || $[19] !== onOpenChange) {
    t126 = t57;
    t63 = (item) => {
      const t2 = item.onSelect();
      const t6 = setQuery("");
      const t10 = onOpenChange(false);
      return undefined;
    };
    t65 = [onOpenChange];
    $[18] = t57;
    $[19] = onOpenChange;
    $[20] = t126;
    $[21] = t127;
    $[22] = t63;
    $[23] = t65;
  } else {
    t126 = $[20];
    t127 = $[21];
    t63 = $[22];
    t65 = $[23];
  }
  const flatList = t126;
  const handleSelect = t127;
  const t66 = useCallback(t63, t65);
  let t128;
  let handleSelect;
  let t71;
  let t76;
  if ($[24] !== t66 || $[25] !== onOpenChange) {
    handleSelect = t66;
    t71 = (e) => {
      switch (e.key) {
        case "ArrowDown":
          const t8 = e.preventDefault();
          const t11 = (i) => {
            return Math.min(i + 1, flatList.length - 1);
          };
          const t12 = setActiveIndex(t11);
          return undefined;
        case "ArrowUp":
          const t14 = e.preventDefault();
          const t16 = (i) => {
            return Math.max(i - 1, 0);
          };
          const t17 = setActiveIndex(t16);
        case "Enter":
          const t19 = e.preventDefault();
          if (flatList[activeIndex]) {
            const t30 = handleSelect(flatList[activeIndex]);
          }
        case "Escape":
          const t34 = onOpenChange(false);
      }
    };
    t76 = [flatList, activeIndex, handleSelect, onOpenChange];
    $[24] = t66;
    $[25] = onOpenChange;
    $[26] = handleSelect;
    $[27] = t128;
    $[28] = t71;
    $[29] = t76;
  } else {
    handleSelect = $[26];
    t128 = $[27];
    t71 = $[28];
    t76 = $[29];
  }
  const handleKeyDown = t128;
  const t77 = useCallback(t71, t76);
  let t129;
  let t80;
  let t82;
  if ($[30] !== t77) {
    t129 = t77;
    t80 = () => {
      const t3 = setActiveIndex(0);
      return undefined;
    };
    t82 = [query];
    $[30] = t77;
    $[31] = t129;
    $[32] = t80;
    $[33] = t82;
  } else {
    t129 = $[31];
    t80 = $[32];
    t82 = $[33];
  }
  const handleKeyDown = t129;
  const t83 = useEffect(t80, t82);
  let t84;
  let t85;
  let t100;
  let t101;
  let t103;
  let t105;
  if ($[34] !== placeholder) {
    t84 = "div";
    t85 = "w-full max-w-md bg-white border rounded-lg shadow-xl";
    const t96 = (e) => {
      return setQuery(e.target.value);
    };
    t100 = (
      <div className="flex items-center border-b px-3">
        <span className="text-gray-400">⌘</span>
        <input ref={inputRef} value={query} onChange={t96} placeholder={placeholder} className="flex-1 px-2 py-3 outline-none" />
      </div>
    );
    t101 = "div";
    t103 = "max-h-72 overflow-y-auto p-1";
    t105 = filteredItems.length;
    $[34] = placeholder;
    $[35] = t84;
    $[36] = t85;
    $[37] = t100;
    $[38] = t101;
    $[39] = t103;
    $[40] = t105;
  } else {
    t84 = $[35];
    t85 = $[36];
    t100 = $[37];
    t101 = $[38];
    t103 = $[39];
    t105 = $[40];
  }
  let t108;
  if (t105 === 0) {
    let t120;
    let t130;
    let t37;
    let t40;
    let t131;
    let t132;
    let t133;
    let t22;
    if ($[41] !== t41 || $[42] !== emptyMessage || $[43] !== items || $[44] !== onOpenChange) {
      t108 = <p className="py-6 text-center text-sm text-gray-500">{emptyMessage}</p>;
      $[41] = t41;
      $[42] = emptyMessage;
      $[43] = items;
      $[44] = onOpenChange;
      $[45] = t120;
      $[46] = t130;
      $[47] = t37;
      $[48] = t40;
      $[49] = t131;
      $[50] = t132;
      $[51] = t133;
      $[52] = t22;
    } else {
      t120 = $[45];
      t130 = $[46];
      t37 = $[47];
      t40 = $[48];
      t131 = $[49];
      t132 = $[50];
      t133 = $[51];
      t22 = $[52];
    }
    filteredItems = t130;
    const query = t131;
    const activeIndex = t132;
    inputRef = t133;
  } else {
    const t117 = (t0) => {
      let group;
      let groupItems;
      const t11 = (item) => {
        let index;
        index = flatList.indexOf(item);
        const t9 = () => {
          return handleSelect(item);
        };
        let t14;
        if (index === activeIndex) {
          t14 = "bg-blue-50 text-blue-700";
        } else {
          t14 = "hover:bg-gray-50";
        }
        let t23;
        t23 = item.description;
        t23 = <span className="ml-2 text-gray-400">{item.description}</span>;
        let t32;
        t32 = item.shortcut;
        t32 = <kbd className="text-xs bg-gray-100 px-1 rounded">{item.shortcut}</kbd>;
        return <button key={item.id} onClick={t9} className={`w-full text-left px-2 py-1.5 rounded text-sm flex justify-between ${t14}`}><div><span>{item.label}</span>{t23}</div>{t32}</button>;
      };
      return <div key={group}><div className="px-2 py-1.5 text-xs font-semibold text-gray-500">{group}</div>{groupItems.map(t11)}</div>;
    };
    t108 = Array.from(groupedItems.entries()).map(t117);
  }
  return <t84 className={t85} onKeyDown={handleKeyDown}>{t100}<t101 ref={listRef} className={t103}>{t108}</t101></t84>;
}

