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
  const $ = _c(3);
  const t71 = useState;
  const t72 = null;
  const t73 = t71(t72);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t74 = Discriminant(4) */
    /* t75 = Discriminant(4) */
  } else {
  }
  /* t76 = Discriminant(6) */
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t77 = Discriminant(4) */
  } else {
  }
  const t78 = useMemo;
  /* t79 = Discriminant(28) */
  const t80 = activeTool;
  const t81 = [t80];
  const t82 = t78(t79, t81);
  const activeIndex = t82;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t84 = Discriminant(4) */
  } else {
  }
  const t85 = useCallback;
  /* t86 = Discriminant(28) */
  const t87 = onToolChange;
  const t88 = [t87];
  const t89 = t85(t86, t88);
  const handleToolClick = t89;
  const t91 = "div";
  const t92 = "flex items-center gap-1 p-1 bg-white rounded-lg shadow";
  const t93 = TOOLS;
  /* t94 = Discriminant(28) */
  const t95 = t93.map(t94);
  const t96 = "div";
  const t97 = "border-l mx-1";
  const t98 = <t96 className={t97} />;
  const t99 = "button";
  const t100 = onLockToggle;
  const t101 = locked;
}

