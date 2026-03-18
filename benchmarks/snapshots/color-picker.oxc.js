import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by excalidraw ColorPicker component
import { useState, useCallback, useMemo, useRef, useEffect } from 'react';

interface ColorPickerProps {
  color: string;
  onChange: (color: string) => void;
  presets?: string[];
  showCustom?: boolean;
}

const DEFAULT_PRESETS = [
  '#000000', '#545454', '#a0a0a0', '#ffffff',
  '#e03131', '#e8590c', '#fcc419', '#40c057',
  '#228be6', '#7048e8', '#be4bdb', '#f06595',
];

export function ColorPicker(t0) {
  const $ = _c(39);
  let t8;
  let t20;
  let t128;
  let t28;
  let t32;
  let t33;
  let addToRecent;
  let t129;
  let t46;
  let t49;
  let t130;
  let t131;
  let t55;
  let t59;
  let t132;
  let t133;
  let t66;
  let t68;
  let t134;
  let t71;
  let t72;
  let t80;
  let t81;
  let t85;
  let { color, onChange, presets, showCustom } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = false;
    $[0] = t8;
  } else {
    t8 = $[0];
  }
  let isOpen;
  let setIsOpen;
  ([isOpen, setIsOpen] = useState(t8));
  let t15 = useState(color);
  let customColor;
  let setCustomColor;
  ([customColor, setCustomColor] = t15);
  if ($[1] !== t15) {
    $[1] = t15;
    $[2] = customColor;
    $[3] = setCustomColor;
  } else {
    customColor = $[2];
    setCustomColor = $[3];
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t20 = [];
    $[4] = t20;
  } else {
    t20 = $[4];
  }
  let t21 = useState(t20);
  let recentColors;
  let setRecentColors;
  ([recentColors, setRecentColors] = t21);
  if ($[5] !== t21) {
    $[5] = t21;
    $[6] = recentColors;
    $[7] = setRecentColors;
  } else {
    recentColors = $[6];
    setRecentColors = $[7];
  }
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    t28 = null;
    $[8] = t128;
    $[9] = t28;
  } else {
    t128 = $[8];
    t28 = $[9];
  }
  let popoverRef = t128;
  popoverRef = useRef(t28);
  if ($[10] === Symbol.for("react.memo_cache_sentinel")) {
    t32 = () => {
      let handleClickOutside;
      let t1 = (e) => {
        let t1;
        t1 = popoverRef.current;
        t1 = !popoverRef.current.contains(e.target);
        if (t1) {
          let t14 = setIsOpen(false);
        }
        return undefined;
      };
      handleClickOutside = t1;
      let t5 = document.addEventListener("mousedown", handleClickOutside);
      let t6 = () => {
        return document.removeEventListener("mousedown", handleClickOutside);
      };
      return t6;
    };
    t33 = [];
    $[10] = t32;
    $[11] = t33;
  } else {
    t32 = $[10];
    t33 = $[11];
  }
  let t34 = useEffect(t32, t33);
  let t39 = (c) => {
    let t3 = (prev) => {
      let next;
      let t5 = (x) => {
        return x !== c;
      };
      next = [c, ...prev.filter(t5)];
      return next.slice(0, 5);
    };
    let t4 = setRecentColors(t3);
    return undefined;
  };
  let t41 = useCallback(t39, []);
  if ($[12] !== t41 || $[13] !== onChange) {
    addToRecent = t41;
    t46 = (c) => {
      let t4 = onChange(c);
      let t8 = addToRecent(c);
      let t12 = setIsOpen(false);
      return undefined;
    };
    t49 = [onChange, addToRecent];
    $[12] = t41;
    $[13] = onChange;
    $[14] = addToRecent;
    $[15] = t129;
    $[16] = t46;
    $[17] = t49;
  } else {
    addToRecent = $[14];
    t129 = $[15];
    t46 = $[16];
    t49 = $[17];
  }
  let handlePresetClick = t129;
  let t50 = useCallback(t46, t49);
  if ($[18] !== t50 || $[19] !== customColor || $[20] !== onChange) {
    t130 = t50;
    t55 = () => {
      if (/^#[0-9a-fA-F]{6}$/.test(customColor)) {
        let t7 = onChange(customColor);
        let t11 = addToRecent(customColor);
        let t15 = setIsOpen(false);
      }
      return undefined;
    };
    t59 = [customColor, onChange, addToRecent];
    $[18] = t50;
    $[19] = customColor;
    $[20] = onChange;
    $[21] = t130;
    $[22] = t131;
    $[23] = t55;
    $[24] = t59;
  } else {
    t130 = $[21];
    t131 = $[22];
    t55 = $[23];
    t59 = $[24];
  }
  handlePresetClick = t130;
  let handleCustomSubmit = t131;
  let t60 = useCallback(t55, t59);
  if ($[25] !== t60) {
    t132 = t60;
    $[25] = t60;
    $[26] = t132;
  } else {
    t132 = $[26];
  }
  handleCustomSubmit = t132;
  if ($[27] !== presets) {
    t66 = () => {
      let rows;
      rows = [];
      let i;
      i = 0;
      if (i < presets.length) {
        let t16 = rows.push(presets.slice(i, i + 4));
        i = i + 4;
      } else {
        return rows;
      }
    };
    t68 = [presets];
    $[27] = presets;
    $[28] = t133;
    $[29] = t66;
    $[30] = t68;
  } else {
    t133 = $[28];
    t66 = $[29];
    t68 = $[30];
  }
  let groupedPresets = t133;
  let t69 = useMemo(t66, t68);
  if ($[31] !== t69 || $[32] !== color) {
    t134 = t69;
    t71 = "div";
    t72 = "relative inline-block";
    let t75 = () => {
      return setIsOpen(!isOpen);
    };
    t80 = <button onClick={t75} className="w-8 h-8 rounded border-2" style={{ backgroundColor: color }} aria-label="Pick color" />;
    $[31] = t69;
    $[32] = color;
    $[33] = t134;
    $[34] = t71;
    $[35] = t72;
    $[36] = t80;
    $[37] = t81;
  } else {
    t134 = $[33];
    t71 = $[34];
    t72 = $[35];
    t80 = $[36];
    t81 = $[37];
  }
  groupedPresets = t134;
  t81 = isOpen;
  if ($[38] === Symbol.for("react.memo_cache_sentinel")) {
    $[38] = t85;
  } else {
    t85 = $[38];
  }
  t85 = recentColors.length > 0;
  let t99 = (c) => {
    let t3 = () => {
      return handlePresetClick(c);
    };
    return <button key={c} onClick={t3} className="w-6 h-6 rounded border" style={{ backgroundColor: c }} />;
  };
  t85 = <div className="mb-2"><span className="text-xs text-gray-500">Recent</span><div className="flex gap-1 mt-1">{recentColors.map(t99)}</div></div>;
  let t106 = (row, i) => {
    let t6 = (c) => {
      let t3 = () => {
        return handlePresetClick(c);
      };
      let t8;
      if (c === color) {
        t8 = "ring-2 ring-blue-500";
      } else {
        t8 = "";
      }
      return <button key={c} onClick={t3} className={`w-6 h-6 rounded border ${t8}`} style={{ backgroundColor: c }} />;
    };
    return <div key={i} className="flex gap-1">{row.map(t6)}</div>;
  };
  let t109;
  t109 = showCustom;
  let t116 = (e) => {
    return setCustomColor(e.target.value);
  };
  t109 = <div className="mt-2 flex gap-1"><input type="text" value={customColor} onChange={t116} placeholder="#000000" className="w-20 text-xs border rounded px-1" /><button onClick={handleCustomSubmit} className="text-xs px-2 bg-blue-500 text-white rounded">\n                Apply\n              </button></div>;
  t81 = <div className="absolute z-50 mt-1 p-3 bg-white rounded-lg shadow-lg border">{t85}<div className="space-y-1">{groupedPresets.map(t106)}</div>{t109}</div>;
  return <div className={t72} ref={popoverRef}>{t80}{t81}</div>;
}

