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
  const $ = _c(23);
  const { color, onChange, presets, showCustom } = t0;
  let customColor;
  let setCustomColor;
  let recentColors;
  let setRecentColors;
  let popoverRef;
  if ($[0] !== color || $[1] !== useState || $[2] !== useState) {
    $[0] = color;
    $[1] = useState;
    $[2] = useState;
  }
  popoverRef = useRef(null);
  let addToRecent;
  if ($[3] !== useEffect) {
    const t207 = () => {
      const t2 = (e) => {
        t1 = popoverRef.current;
        t1 = !popoverRef.current.contains(e.target);
        if (t1) {
          const t19 = setIsOpen(false);
        }
        return undefined;
      };
      const handleClickOutside = t2;
      const t8 = document.addEventListener("mousedown", handleClickOutside);
      const t9 = () => {
        return document.removeEventListener("mousedown", handleClickOutside);
      };
      return t9;
    };
    const t209 = useEffect(t207, []);
    $[3] = useEffect;
  }
  let handlePresetClick;
  if ($[4] !== useCallback) {
    const t212 = (c) => {
      const t3 = (prev) => {
        const t7 = (x) => {
          return x !== c;
        };
        const next = [c, ...prev.filter(t7)];
        return next.slice(0, 5);
      };
      const t4 = setRecentColors(t3);
      return undefined;
    };
    addToRecent = useCallback(t212, []);
    $[4] = useCallback;
  }
  let handleCustomSubmit;
  if ($[5] !== addToRecent || $[6] !== onChange || $[7] !== useCallback) {
    const t218 = (c) => {
      const t5 = onChange(c);
      const t10 = addToRecent(c);
      const t14 = setIsOpen(false);
      return undefined;
    };
    handlePresetClick = useCallback(t218, [onChange, addToRecent]);
    $[5] = addToRecent;
    $[6] = onChange;
    $[7] = useCallback;
  }
  let groupedPresets;
  if ($[8] !== addToRecent || $[9] !== customColor || $[10] !== onChange || $[11] !== useCallback) {
    const t226 = () => {
      if (/^#[0-9a-fA-F]{6}$/.test(customColor)) {
        const t8 = onChange(customColor);
        const t13 = addToRecent(customColor);
        const t17 = setIsOpen(false);
      }
      return undefined;
    };
    handleCustomSubmit = useCallback(t226, [customColor, onChange, addToRecent]);
    $[8] = addToRecent;
    $[9] = customColor;
    $[10] = onChange;
    $[11] = useCallback;
  }
  const t235 = () => {
    const rows = [];
    let i = 0;
    if (i < presets.length) {
      const t25 = rows.push(presets.slice(i, i + 4));
      i = i + 4;
    } else {
      return rows;
    }
  };
  groupedPresets = useMemo(t235, [presets]);
  const t244 = () => {
    return setIsOpen(!isOpen);
  };
  let t302;
  if ($[12] !== color || $[13] !== customColor || $[14] !== handleCustomSubmit || $[15] !== isOpen || $[16] !== popoverRef || $[17] !== presets || $[18] !== recentColors || $[19] !== recentColors || $[20] !== showCustom || $[21] !== useMemo) {
    t114 = isOpen;
    $[12] = color;
    $[13] = customColor;
    $[14] = handleCustomSubmit;
    $[15] = isOpen;
    $[16] = popoverRef;
    $[17] = presets;
    $[18] = recentColors;
    $[19] = recentColors;
    $[20] = showCustom;
    $[21] = useMemo;
    $[22] = t302;
  } else {
    t302 = $[22];
  }
  t121 = recentColors.length > 0;
  const t377 = (c) => {
    const t4 = () => {
      return handlePresetClick(c);
    };
    return <button key={c} onClick={t4} className="w-6 h-6 rounded border" style={{ backgroundColor: c }} />;
  };
  t121 = <div className="mb-2"><span className="text-xs text-gray-500">Recent</span><div className="flex gap-1 mt-1">{recentColors.map(t377)}</div></div>;
  const t328 = (row, i) => {
    const t8 = (c) => {
      const t4 = () => {
        return handlePresetClick(c);
      };
      if (c === color) {
        t10 = "ring-2 ring-blue-500";
      } else {
        t10 = "";
      }
      return <button key={c} onClick={t4} className={`w-6 h-6 rounded border ${t10}`} style={{ backgroundColor: c }} />;
    };
    return <div key={i} className="flex gap-1">{row.map(t8)}</div>;
  };
  t151 = showCustom;
  const t357 = (e) => {
    return setCustomColor(e.target.value);
  };
  t151 = <div className="mt-2 flex gap-1"><input type="text" value={customColor} onChange={t357} placeholder="#000000" className="w-20 text-xs border rounded px-1" /><button onClick={handleCustomSubmit} className="text-xs px-2 bg-blue-500 text-white rounded">\n                Apply\n              </button></div>;
  t114 = <div className="absolute z-50 mt-1 p-3 bg-white rounded-lg shadow-lg border">{t121}<div className="space-y-1">{groupedPresets.map(t328)}</div>{t151}</div>;
  return <div className="relative inline-block" ref={popoverRef}><button onClick={t244} className="w-8 h-8 rounded border-2" style={{ backgroundColor: color }} aria-label="Pick color" />{t114}</div>;
}

