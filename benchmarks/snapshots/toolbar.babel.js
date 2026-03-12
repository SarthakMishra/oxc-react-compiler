import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by excalidraw toolbar
import { useState, useCallback, useMemo } from 'react';
import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
const TOOLS = [{
  id: 'select',
  label: 'Select',
  shortcut: 'V'
}, {
  id: 'rectangle',
  label: 'Rectangle',
  shortcut: 'R'
}, {
  id: 'ellipse',
  label: 'Ellipse',
  shortcut: 'O'
}, {
  id: 'arrow',
  label: 'Arrow',
  shortcut: 'A'
}, {
  id: 'text',
  label: 'Text',
  shortcut: 'T'
}, {
  id: 'eraser',
  label: 'Eraser',
  shortcut: 'E'
}];
export function Toolbar(t0) {
  const $ = _c(19);
  const {
    activeTool,
    onToolChange,
    locked,
    onLockToggle
  } = t0;
  const [showTooltip, setShowTooltip] = useState(null);
  let t1;
  if ($[0] !== activeTool) {
    t1 = t => t.id === activeTool;
    $[0] = activeTool;
    $[1] = t1;
  } else {
    t1 = $[1];
  }
  const activeIndex = TOOLS.findIndex(t1);
  let t2;
  if ($[2] !== onToolChange) {
    t2 = tool => {
      onToolChange(tool);
    };
    $[2] = onToolChange;
    $[3] = t2;
  } else {
    t2 = $[3];
  }
  const handleToolClick = t2;
  let t3;
  if ($[4] !== activeTool || $[5] !== handleToolClick || $[6] !== showTooltip) {
    t3 = TOOLS.map(tool_0 => /*#__PURE__*/_jsxs("button", {
      onClick: () => handleToolClick(tool_0.id),
      onMouseEnter: () => setShowTooltip(tool_0.id),
      onMouseLeave: () => setShowTooltip(null),
      className: activeTool === tool_0.id ? "bg-blue-100" : "hover:bg-gray-100",
      children: [tool_0.label, showTooltip === tool_0.id && /*#__PURE__*/_jsxs("span", {
        className: "tooltip",
        children: [tool_0.label, " (", tool_0.shortcut, ")"]
      })]
    }, tool_0.id));
    $[4] = activeTool;
    $[5] = handleToolClick;
    $[6] = showTooltip;
    $[7] = t3;
  } else {
    t3 = $[7];
  }
  let t4;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    t4 = /*#__PURE__*/_jsx("div", {
      className: "border-l mx-1"
    });
    $[8] = t4;
  } else {
    t4 = $[8];
  }
  const t5 = locked ? "bg-blue-100" : "";
  const t6 = locked ? "\uD83D\uDD12" : "\uD83D\uDD13";
  let t7;
  if ($[9] !== onLockToggle || $[10] !== t5 || $[11] !== t6) {
    t7 = /*#__PURE__*/_jsx("button", {
      onClick: onLockToggle,
      className: t5,
      children: t6
    });
    $[9] = onLockToggle;
    $[10] = t5;
    $[11] = t6;
    $[12] = t7;
  } else {
    t7 = $[12];
  }
  const t8 = activeIndex + 1;
  let t9;
  if ($[13] !== t8) {
    t9 = /*#__PURE__*/_jsxs("span", {
      className: "text-xs text-gray-500",
      children: ["Tool ", t8, "/", TOOLS.length]
    });
    $[13] = t8;
    $[14] = t9;
  } else {
    t9 = $[14];
  }
  let t10;
  if ($[15] !== t3 || $[16] !== t7 || $[17] !== t9) {
    t10 = /*#__PURE__*/_jsxs("div", {
      className: "flex items-center gap-1 p-1 bg-white rounded-lg shadow",
      children: [t3, t4, t7, t9]
    });
    $[15] = t3;
    $[16] = t7;
    $[17] = t9;
    $[18] = t10;
  } else {
    t10 = $[18];
  }
  return t10;
}