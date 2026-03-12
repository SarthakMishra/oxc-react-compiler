import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by shadcn/ui command/combobox component
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function CommandMenu(t0) {
  const $ = _c(37);
  const {
    items,
    placeholder: t1,
    emptyMessage: t2,
    onOpenChange
  } = t0;
  const placeholder = t1 === undefined ? "Type a command or search..." : t1;
  const emptyMessage = t2 === undefined ? "No results found." : t2;
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef(null);
  const listRef = useRef(null);
  let t3;
  let t4;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t3 = () => {
      inputRef.current?.focus();
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
  bb0: {
    if (!query) {
      t5 = items;
      break bb0;
    }
    let t6;
    if ($[2] !== items || $[3] !== query) {
      const q = query.toLowerCase();
      t6 = items.filter(item => item.label.toLowerCase().includes(q) || item.description && item.description.toLowerCase().includes(q));
      $[2] = items;
      $[3] = query;
      $[4] = t6;
    } else {
      t6 = $[4];
    }
    t5 = t6;
  }
  const filteredItems = t5;
  let groups;
  if ($[5] !== filteredItems) {
    groups = new Map();
    for (const item_0 of filteredItems) {
      const group = item_0.group || "Actions";
      if (!groups.has(group)) {
        groups.set(group, []);
      }
      groups.get(group).push(item_0);
    }
    $[5] = filteredItems;
    $[6] = groups;
  } else {
    groups = $[6];
  }
  const groupedItems = groups;
  let result;
  if ($[7] !== groupedItems) {
    result = [];
    for (const items_0 of groupedItems.values()) {
      result.push(...items_0);
    }
    $[7] = groupedItems;
    $[8] = result;
  } else {
    result = $[8];
  }
  const flatList = result;
  let t6;
  if ($[9] !== onOpenChange) {
    t6 = item_1 => {
      item_1.onSelect();
      setQuery("");
      onOpenChange?.(false);
    };
    $[9] = onOpenChange;
    $[10] = t6;
  } else {
    t6 = $[10];
  }
  const handleSelect = t6;
  let t7;
  if ($[11] !== activeIndex || $[12] !== flatList || $[13] !== handleSelect || $[14] !== onOpenChange) {
    t7 = e => {
      bb56: switch (e.key) {
        case "ArrowDown":
          {
            e.preventDefault();
            setActiveIndex(i_0 => Math.min(i_0 + 1, flatList.length - 1));
            break bb56;
          }
        case "ArrowUp":
          {
            e.preventDefault();
            setActiveIndex(_temp);
            break bb56;
          }
        case "Enter":
          {
            e.preventDefault();
            if (flatList[activeIndex]) {
              handleSelect(flatList[activeIndex]);
            }
            break bb56;
          }
        case "Escape":
          {
            onOpenChange?.(false);
          }
      }
    };
    $[11] = activeIndex;
    $[12] = flatList;
    $[13] = handleSelect;
    $[14] = onOpenChange;
    $[15] = t7;
  } else {
    t7 = $[15];
  }
  const handleKeyDown = t7;
  let t8;
  if ($[16] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = () => {
      setActiveIndex(0);
    };
    $[16] = t8;
  } else {
    t8 = $[16];
  }
  let t9;
  if ($[17] !== query) {
    t9 = [query];
    $[17] = query;
    $[18] = t9;
  } else {
    t9 = $[18];
  }
  useEffect(t8, t9);
  let t10;
  if ($[19] === Symbol.for("react.memo_cache_sentinel")) {
    t10 = /*#__PURE__*/_jsx("span", {
      className: "text-gray-400",
      children: "\u2318"
    });
    $[19] = t10;
  } else {
    t10 = $[19];
  }
  let t11;
  if ($[20] === Symbol.for("react.memo_cache_sentinel")) {
    t11 = e_0 => setQuery(e_0.target.value);
    $[20] = t11;
  } else {
    t11 = $[20];
  }
  let t12;
  if ($[21] !== placeholder || $[22] !== query) {
    t12 = /*#__PURE__*/_jsxs("div", {
      className: "flex items-center border-b px-3",
      children: [t10, /*#__PURE__*/_jsx("input", {
        ref: inputRef,
        value: query,
        onChange: t11,
        placeholder: placeholder,
        className: "flex-1 px-2 py-3 outline-none"
      })]
    });
    $[21] = placeholder;
    $[22] = query;
    $[23] = t12;
  } else {
    t12 = $[23];
  }
  let t13;
  if ($[24] !== activeIndex || $[25] !== emptyMessage || $[26] !== filteredItems.length || $[27] !== flatList || $[28] !== groupedItems || $[29] !== handleSelect) {
    t13 = filteredItems.length === 0 ? /*#__PURE__*/_jsx("p", {
      className: "py-6 text-center text-sm text-gray-500",
      children: emptyMessage
    }) : Array.from(groupedItems.entries()).map(t14 => {
      const [group_0, groupItems] = t14;
      return /*#__PURE__*/_jsxs("div", {
        children: [/*#__PURE__*/_jsx("div", {
          className: "px-2 py-1.5 text-xs font-semibold text-gray-500",
          children: group_0
        }), groupItems.map(item_2 => {
          const index = flatList.indexOf(item_2);
          return /*#__PURE__*/_jsxs("button", {
            onClick: () => handleSelect(item_2),
            className: `w-full text-left px-2 py-1.5 rounded text-sm flex justify-between ${index === activeIndex ? "bg-blue-50 text-blue-700" : "hover:bg-gray-50"}`,
            children: [/*#__PURE__*/_jsxs("div", {
              children: [/*#__PURE__*/_jsx("span", {
                children: item_2.label
              }), item_2.description && /*#__PURE__*/_jsx("span", {
                className: "ml-2 text-gray-400",
                children: item_2.description
              })]
            }), item_2.shortcut && /*#__PURE__*/_jsx("kbd", {
              className: "text-xs bg-gray-100 px-1 rounded",
              children: item_2.shortcut
            })]
          }, item_2.id);
        })]
      }, group_0);
    });
    $[24] = activeIndex;
    $[25] = emptyMessage;
    $[26] = filteredItems.length;
    $[27] = flatList;
    $[28] = groupedItems;
    $[29] = handleSelect;
    $[30] = t13;
  } else {
    t13 = $[30];
  }
  let t14;
  if ($[31] !== t13) {
    t14 = /*#__PURE__*/_jsx("div", {
      ref: listRef,
      className: "max-h-72 overflow-y-auto p-1",
      children: t13
    });
    $[31] = t13;
    $[32] = t14;
  } else {
    t14 = $[32];
  }
  let t15;
  if ($[33] !== handleKeyDown || $[34] !== t12 || $[35] !== t14) {
    t15 = /*#__PURE__*/_jsxs("div", {
      className: "w-full max-w-md bg-white border rounded-lg shadow-xl",
      onKeyDown: handleKeyDown,
      children: [t12, t14]
    });
    $[33] = handleKeyDown;
    $[34] = t12;
    $[35] = t14;
    $[36] = t15;
  } else {
    t15 = $[36];
  }
  return t15;
}
function _temp(i) {
  return Math.max(i - 1, 0);
}