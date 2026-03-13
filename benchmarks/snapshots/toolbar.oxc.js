import { c as _c } from "react/compiler-runtime";
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
  const $ = _c(13);
  const { activeTool, onToolChange, locked, onLockToggle } = t0;
  let handleToolClick;
  if ($[0] !== activeTool || $[1] !== useMemo) {
    const t99 = () => {
      const t2 = (t) => {
        return t.id === activeTool;
      };
      return TOOLS.findIndex(t2);
    };
    const activeIndex = useMemo(t99, [activeTool]);
    $[0] = activeTool;
    $[1] = useMemo;
  }
  let t57;
  if ($[2] !== locked || $[3] !== onToolChange || $[4] !== useCallback) {
    const t106 = (tool) => {
      const t5 = onToolChange(tool);
      return undefined;
    };
    handleToolClick = useCallback(t106, [onToolChange]);
    const t114 = (tool) => {
      const t5 = () => {
        return handleToolClick(tool.id);
      };
      const t6 = () => {
        return setShowTooltip(tool.id);
      };
      const t7 = () => {
        return setShowTooltip(null);
      };
      if (activeTool === tool.id) {
        t14 = "bg-blue-100";
      } else {
        t14 = "hover:bg-gray-100";
      }
      t23 = showTooltip === tool.id;
      t23 = <span className="tooltip">{tool.label} ({tool.shortcut})</span>;
      return <button key={tool.id} onClick={t5} onMouseEnter={t6} onMouseLeave={t7} className={t14}>{tool.label}{t23}</button>;
    };
    const t115 = TOOLS.map(t114);
    $[2] = locked;
    $[3] = onToolChange;
    $[4] = useCallback;
  }
  if (t121) {
    t57 = "bg-blue-100";
  } else {
    t57 = "";
  }
  if ($[5] !== locked) {
    $[5] = locked;
  }
  if (t127) {
    t65 = "🔒";
  } else {
    t65 = "🔓";
  }
  let t146;
  if ($[6] !== t57 || $[7] !== t65 || $[8] !== TOOLS || $[9] !== TOOLS || $[10] !== activeIndex || $[11] !== onLockToggle) {
    t146 = (
      <t111 className={t112}>
        {t115}
        {t118}
        <t119 onClick={t120} className={t57}>{t65}</t119>
        <span className="text-xs text-gray-500">Tool {activeIndex + 1}/{TOOLS.length}</span>
      </t111>
    );
    $[6] = t57;
    $[7] = t65;
    $[8] = TOOLS;
    $[9] = TOOLS;
    $[10] = activeIndex;
    $[11] = onLockToggle;
    $[12] = t146;
  } else {
    t146 = $[12];
  }
  return t146;
}

