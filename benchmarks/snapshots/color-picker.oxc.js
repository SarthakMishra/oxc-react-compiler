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
  const $ = _c(26);
  let color;
  let onChange;
  let presets;
  let showCustom;
  if ($[0] !== color || $[1] !== onChange || $[2] !== presets || $[3] !== showCustom) {
    $[0] = color;
    $[1] = onChange;
    $[2] = presets;
    $[3] = showCustom;
  } else {
  }
  ({ color, onChange, presets, showCustom } = t0);
  const t174 = useState;
  const t175 = false;
  const t176 = t174(t175);
  let isOpen;
  let setIsOpen;
  if ($[4] !== isOpen || $[5] !== setIsOpen) {
    $[4] = isOpen;
    $[5] = setIsOpen;
  } else {
  }
  ([isOpen, setIsOpen] = t176);
  let customColor;
  let setCustomColor;
  if ($[6] !== useState || $[7] !== color || $[8] !== customColor || $[9] !== setCustomColor) {
    const t180 = useState;
    const t181 = color;
    const t182 = t180(t181);
    ([customColor, setCustomColor] = t182);
    $[6] = useState;
    $[7] = color;
    $[8] = customColor;
    $[9] = setCustomColor;
  } else {
  }
  const t186 = useState;
  const t187 = [];
  const t188 = t186(t187);
  let recentColors;
  let setRecentColors;
  if ($[10] !== recentColors || $[11] !== setRecentColors) {
    $[10] = recentColors;
    $[11] = setRecentColors;
  } else {
  }
  ([recentColors, setRecentColors] = t188);
  let popoverRef;
  if ($[12] !== popoverRef) {
    $[12] = popoverRef;
  } else {
  }
  const t193 = useRef;
  const t194 = null;
  const t195 = t193(t194);
  popoverRef = t195;
  const t197 = useEffect;
  const t198 = () => {
    let handleClickOutside;
    const t2 = (e) => {
      const t2 = popoverRef;
      const t3 = t2.current;
      const t5 = popoverRef;
      const t6 = t5.current;
      const t8 = e;
      const t9 = t8.target;
      const t10 = t6.contains(t9);
      const t11 = !t10;
      const t14 = setIsOpen;
      const t15 = false;
      const t16 = t14(t15);
      const t17 = undefined;
      return t17;
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
    const t10 = undefined;
    return t10;
  };
  const t199 = [];
  const t200 = t197(t198, t199);
  let addToRecent;
  if ($[13] !== addToRecent) {
    $[13] = addToRecent;
  } else {
  }
  const t202 = useCallback;
  const t203 = (c) => {
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
      const t16 = undefined;
      return t16;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t204 = [];
  const t205 = t202(t203, t204);
  addToRecent = t205;
  let handlePresetClick;
  if ($[14] !== handlePresetClick) {
    $[14] = handlePresetClick;
  } else {
  }
  let handleCustomSubmit;
  if ($[15] !== useCallback || $[16] !== onChange || $[17] !== addToRecent || $[18] !== handlePresetClick || $[19] !== handleCustomSubmit) {
    const t208 = useCallback;
    const t209 = (c) => {
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
    const t210 = onChange;
    const t211 = addToRecent;
    const t212 = [t210, t211];
    const t213 = t208(t209, t212);
    handlePresetClick = t213;
    $[15] = useCallback;
    $[16] = onChange;
    $[17] = addToRecent;
    $[18] = handlePresetClick;
    $[19] = handleCustomSubmit;
  } else {
  }
  let groupedPresets;
  if ($[20] !== useCallback || $[21] !== customColor || $[22] !== onChange || $[23] !== addToRecent || $[24] !== handleCustomSubmit || $[25] !== groupedPresets) {
    const t216 = useCallback;
    const t217 = () => {
      const t0 = /^#[0-9a-fA-F]{6}$/;
      const t2 = customColor;
      const t3 = t0.test(t2);
      const t5 = onChange;
      const t7 = customColor;
      const t8 = t5(t7);
      const t10 = addToRecent;
      const t12 = customColor;
      const t13 = t10(t12);
      const t15 = setIsOpen;
      const t16 = false;
      const t17 = t15(t16);
      const t18 = undefined;
      return t18;
    };
    const t218 = customColor;
    const t219 = onChange;
    const t220 = addToRecent;
    const t221 = [t218, t219, t220];
    const t222 = t216(t217, t221);
    handleCustomSubmit = t222;
    $[20] = useCallback;
    $[21] = customColor;
    $[22] = onChange;
    $[23] = addToRecent;
    $[24] = handleCustomSubmit;
    $[25] = groupedPresets;
  } else {
  }
  const t225 = useMemo;
  const t226 = () => {
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
    const t33 = rows;
    return t33;
    const t34 = undefined;
    return t34;
  };
  const t227 = presets;
  const t228 = [t227];
  const t229 = t225(t226, t228);
  groupedPresets = t229;
  const t231 = "div";
  const t232 = "relative inline-block";
  const t233 = popoverRef;
  const t234 = "button";
  const t235 = () => {
    const t1 = setIsOpen;
    const t3 = isOpen;
    const t4 = !t3;
    const t5 = t1(t4);
    return t5;
  };
  const t236 = "w-8 h-8 rounded border-2";
  const t237 = color;
  const t238 = { backgroundColor: t237 };
  const t239 = "Pick color";
  const t240 = _jsx(t234, { onClick: t235, className: t236, style: t238, aria-label: t239 });
}

