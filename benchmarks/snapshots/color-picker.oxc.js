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
  const $ = _c(53);
  const { color, onChange, presets, showCustom } = t0;
  let t81;
  let t127;
  let t128;
  let t66;
  let t68;
  let t129;
  let t28;
  let t8;
  if ($[0] !== t80 || $[1] !== t85 || $[2] !== t109 || $[3] !== presets) {
    t8 = false;
    $[0] = t80;
    $[1] = t85;
    $[2] = t109;
    $[3] = presets;
    $[4] = t81;
    $[5] = t127;
    $[6] = t128;
    $[7] = t66;
    $[8] = t68;
    $[9] = t129;
    $[10] = t28;
    $[11] = t8;
  } else {
    t81 = $[4];
    t127 = $[5];
    t128 = $[6];
    t66 = $[7];
    t68 = $[8];
    t129 = $[9];
    t28 = $[10];
    t8 = $[11];
  }
  const groupedPresets = t128;
  const popoverRef = t129;
  let t130;
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    $[12] = t130;
  } else {
    t130 = $[12];
  }
  const isOpen = t130;
  let setIsOpen;
  const t15 = useState(color);
  let customColor;
  let setCustomColor;
  if ($[13] !== t15) {
    $[13] = t15;
  }
  let t20;
  if ($[14] !== t21) {
    $[14] = t21;
    $[15] = t20;
  } else {
    t20 = $[15];
  }
  let popoverRef;
  const t29 = useRef(null);
  let t131;
  if ($[16] === Symbol.for("react.memo_cache_sentinel")) {
    t131 = t29;
    $[16] = t131;
  } else {
    t131 = $[16];
  }
  popoverRef = t131;
  let t32;
  let t33;
  if ($[17] === Symbol.for("react.memo_cache_sentinel")) {
    t32 = () => {
      let handleClickOutside;
      const t1 = (e) => {
        let t1;
        t1 = popoverRef.current;
        t1 = !popoverRef.current.contains(e.target);
        if (t1) {
          const t14 = setIsOpen(false);
        }
        return undefined;
      };
      handleClickOutside = t1;
      const t5 = document.addEventListener("mousedown", handleClickOutside);
      const t6 = () => {
        return document.removeEventListener("mousedown", handleClickOutside);
      };
      return t6;
    };
    t33 = [];
    $[17] = t32;
    $[18] = t33;
  } else {
    t32 = $[17];
    t33 = $[18];
  }
  const t34 = useEffect(t32, t33);
  let addToRecent;
  const t39 = (c) => {
    const t3 = (prev) => {
      let next;
      const t5 = (x) => {
        return x !== c;
      };
      next = [c, ...prev.filter(t5)];
      return next.slice(0, 5);
    };
    const t4 = setRecentColors(t3);
    return undefined;
  };
  const t41 = useCallback(t39, []);
  let t132;
  let t46;
  let t49;
  if ($[19] !== t41 || $[20] !== onChange) {
    addToRecent = t41;
    t46 = (c) => {
      const t4 = onChange(c);
      const t8 = addToRecent(c);
      const t12 = setIsOpen(false);
      return undefined;
    };
    t49 = [onChange, addToRecent];
    $[19] = t41;
    $[20] = onChange;
    $[21] = addToRecent;
    $[22] = t132;
    $[23] = t46;
    $[24] = t49;
  } else {
    addToRecent = $[21];
    t132 = $[22];
    t46 = $[23];
    t49 = $[24];
  }
  const handlePresetClick = t132;
  const t50 = useCallback(t46, t49);
  let t134;
  let t133;
  let t55;
  let t59;
  if ($[25] !== t50 || $[26] !== handleCustomSubmit || $[27] !== onChange || $[28] !== showCustom) {
    t133 = t50;
    t55 = () => {
      if (/^#[0-9a-fA-F]{6}$/.test(customColor)) {
        const t7 = onChange(customColor);
        const t11 = addToRecent(customColor);
        const t15 = setIsOpen(false);
      }
      return undefined;
    };
    t59 = [customColor, onChange, addToRecent];
    $[25] = t50;
    $[26] = handleCustomSubmit;
    $[27] = onChange;
    $[28] = showCustom;
    $[29] = t133;
    $[30] = t134;
    $[31] = t55;
    $[32] = t59;
  } else {
    t133 = $[29];
    t134 = $[30];
    t55 = $[31];
    t59 = $[32];
  }
  const handlePresetClick = t133;
  const handleCustomSubmit = t134;
  const t60 = useCallback(t55, t59);
  let t135;
  if ($[33] !== t60) {
    t135 = t60;
    $[33] = t60;
    $[34] = t135;
  } else {
    t135 = $[34];
  }
  const handleCustomSubmit = t135;
  let groupedPresets;
  t66 = () => {
    let rows;
    rows = [];
    let i;
    i = 0;
    if (i < presets.length) {
      const t16 = rows.push(presets.slice(i, i + 4));
      i = i + 4;
    } else {
      return rows;
    }
  };
  const t69 = useMemo(t66, [presets]);
  let t136;
  let t71;
  let t72;
  let t80;
  if ($[35] !== t69 || $[36] !== color) {
    t136 = t69;
    t71 = "div";
    t72 = "relative inline-block";
    const t75 = () => {
      return setIsOpen(!isOpen);
    };
    t80 = <button onClick={t75} className="w-8 h-8 rounded border-2" style={{ backgroundColor: color }} aria-label="Pick color" />;
    $[35] = t69;
    $[36] = color;
    $[37] = t136;
    $[38] = t71;
    $[39] = t72;
    $[40] = t80;
    $[41] = t81;
  } else {
    t136 = $[37];
    t71 = $[38];
    t72 = $[39];
    t80 = $[40];
    t81 = $[41];
  }
  groupedPresets = t136;
  t81 = isOpen;
  let t85;
  let t109;
  let t137;
  let t138;
  let t39;
  let t40;
  if ($[42] !== t50 || $[43] !== addToRecent || $[44] !== customColor || $[45] !== onChange || $[46] !== showCustom) {
    $[42] = t50;
    $[43] = addToRecent;
    $[44] = customColor;
    $[45] = onChange;
    $[46] = showCustom;
    $[47] = t85;
    $[48] = t109;
    $[49] = t137;
    $[50] = t138;
    $[51] = t39;
    $[52] = t40;
  } else {
    t85 = $[47];
    t109 = $[48];
    t137 = $[49];
    t138 = $[50];
    t39 = $[51];
    t40 = $[52];
  }
  customColor = t137;
  addToRecent = t138;
  t85 = recentColors.length > 0;
  const t99 = (c) => {
    const t3 = () => {
      return handlePresetClick(c);
    };
    return <button key={c} onClick={t3} className="w-6 h-6 rounded border" style={{ backgroundColor: c }} />;
  };
  t85 = <div className="mb-2"><span className="text-xs text-gray-500">Recent</span><div className="flex gap-1 mt-1">{recentColors.map(t99)}</div></div>;
  const t106 = (row, i) => {
    const t6 = (c) => {
      const t3 = () => {
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
  t109 = showCustom;
  const t116 = (e) => {
    return setCustomColor(e.target.value);
  };
  t109 = <div className="mt-2 flex gap-1"><input type="text" value={customColor} onChange={t116} placeholder="#000000" className="w-20 text-xs border rounded px-1" /><button onClick={handleCustomSubmit} className="text-xs px-2 bg-blue-500 text-white rounded">\n                Apply\n              </button></div>;
  t81 = <div className="absolute z-50 mt-1 p-3 bg-white rounded-lg shadow-lg border">{t85}<div className="space-y-1">{groupedPresets.map(t106)}</div>{t109}</div>;
  return <t71 className={t72} ref={popoverRef}>{t80}{t81}</t71>;
}

