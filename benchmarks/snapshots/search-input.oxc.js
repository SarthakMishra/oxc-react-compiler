// S tier - Inspired by shadcn/ui command palette search
import { useState, useCallback, useRef, useEffect } from 'react';

interface SearchInputProps {
  onSearch: (query: string) => void;
  placeholder?: string;
  debounceMs?: number;
}

export function SearchInput({ onSearch, placeholder = 'Search...', debounceMs = 300 }: SearchInputProps) {
  const [value, setValue] = useState('');
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newVal = e.target.value;
      setValue(newVal);

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        onSearch(newVal);
      }, debounceMs);
    },
    [onSearch, debounceMs]
  );

  const handleClear = useCallback(() => {
    setValue('');
    onSearch('');
    if (timerRef.current) clearTimeout(timerRef.current);
  }, [onSearch]);

  return (
    <div className="relative">
      <input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder={placeholder}
        className="w-full px-3 py-2 border rounded"
      />
      {value && (
        <button onClick={handleClear} className="absolute right-2 top-2 text-gray-400">
          ×
        </button>
      )}
    </div>
  );
}
