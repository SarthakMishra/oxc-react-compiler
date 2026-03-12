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

export function ColorPicker({ color, onChange, presets = DEFAULT_PRESETS, showCustom = true }: ColorPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [customColor, setCustomColor] = useState(color);
  const [recentColors, setRecentColors] = useState<string[]>([]);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const addToRecent = useCallback((c: string) => {
    setRecentColors((prev) => {
      const next = [c, ...prev.filter((x) => x !== c)];
      return next.slice(0, 5);
    });
  }, []);

  const handlePresetClick = useCallback(
    (c: string) => {
      onChange(c);
      addToRecent(c);
      setIsOpen(false);
    },
    [onChange, addToRecent]
  );

  const handleCustomSubmit = useCallback(() => {
    if (/^#[0-9a-fA-F]{6}$/.test(customColor)) {
      onChange(customColor);
      addToRecent(customColor);
      setIsOpen(false);
    }
  }, [customColor, onChange, addToRecent]);

  const groupedPresets = useMemo(() => {
    const rows: string[][] = [];
    for (let i = 0; i < presets.length; i += 4) {
      rows.push(presets.slice(i, i + 4));
    }
    return rows;
  }, [presets]);

  return (
    <div className="relative inline-block" ref={popoverRef}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="w-8 h-8 rounded border-2"
        style={{ backgroundColor: color }}
        aria-label="Pick color"
      />

      {isOpen && (
        <div className="absolute z-50 mt-1 p-3 bg-white rounded-lg shadow-lg border">
          {recentColors.length > 0 && (
            <div className="mb-2">
              <span className="text-xs text-gray-500">Recent</span>
              <div className="flex gap-1 mt-1">
                {recentColors.map((c) => (
                  <button
                    key={c}
                    onClick={() => handlePresetClick(c)}
                    className="w-6 h-6 rounded border"
                    style={{ backgroundColor: c }}
                  />
                ))}
              </div>
            </div>
          )}

          <div className="space-y-1">
            {groupedPresets.map((row, i) => (
              <div key={i} className="flex gap-1">
                {row.map((c) => (
                  <button
                    key={c}
                    onClick={() => handlePresetClick(c)}
                    className={`w-6 h-6 rounded border ${c === color ? 'ring-2 ring-blue-500' : ''}`}
                    style={{ backgroundColor: c }}
                  />
                ))}
              </div>
            ))}
          </div>

          {showCustom && (
            <div className="mt-2 flex gap-1">
              <input
                type="text"
                value={customColor}
                onChange={(e) => setCustomColor(e.target.value)}
                placeholder="#000000"
                className="w-20 text-xs border rounded px-1"
              />
              <button onClick={handleCustomSubmit} className="text-xs px-2 bg-blue-500 text-white rounded">
                Apply
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
