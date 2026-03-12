import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// S tier - Inspired by excalidraw toolbar
import { useState, useCallback, useMemo } from 'react';

type Tool = 'select' | 'rectangle' | 'ellipse' | 'arrow' | 'text' | 'eraser';

interface ToolbarProps {
  activeTool: Tool;
  onToolChange: (tool: Tool) => void;
  locked: boolean;
  onLockToggle: () => void;
}

const TOOLS: { id: Tool; label: string; shortcut: string }[] = [
  { id: 'select', label: 'Select', shortcut: 'V' },
  { id: 'rectangle', label: 'Rectangle', shortcut: 'R' },
  { id: 'ellipse', label: 'Ellipse', shortcut: 'O' },
  { id: 'arrow', label: 'Arrow', shortcut: 'A' },
  { id: 'text', label: 'Text', shortcut: 'T' },
  { id: 'eraser', label: 'Eraser', shortcut: 'E' },
];

export function Toolbar(t0) {
  const $ = _c(24);
  const { activeTool, onToolChange, locked, onLockToggle } = t0;
  if ($[0] !== activeTool || $[1] !== onToolChange || $[2] !== locked || $[3] !== onLockToggle) {
    $[0] = activeTool;
    $[1] = onToolChange;
    $[2] = locked;
    $[3] = onLockToggle;
  } else {
  }
  const t91 = useState;
  const t92 = null;
  const t93 = t91(t92);
  let showTooltip;
  let setShowTooltip;
  if ($[4] !== showTooltip || $[5] !== setShowTooltip) {
    $[4] = showTooltip;
    $[5] = setShowTooltip;
  } else {
  }
  ([showTooltip, setShowTooltip] = t93);
  let activeIndex;
  if ($[6] !== activeIndex) {
    $[6] = activeIndex;
  } else {
  }
  let handleToolClick;
  if ($[7] !== useMemo || $[8] !== activeTool || $[9] !== activeIndex || $[10] !== handleToolClick) {
    const t98 = useMemo;
    const t99 = () => {
      const t1 = TOOLS;
      const t2 = (t) => {
        const t2 = t;
        const t3 = t2.id;
        const t5 = activeTool;
        const t6 = t3 === t5;
        return t6;
      };
      const t3 = t1.findIndex(t2);
      return t3;
    };
    const t100 = activeTool;
    const t101 = [t100];
    const t102 = t98(t99, t101);
    activeIndex = t102;
    $[7] = useMemo;
    $[8] = activeTool;
    $[9] = activeIndex;
    $[10] = handleToolClick;
  } else {
  }
  let t57;
  if ($[11] !== useCallback || $[12] !== onToolChange || $[13] !== handleToolClick || $[14] !== locked || $[15] !== t57) {
    const t105 = useCallback;
    const t106 = (tool) => {
      const t2 = onToolChange;
      const t4 = tool;
      const t5 = t2(t4);
      const t6 = undefined;
      return t6;
    };
    const t107 = onToolChange;
    const t108 = [t107];
    const t109 = t105(t106, t108);
    handleToolClick = t109;
    const t111 = "div";
    const t112 = "flex items-center gap-1 p-1 bg-white rounded-lg shadow";
    const t113 = TOOLS;
    const t114 = (tool) => {
      const t1 = "button";
      const t3 = tool;
      const t4 = t3.id;
      const t5 = () => {
        const t1 = handleToolClick;
        const t3 = tool;
        const t4 = t3.id;
        const t5 = t1(t4);
        return t5;
      };
      const t6 = () => {
        const t1 = setShowTooltip;
        const t3 = tool;
        const t4 = t3.id;
        const t5 = t1(t4);
        return t5;
      };
      const t7 = () => {
        const t1 = setShowTooltip;
        const t2 = null;
        const t3 = t1(t2);
        return t3;
      };
      const t9 = activeTool;
      const t11 = tool;
      const t12 = t11.id;
      const t13 = t9 === t12;
      let t14;
      if (t13) {
        const t16 = "bg-blue-100";
        t14 = t16;
      } else {
        const t18 = "hover:bg-gray-100";
        t14 = t18;
      }
      const t21 = tool;
      const t22 = t21.label;
      let t23;
      const t26 = showTooltip;
      const t28 = tool;
      const t29 = t28.id;
      const t30 = t26 === t29;
      t23 = t30;
      const t32 = "span";
      const t33 = "tooltip";
      const t35 = tool;
      const t36 = t35.label;
      const t37 = " (";
      const t39 = tool;
      const t40 = t39.shortcut;
      const t41 = ")";
      const t42 = _jsxs(t32, { className: t33, children: [t36, t37, t40, t41] });
      t23 = t42;
      const t44 = _jsxs(t1, { key: t4, onClick: t5, onMouseEnter: t6, onMouseLeave: t7, className: t14, children: [t22, t23] });
      return t44;
    };
    const t115 = t113.map(t114);
    const t116 = "div";
    const t117 = "border-l mx-1";
    const t118 = _jsx(t116, { className: t117 });
    const t119 = "button";
    const t120 = onLockToggle;
    const t121 = locked;
    $[11] = useCallback;
    $[12] = onToolChange;
    $[13] = handleToolClick;
    $[14] = locked;
    $[15] = t57;
  } else {
  }
  if (t121) {
    const t149 = "bg-blue-100";
    t57 = t149;
  } else {
    const t151 = "";
    t57 = t151;
  }
  if ($[16] !== locked) {
    const t127 = locked;
    $[16] = locked;
  } else {
  }
  let t65;
  if (t127) {
    const t147 = "🔒";
    t65 = t147;
  } else {
    const t129 = "🔓";
    t65 = t129;
  }
  let t146;
  if ($[17] !== t57 || $[18] !== t65 || $[19] !== activeIndex || $[20] !== TOOLS || $[21] !== TOOLS || $[22] !== onLockToggle) {
    const t135 = _jsx(t119, { onClick: t120, className: t57, children: t65 });
    const t136 = "span";
    const t137 = "text-xs text-gray-500";
    const t138 = "Tool ";
    const t139 = activeIndex;
    const t140 = 1;
    const t141 = t139 + t140;
    const t142 = "/";
    const t143 = TOOLS;
    const t144 = t143.length;
    const t145 = _jsxs(t136, { className: t137, children: [t138, t141, t142, t144] });
    t146 = _jsxs(t111, { className: t112, children: [t115, t118, t135, t145] });
    $[23] = t146;
    $[17] = t57;
    $[18] = t65;
    $[19] = activeIndex;
    $[20] = TOOLS;
    $[21] = TOOLS;
    $[22] = onLockToggle;
  } else {
    t146 = $[23];
  }
  return t146;
}

