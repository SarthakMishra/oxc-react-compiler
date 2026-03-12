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
  const $ = _c(15);
  let activeTool;
  let onToolChange;
  let locked;
  let onLockToggle;
  if ($[0] !== activeTool || $[1] !== onToolChange || $[2] !== locked || $[3] !== onLockToggle) {
    $[0] = activeTool;
    $[1] = onToolChange;
    $[2] = locked;
    $[3] = onLockToggle;
  } else {
  }
  ({ activeTool, onToolChange, locked, onLockToggle } = t0);
  const t85 = useState;
  const t86 = null;
  const t87 = t85(t86);
  let showTooltip;
  let setShowTooltip;
  if ($[4] !== showTooltip || $[5] !== setShowTooltip) {
    $[4] = showTooltip;
    $[5] = setShowTooltip;
  } else {
  }
  ([showTooltip, setShowTooltip] = t87);
  let activeIndex;
  if ($[6] !== activeIndex) {
    $[6] = activeIndex;
  } else {
  }
  let handleToolClick;
  if ($[7] !== useMemo || $[8] !== activeTool || $[9] !== activeIndex || $[10] !== handleToolClick) {
    const t92 = useMemo;
    const t93 = () => {
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
    const t94 = activeTool;
    const t95 = [t94];
    const t96 = t92(t93, t95);
    activeIndex = t96;
    $[7] = useMemo;
    $[8] = activeTool;
    $[9] = activeIndex;
    $[10] = handleToolClick;
  } else {
  }
  if ($[11] !== useCallback || $[12] !== onToolChange || $[13] !== handleToolClick || $[14] !== locked) {
    const t99 = useCallback;
    const t100 = (tool) => {
      const t2 = onToolChange;
      const t4 = tool;
      const t5 = t2(t4);
      const t6 = undefined;
      return t6;
    };
    const t101 = onToolChange;
    const t102 = [t101];
    const t103 = t99(t100, t102);
    handleToolClick = t103;
    const t105 = "div";
    const t106 = "flex items-center gap-1 p-1 bg-white rounded-lg shadow";
    const t107 = TOOLS;
    const t108 = (tool) => {
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
      const t14 = "bg-blue-100";
      const t15 = "hover:bg-gray-100";
      const t18 = tool;
      const t19 = t18.label;
      const t21 = showTooltip;
      const t23 = tool;
      const t24 = t23.id;
      const t25 = t21 === t24;
      const t26 = "span";
      const t27 = "tooltip";
      const t29 = tool;
      const t30 = t29.label;
      const t31 = " (";
      const t33 = tool;
      const t34 = t33.shortcut;
      const t35 = ")";
      const t36 = _jsxs(t26, { className: t27, children: [t30, t31, t34, t35] });
      const t38 = _jsxs(t1, { key: t4, onClick: t5, onMouseEnter: t6, onMouseLeave: t7, className: t16, children: [t19, t37] });
      return t38;
    };
    const t109 = t107.map(t108);
    const t110 = "div";
    const t111 = "border-l mx-1";
    const t112 = _jsx(t110, { className: t111 });
    const t113 = "button";
    const t114 = onLockToggle;
    const t115 = locked;
    $[11] = useCallback;
    $[12] = onToolChange;
    $[13] = handleToolClick;
    $[14] = locked;
  } else {
  }
}

