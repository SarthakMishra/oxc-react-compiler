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

export function Toolbar({ activeTool, onToolChange, locked, onLockToggle }: ToolbarProps) {
  const [showTooltip, setShowTooltip] = useState<string | null>(null);

  const activeIndex = useMemo(
    () => TOOLS.findIndex((t) => t.id === activeTool),
    [activeTool]
  );

  const handleToolClick = useCallback(
    (tool: Tool) => {
      onToolChange(tool);
    },
    [onToolChange]
  );

  return (
    <div className="flex items-center gap-1 p-1 bg-white rounded-lg shadow">
      {TOOLS.map((tool) => (
        <button
          key={tool.id}
          onClick={() => handleToolClick(tool.id)}
          onMouseEnter={() => setShowTooltip(tool.id)}
          onMouseLeave={() => setShowTooltip(null)}
          className={activeTool === tool.id ? 'bg-blue-100' : 'hover:bg-gray-100'}
        >
          {tool.label}
          {showTooltip === tool.id && (
            <span className="tooltip">{tool.label} ({tool.shortcut})</span>
          )}
        </button>
      ))}
      <div className="border-l mx-1" />
      <button onClick={onLockToggle} className={locked ? 'bg-blue-100' : ''}>
        {locked ? '🔒' : '🔓'}
      </button>
      <span className="text-xs text-gray-500">Tool {activeIndex + 1}/{TOOLS.length}</span>
    </div>
  );
}
