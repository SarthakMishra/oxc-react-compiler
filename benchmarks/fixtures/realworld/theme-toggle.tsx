// XS tier - Inspired by excalidraw theme toggle
import { useState, useCallback } from 'react';

export function ThemeToggle() {
  const [theme, setTheme] = useState<'light' | 'dark'>('light');

  const toggle = useCallback(() => {
    setTheme((t) => (t === 'light' ? 'dark' : 'light'));
  }, []);

  return (
    <button onClick={toggle} className={theme === 'dark' ? 'bg-gray-800 text-white' : 'bg-white text-black'}>
      {theme === 'dark' ? '☀️' : '🌙'}
    </button>
  );
}
