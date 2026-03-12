import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by excalidraw ColorPicker component
import { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
const DEFAULT_PRESETS = ['#000000', '#545454', '#a0a0a0', '#ffffff', '#e03131', '#e8590c', '#fcc419', '#40c057', '#228be6', '#7048e8', '#be4bdb', '#f06595'];
export function ColorPicker(t0) {
  const $ = _c(30);
  const {
    color,
    onChange,
    presets: t1,
    showCustom: t2
  } = t0;
  const presets = t1 === undefined ? DEFAULT_PRESETS : t1;
  const showCustom = t2 === undefined ? true : t2;
  const [isOpen, setIsOpen] = useState(false);
  const [customColor, setCustomColor] = useState(color);
  let t3;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t3 = [];
    $[0] = t3;
  } else {
    t3 = $[0];
  }
  const [recentColors, setRecentColors] = useState(t3);
  const popoverRef = useRef(null);
  let t4;
  let t5;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t4 = () => {
      const handleClickOutside = e => {
        if (popoverRef.current && !popoverRef.current.contains(e.target)) {
          setIsOpen(false);
        }
      };
      document.addEventListener("mousedown", handleClickOutside);
      return () => document.removeEventListener("mousedown", handleClickOutside);
    };
    t5 = [];
    $[1] = t4;
    $[2] = t5;
  } else {
    t4 = $[1];
    t5 = $[2];
  }
  useEffect(t4, t5);
  let t6;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    t6 = c => {
      setRecentColors(prev => {
        const next = [c, ...prev.filter(x => x !== c)];
        return next.slice(0, 5);
      });
    };
    $[3] = t6;
  } else {
    t6 = $[3];
  }
  const addToRecent = t6;
  let t7;
  if ($[4] !== onChange) {
    t7 = c_0 => {
      onChange(c_0);
      addToRecent(c_0);
      setIsOpen(false);
    };
    $[4] = onChange;
    $[5] = t7;
  } else {
    t7 = $[5];
  }
  const handlePresetClick = t7;
  let t8;
  if ($[6] !== customColor || $[7] !== onChange) {
    t8 = () => {
      if (/^#[0-9a-fA-F]{6}$/.test(customColor)) {
        onChange(customColor);
        addToRecent(customColor);
        setIsOpen(false);
      }
    };
    $[6] = customColor;
    $[7] = onChange;
    $[8] = t8;
  } else {
    t8 = $[8];
  }
  const handleCustomSubmit = t8;
  let rows;
  if ($[9] !== presets) {
    rows = [];
    for (let i = 0; i < presets.length; i = i + 4, i) {
      rows.push(presets.slice(i, i + 4));
    }
    $[9] = presets;
    $[10] = rows;
  } else {
    rows = $[10];
  }
  const groupedPresets = rows;
  let t9;
  if ($[11] !== isOpen) {
    t9 = () => setIsOpen(!isOpen);
    $[11] = isOpen;
    $[12] = t9;
  } else {
    t9 = $[12];
  }
  let t10;
  if ($[13] !== color) {
    t10 = {
      backgroundColor: color
    };
    $[13] = color;
    $[14] = t10;
  } else {
    t10 = $[14];
  }
  let t11;
  if ($[15] !== t10 || $[16] !== t9) {
    t11 = /*#__PURE__*/_jsx("button", {
      onClick: t9,
      className: "w-8 h-8 rounded border-2",
      style: t10,
      "aria-label": "Pick color"
    });
    $[15] = t10;
    $[16] = t9;
    $[17] = t11;
  } else {
    t11 = $[17];
  }
  let t12;
  if ($[18] !== color || $[19] !== customColor || $[20] !== groupedPresets || $[21] !== handleCustomSubmit || $[22] !== handlePresetClick || $[23] !== isOpen || $[24] !== recentColors || $[25] !== showCustom) {
    t12 = isOpen && /*#__PURE__*/_jsxs("div", {
      className: "absolute z-50 mt-1 p-3 bg-white rounded-lg shadow-lg border",
      children: [recentColors.length > 0 && /*#__PURE__*/_jsxs("div", {
        className: "mb-2",
        children: [/*#__PURE__*/_jsx("span", {
          className: "text-xs text-gray-500",
          children: "Recent"
        }), /*#__PURE__*/_jsx("div", {
          className: "flex gap-1 mt-1",
          children: recentColors.map(c_1 => /*#__PURE__*/_jsx("button", {
            onClick: () => handlePresetClick(c_1),
            className: "w-6 h-6 rounded border",
            style: {
              backgroundColor: c_1
            }
          }, c_1))
        })]
      }), /*#__PURE__*/_jsx("div", {
        className: "space-y-1",
        children: groupedPresets.map((row, i_0) => /*#__PURE__*/_jsx("div", {
          className: "flex gap-1",
          children: row.map(c_2 => /*#__PURE__*/_jsx("button", {
            onClick: () => handlePresetClick(c_2),
            className: `w-6 h-6 rounded border ${c_2 === color ? "ring-2 ring-blue-500" : ""}`,
            style: {
              backgroundColor: c_2
            }
          }, c_2))
        }, i_0))
      }), showCustom && /*#__PURE__*/_jsxs("div", {
        className: "mt-2 flex gap-1",
        children: [/*#__PURE__*/_jsx("input", {
          type: "text",
          value: customColor,
          onChange: e_0 => setCustomColor(e_0.target.value),
          placeholder: "#000000",
          className: "w-20 text-xs border rounded px-1"
        }), /*#__PURE__*/_jsx("button", {
          onClick: handleCustomSubmit,
          className: "text-xs px-2 bg-blue-500 text-white rounded",
          children: "Apply"
        })]
      })]
    });
    $[18] = color;
    $[19] = customColor;
    $[20] = groupedPresets;
    $[21] = handleCustomSubmit;
    $[22] = handlePresetClick;
    $[23] = isOpen;
    $[24] = recentColors;
    $[25] = showCustom;
    $[26] = t12;
  } else {
    t12 = $[26];
  }
  let t13;
  if ($[27] !== t11 || $[28] !== t12) {
    t13 = /*#__PURE__*/_jsxs("div", {
      className: "relative inline-block",
      ref: popoverRef,
      children: [t11, t12]
    });
    $[27] = t11;
    $[28] = t12;
    $[29] = t13;
  } else {
    t13 = $[29];
  }
  return t13;
}