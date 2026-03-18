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
  const $ = _c(47);
  let t8;
  let t14;
  let t121;
  let t122;
  let t26;
  let t30;
  let t31;
  let t123;
  let t37;
  let t40;
  let filteredItems;
  let t124;
  let t46;
  let t48;
  let groupedItems;
  let t125;
  let t54;
  let t56;
  let t126;
  let t127;
  let t63;
  let t65;
  let handleSelect;
  let t128;
  let t71;
  let t76;
  let t129;
  let t80;
  let t82;
  let t84;
  let t85;
  let t100;
  let t101;
  let t103;
  let t108;
  let { items, placeholder, emptyMessage, onOpenChange } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = "";
    $[0] = t8;
  } else {
    t8 = $[0];
  }
  let query;
  let setQuery;
  ([query, setQuery] = useState(t8));
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t14 = 0;
    $[1] = t14;
  } else {
    t14 = $[1];
  }
  let activeIndex;
  let setActiveIndex;
  ([activeIndex, setActiveIndex] = useState(t14));
  let inputRef;
  let t23 = useRef(null);
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
  let listRef = t122;
  listRef = useRef(t26);
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    t30 = () => {
      let t3 = inputRef.current.focus();
      return undefined;
    };
    t31 = [];
    $[5] = t30;
    $[6] = t31;
  } else {
    t30 = $[5];
    t31 = $[6];
  }
  let t32 = useEffect(t30, t31);
  if ($[7] !== items) {
    t37 = () => {
      if (!query) {
        return items;
      }
      let q;
      q = query.toLowerCase();
      let t9 = (item) => {
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
    t40 = [items, query];
    $[7] = items;
    $[8] = t123;
    $[9] = t37;
    $[10] = t40;
  } else {
    t123 = $[8];
    t37 = $[9];
    t40 = $[10];
  }
  filteredItems = t123;
  let t41 = useMemo(t37, t40);
  if ($[11] !== t41) {
    filteredItems = t41;
    t46 = () => {
      let groups;
      groups = new Map();
      let t5 = filteredItems[Symbol.iterator]();
      let t6 = t5.next();
      let item;
      item = t6;
      let group;
      let t9;
      t9 = item.group;
      t9 = "Actions";
      group = t9;
      if (!groups.has(group)) {
        let t20 = groups.set(group, []);
      }
      let t25 = groups.get(group).push(item);
    };
    t48 = [filteredItems];
    $[11] = t41;
    $[12] = filteredItems;
    $[13] = t124;
    $[14] = t46;
    $[15] = t48;
  } else {
    filteredItems = $[12];
    t124 = $[13];
    t46 = $[14];
    t48 = $[15];
  }
  groupedItems = t124;
  let t49 = useMemo(t46, t48);
  if ($[16] !== t49) {
    groupedItems = t49;
    t54 = () => {
      let result;
      result = [];
      let t5 = groupedItems.values()[Symbol.iterator]();
      let t6 = t5.next();
      let items;
      items = t6;
      let t10 = result.push(items);
    };
    t56 = [groupedItems];
    $[16] = t49;
    $[17] = groupedItems;
    $[18] = t125;
    $[19] = t54;
    $[20] = t56;
  } else {
    groupedItems = $[17];
    t125 = $[18];
    t54 = $[19];
    t56 = $[20];
  }
  let flatList = t125;
  let t57 = useMemo(t54, t56);
  if ($[21] !== t57 || $[22] !== onOpenChange) {
    t126 = t57;
    t63 = (item) => {
      let t2 = item.onSelect();
      let t6 = setQuery("");
      let t10 = onOpenChange(false);
      return undefined;
    };
    t65 = [onOpenChange];
    $[21] = t57;
    $[22] = onOpenChange;
    $[23] = t126;
    $[24] = t127;
    $[25] = t63;
    $[26] = t65;
  } else {
    t126 = $[23];
    t127 = $[24];
    t63 = $[25];
    t65 = $[26];
  }
  flatList = t126;
  handleSelect = t127;
  let t66 = useCallback(t63, t65);
  if ($[27] !== t66 || $[28] !== onOpenChange) {
    handleSelect = t66;
    t71 = (e) => {
      switch (e.key) {
        case "ArrowDown":
          let t8 = e.preventDefault();
          let t11 = (i) => {
            return Math.min(i + 1, flatList.length - 1);
          };
          let t12 = setActiveIndex(t11);
          return undefined;
        case "ArrowUp":
          let t14 = e.preventDefault();
          let t16 = (i) => {
            return Math.max(i - 1, 0);
          };
          let t17 = setActiveIndex(t16);
        case "Enter":
          let t19 = e.preventDefault();
          if (flatList[activeIndex]) {
            let t30 = handleSelect(flatList[activeIndex]);
          }
        case "Escape":
          let t34 = onOpenChange(false);
      }
    };
    t76 = [flatList, activeIndex, handleSelect, onOpenChange];
    $[27] = t66;
    $[28] = onOpenChange;
    $[29] = handleSelect;
    $[30] = t128;
    $[31] = t71;
    $[32] = t76;
  } else {
    handleSelect = $[29];
    t128 = $[30];
    t71 = $[31];
    t76 = $[32];
  }
  let handleKeyDown = t128;
  let t77 = useCallback(t71, t76);
  if ($[33] !== t77) {
    t129 = t77;
    t80 = () => {
      let t3 = setActiveIndex(0);
      return undefined;
    };
    t82 = [query];
    $[33] = t77;
    $[34] = t129;
    $[35] = t80;
    $[36] = t82;
  } else {
    t129 = $[34];
    t80 = $[35];
    t82 = $[36];
  }
  handleKeyDown = t129;
  let t83 = useEffect(t80, t82);
  if ($[37] !== filteredItems.length || $[38] !== placeholder) {
    let t96 = (e) => {
      return setQuery(e.target.value);
    };
    if (filteredItems.length === 0) {
      if ($[39] !== emptyMessage) {
        t108 = <p className="py-6 text-center text-sm text-gray-500">{emptyMessage}</p>;
        $[39] = emptyMessage;
        $[40] = t108;
      } else {
        t108 = $[40];
      }
    } else {
      let t117 = (t0) => {
        let group;
        let groupItems;
        ([group, groupItems] = t0);
        let t11 = (item) => {
          let index;
          index = flatList.indexOf(item);
          let t9 = () => {
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
    return <div className="w-full max-w-md bg-white border rounded-lg shadow-xl" onKeyDown={handleKeyDown}><div className="flex items-center border-b px-3"><span className="text-gray-400">⌘</span><input ref={inputRef} value={query} onChange={t96} placeholder={placeholder} className="flex-1 px-2 py-3 outline-none" /></div><div ref={listRef} className="max-h-72 overflow-y-auto p-1">{t108}</div></div>;
    $[37] = filteredItems.length;
    $[38] = placeholder;
    $[39] = t84;
    $[40] = t85;
    $[41] = t100;
    $[42] = t101;
    $[43] = t103;
    $[44] = t108;
  } else {
    t84 = $[39];
    t85 = $[40];
    t100 = $[41];
    t101 = $[42];
    t103 = $[43];
    t108 = $[44];
  }
}

