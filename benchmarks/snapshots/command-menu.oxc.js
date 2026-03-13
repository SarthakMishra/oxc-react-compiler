// M tier - Inspired by shadcn/ui command/combobox component
import { useState, useMemo, useCallback, useRef, useEffect } from 'react';

interface CommandItem {
  id: string;
  label: string;
  description?: string;
  group?: string;
  shortcut?: string;
  onSelect: () => void;
}

interface CommandMenuProps {
  items: CommandItem[];
  placeholder?: string;
  emptyMessage?: string;
  onOpenChange?: (open: boolean) => void;
}

export function CommandMenu({
  items,
  placeholder = 'Type a command or search...',
  emptyMessage = 'No results found.',
  onOpenChange,
}: CommandMenuProps) {
  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const filteredItems = useMemo(() => {
    if (!query) return items;
    const q = query.toLowerCase();
    return items.filter(
      (item) =>
        item.label.toLowerCase().includes(q) ||
        (item.description && item.description.toLowerCase().includes(q))
    );
  }, [items, query]);

  const groupedItems = useMemo(() => {
    const groups = new Map<string, CommandItem[]>();
    for (const item of filteredItems) {
      const group = item.group || 'Actions';
      if (!groups.has(group)) groups.set(group, []);
      groups.get(group)!.push(item);
    }
    return groups;
  }, [filteredItems]);

  const flatList = useMemo(() => {
    const result: CommandItem[] = [];
    for (const items of groupedItems.values()) {
      result.push(...items);
    }
    return result;
  }, [groupedItems]);

  const handleSelect = useCallback(
    (item: CommandItem) => {
      item.onSelect();
      setQuery('');
      onOpenChange?.(false);
    },
    [onOpenChange]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          setActiveIndex((i) => Math.min(i + 1, flatList.length - 1));
          break;
        case 'ArrowUp':
          e.preventDefault();
          setActiveIndex((i) => Math.max(i - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (flatList[activeIndex]) {
            handleSelect(flatList[activeIndex]);
          }
          break;
        case 'Escape':
          onOpenChange?.(false);
          break;
      }
    },
    [flatList, activeIndex, handleSelect, onOpenChange]
  );

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  return (
    <div className="w-full max-w-md bg-white border rounded-lg shadow-xl" onKeyDown={handleKeyDown}>
      <div className="flex items-center border-b px-3">
        <span className="text-gray-400">⌘</span>
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={placeholder}
          className="flex-1 px-2 py-3 outline-none"
        />
      </div>

      <div ref={listRef} className="max-h-72 overflow-y-auto p-1">
        {filteredItems.length === 0 ? (
          <p className="py-6 text-center text-sm text-gray-500">{emptyMessage}</p>
        ) : (
          Array.from(groupedItems.entries()).map(([group, groupItems]) => (
            <div key={group}>
              <div className="px-2 py-1.5 text-xs font-semibold text-gray-500">{group}</div>
              {groupItems.map((item) => {
                const index = flatList.indexOf(item);
                return (
                  <button
                    key={item.id}
                    onClick={() => handleSelect(item)}
                    className={`w-full text-left px-2 py-1.5 rounded text-sm flex justify-between ${
                      index === activeIndex ? 'bg-blue-50 text-blue-700' : 'hover:bg-gray-50'
                    }`}
                  >
                    <div>
                      <span>{item.label}</span>
                      {item.description && <span className="ml-2 text-gray-400">{item.description}</span>}
                    </div>
                    {item.shortcut && (
                      <kbd className="text-xs bg-gray-100 px-1 rounded">{item.shortcut}</kbd>
                    )}
                  </button>
                );
              })}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
