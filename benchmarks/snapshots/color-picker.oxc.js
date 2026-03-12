import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
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
  const $ = _c(42);
  const { color, onChange, presets, showCustom } = t0;
  if ($[0] !== color || $[1] !== onChange || $[2] !== presets || $[3] !== showCustom) {
    $[0] = color;
    $[1] = onChange;
    $[2] = presets;
    $[3] = showCustom;
  } else {
  }
  const t183 = useState;
  const t184 = false;
  const t185 = t183(t184);
  let isOpen;
  let setIsOpen;
  if ($[4] !== isOpen || $[5] !== setIsOpen) {
    $[4] = isOpen;
    $[5] = setIsOpen;
  } else {
  }
  ([isOpen, setIsOpen] = t185);
  let customColor;
  let setCustomColor;
  if ($[6] !== useState || $[7] !== color || $[8] !== customColor || $[9] !== setCustomColor) {
    const t189 = useState;
    const t190 = color;
    const t191 = t189(t190);
    ([customColor, setCustomColor] = t191);
    $[6] = useState;
    $[7] = color;
    $[8] = customColor;
    $[9] = setCustomColor;
  } else {
  }
  const t195 = useState;
  const t196 = [];
  const t197 = t195(t196);
  let recentColors;
  let setRecentColors;
  if ($[10] !== recentColors || $[11] !== setRecentColors) {
    $[10] = recentColors;
    $[11] = setRecentColors;
  } else {
  }
  ([recentColors, setRecentColors] = t197);
  let popoverRef;
  if ($[12] !== popoverRef) {
    $[12] = popoverRef;
  } else {
  }
  const t202 = useRef;
  const t203 = null;
  const t204 = t202(t203);
  popoverRef = t204;
  const t206 = useEffect;
  const t207 = () => {
    let handleClickOutside;
    const t2 = (e) => {
      let t1;
      const t4 = popoverRef;
      const t5 = t4.current;
      t1 = t5;
      const t8 = popoverRef;
      const t9 = t8.current;
      const t11 = e;
      const t12 = t11.target;
      const t13 = t9.contains(t12);
      const t14 = !t13;
      t1 = t14;
      if (t1) {
        const t17 = setIsOpen;
        const t18 = false;
        const t19 = t17(t18);
      } else {
      }
      const t20 = undefined;
      return t20;
    };
    handleClickOutside = t2;
    const t4 = document;
    const t5 = "mousedown";
    const t7 = handleClickOutside;
    const t8 = t4.addEventListener(t5, t7);
    const t9 = () => {
      const t0 = document;
      const t1 = "mousedown";
      const t3 = handleClickOutside;
      const t4 = t0.removeEventListener(t1, t3);
      return t4;
    };
    return t9;
  };
  const t208 = [];
  const t209 = t206(t207, t208);
  let addToRecent;
  if ($[13] !== addToRecent) {
    $[13] = addToRecent;
  } else {
  }
  const t211 = useCallback;
  const t212 = (c) => {
    const t2 = setRecentColors;
    const t3 = (prev) => {
      let next;
      const t4 = c;
      const t6 = prev;
      const t7 = (x) => {
        const t2 = x;
        const t4 = c;
        const t5 = t2 !== t4;
        return t5;
      };
      const t8 = t6.filter(t7);
      const t9 = [t4, ...t8];
      next = t9;
      const t12 = next;
      const t13 = 0;
      const t14 = 5;
      const t15 = t12.slice(t13, t14);
      return t15;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t213 = [];
  const t214 = t211(t212, t213);
  addToRecent = t214;
  let handlePresetClick;
  if ($[14] !== handlePresetClick) {
    $[14] = handlePresetClick;
  } else {
  }
  let handleCustomSubmit;
  if ($[15] !== useCallback || $[16] !== onChange || $[17] !== addToRecent || $[18] !== handlePresetClick || $[19] !== handleCustomSubmit) {
    const t217 = useCallback;
    const t218 = (c) => {
      const t2 = onChange;
      const t4 = c;
      const t5 = t2(t4);
      const t7 = addToRecent;
      const t9 = c;
      const t10 = t7(t9);
      const t12 = setIsOpen;
      const t13 = false;
      const t14 = t12(t13);
      const t15 = undefined;
      return t15;
    };
    const t219 = onChange;
    const t220 = addToRecent;
    const t221 = [t219, t220];
    const t222 = t217(t218, t221);
    handlePresetClick = t222;
    $[15] = useCallback;
    $[16] = onChange;
    $[17] = addToRecent;
    $[18] = handlePresetClick;
    $[19] = handleCustomSubmit;
  } else {
  }
  let groupedPresets;
  if ($[20] !== useCallback || $[21] !== customColor || $[22] !== onChange || $[23] !== addToRecent || $[24] !== handleCustomSubmit || $[25] !== groupedPresets) {
    const t225 = useCallback;
    const t226 = () => {
      const t0 = /^#[0-9a-fA-F]{6}$/;
      const t2 = customColor;
      const t3 = t0.test(t2);
      if (t3) {
        const t5 = onChange;
        const t7 = customColor;
        const t8 = t5(t7);
        const t10 = addToRecent;
        const t12 = customColor;
        const t13 = t10(t12);
        const t15 = setIsOpen;
        const t16 = false;
        const t17 = t15(t16);
      } else {
      }
      const t18 = undefined;
      return t18;
    };
    const t227 = customColor;
    const t228 = onChange;
    const t229 = addToRecent;
    const t230 = [t227, t228, t229];
    const t231 = t225(t226, t230);
    handleCustomSubmit = t231;
    $[20] = useCallback;
    $[21] = customColor;
    $[22] = onChange;
    $[23] = addToRecent;
    $[24] = handleCustomSubmit;
    $[25] = groupedPresets;
  } else {
  }
  const t234 = useMemo;
  const t235 = () => {
    let rows;
    const t2 = [];
    rows = t2;
    let i;
    const t6 = 0;
    i = t6;
    const t9 = i;
    const t11 = presets;
    const t12 = t11.length;
    const t13 = t9 < t12;
    if (t13) {
      const t15 = rows;
      const t17 = presets;
      const t19 = i;
      const t21 = i;
      const t22 = 4;
      const t23 = t21 + t22;
      const t24 = t17.slice(t19, t23);
      const t25 = t15.push(t24);
      const t26 = 4;
      const t28 = i;
      const t29 = t28 + t26;
      i = t29;
    } else {
      const t33 = rows;
      return t33;
    }
  };
  const t236 = presets;
  const t237 = [t236];
  const t238 = t234(t235, t237);
  groupedPresets = t238;
  const t240 = "div";
  const t241 = "relative inline-block";
  const t242 = popoverRef;
  const t243 = "button";
  const t244 = () => {
    const t1 = setIsOpen;
    const t3 = isOpen;
    const t4 = !t3;
    const t5 = t1(t4);
    return t5;
  };
  const t245 = "w-8 h-8 rounded border-2";
  const t246 = color;
  const t247 = { backgroundColor: t246 };
  const t248 = "Pick color";
  const t249 = _jsx(t243, { onClick: t244, className: t245, style: t247, "aria-label": t248 });
  let t114;
  let t302;
  if ($[26] !== t114 || $[27] !== recentColors || $[28] !== t121 || $[29] !== recentColors || $[30] !== groupedPresets || $[31] !== showCustom || $[32] !== t151 || $[33] !== customColor || $[34] !== handleCustomSubmit || $[35] !== useMemo || $[36] !== presets || $[37] !== groupedPresets || $[38] !== popoverRef || $[39] !== color || $[40] !== isOpen) {
    const t251 = isOpen;
    t114 = t251;
    $[41] = t302;
    $[26] = t114;
    $[27] = recentColors;
    $[28] = t121;
    $[29] = recentColors;
    $[30] = groupedPresets;
    $[31] = showCustom;
    $[32] = t151;
    $[33] = customColor;
    $[34] = handleCustomSubmit;
    $[35] = useMemo;
    $[36] = presets;
    $[37] = groupedPresets;
    $[38] = popoverRef;
    $[39] = color;
    $[40] = isOpen;
  } else {
    t302 = $[41];
  }
  const t303 = "div";
  const t304 = "absolute z-50 mt-1 p-3 bg-white rounded-lg shadow-lg border";
  let t121;
  const t306 = recentColors;
  const t307 = t306.length;
  const t308 = 0;
  const t309 = t307 > t308;
  t121 = t309;
  const t368 = "div";
  const t369 = "mb-2";
  const t370 = "span";
  const t371 = "text-xs text-gray-500";
  const t372 = "Recent";
  const t373 = _jsx(t370, { className: t371, children: t372 });
  const t374 = "div";
  const t375 = "flex gap-1 mt-1";
  const t376 = recentColors;
  const t377 = (c) => {
    const t1 = "button";
    const t3 = c;
    const t4 = () => {
      const t1 = handlePresetClick;
      const t3 = c;
      const t4 = t1(t3);
      return t4;
    };
    const t5 = "w-6 h-6 rounded border";
    const t7 = c;
    const t8 = { backgroundColor: t7 };
    const t9 = _jsx(t1, { key: t3, onClick: t4, className: t5, style: t8 });
    return t9;
  };
  const t378 = t376.map(t377);
  const t379 = _jsx(t374, { className: t375, children: t378 });
  const t380 = _jsxs(t368, { className: t369, children: [t373, t379] });
  t121 = t380;
  const t325 = "div";
  const t326 = "space-y-1";
  const t327 = groupedPresets;
  const t328 = (row, i) => {
    const t2 = "div";
    const t4 = i;
    const t5 = "flex gap-1";
    const t7 = row;
    const t8 = (c) => {
      const t1 = "button";
      const t3 = c;
      const t4 = () => {
        const t1 = handlePresetClick;
        const t3 = c;
        const t4 = t1(t3);
        return t4;
      };
      const t6 = c;
      const t8 = color;
      const t9 = t6 === t8;
      let t10;
      if (t9) {
        const t12 = "ring-2 ring-blue-500";
        t10 = t12;
      } else {
        const t14 = "";
        t10 = t14;
      }
      const t16 = `w-6 h-6 rounded border ${t10}`;
      const t18 = c;
      const t19 = { backgroundColor: t18 };
      const t20 = _jsx(t1, { key: t3, onClick: t4, className: t16, style: t19 });
      return t20;
    };
    const t9 = t7.map(t8);
    const t10 = _jsx(t2, { key: t4, className: t5, children: t9 });
    return t10;
  };
  const t329 = t327.map(t328);
  const t330 = _jsx(t325, { className: t326, children: t329 });
  let t151;
  const t332 = showCustom;
  t151 = t332;
  const t352 = "div";
  const t353 = "mt-2 flex gap-1";
  const t354 = "input";
  const t355 = "text";
  const t356 = customColor;
  const t357 = (e) => {
    const t2 = setCustomColor;
    const t4 = e;
    const t5 = t4.target;
    const t6 = t5.value;
    const t7 = t2(t6);
    return t7;
  };
  const t358 = "#000000";
  const t359 = "w-20 text-xs border rounded px-1";
  const t360 = _jsx(t354, { type: t355, value: t356, onChange: t357, placeholder: t358, className: t359 });
  const t361 = "button";
  const t362 = handleCustomSubmit;
  const t363 = "text-xs px-2 bg-blue-500 text-white rounded";
  const t364 = "\n                Apply\n              ";
  const t365 = _jsx(t361, { onClick: t362, className: t363, children: t364 });
  const t366 = _jsxs(t352, { className: t353, children: [t360, t365] });
  t151 = t366;
  const t350 = _jsxs(t303, { className: t304, children: [t121, t330, t151] });
  t114 = t350;
  t302 = _jsxs(t240, { className: t241, ref: t242, children: [t249, t114] });
  return t302;
}

