import { useState, useMemo, useCallback } from 'react';

interface Column {
  key: string;
  label: string;
  sortable?: boolean;
}

interface DataTableProps {
  data: Record<string, unknown>[];
  columns: Column[];
}

type SortDir = 'asc' | 'desc';

export function DataTable({ data, columns }: DataTableProps) {
  const [sortKey, setSortKey] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>('asc');
  const [filter, setFilter] = useState('');
  const [page, setPage] = useState(0);
  const pageSize = 10;

  const filteredData = useMemo(() => {
    if (!filter) return data;
    const lowerFilter = filter.toLowerCase();
    return data.filter((row) =>
      columns.some((col) => {
        const val = row[col.key];
        return val != null && String(val).toLowerCase().includes(lowerFilter);
      })
    );
  }, [data, columns, filter]);

  const sortedData = useMemo(() => {
    if (!sortKey) return filteredData;
    return [...filteredData].sort((a, b) => {
      const aVal = String(a[sortKey] ?? '');
      const bVal = String(b[sortKey] ?? '');
      const cmp = aVal.localeCompare(bVal);
      return sortDir === 'asc' ? cmp : -cmp;
    });
  }, [filteredData, sortKey, sortDir]);

  const pageCount = Math.ceil(sortedData.length / pageSize);
  const pagedData = useMemo(
    () => sortedData.slice(page * pageSize, (page + 1) * pageSize),
    [sortedData, page]
  );

  const handleSort = useCallback((key: string) => {
    setSortKey((prev) => {
      if (prev === key) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
        return key;
      }
      setSortDir('asc');
      return key;
    });
  }, []);

  return (
    <div>
      <input
        value={filter}
        onChange={(e) => { setFilter(e.target.value); setPage(0); }}
        placeholder="Filter..."
      />
      <table>
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={col.key} onClick={col.sortable ? () => handleSort(col.key) : undefined}>
                {col.label}
                {sortKey === col.key && (sortDir === 'asc' ? ' ↑' : ' ↓')}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {pagedData.map((row, i) => (
            <tr key={i}>
              {columns.map((col) => (
                <td key={col.key}>{String(row[col.key] ?? '')}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <div>
        <button onClick={() => setPage((p) => Math.max(0, p - 1))} disabled={page === 0}>
          Prev
        </button>
        <span>Page {page + 1} of {pageCount}</span>
        <button onClick={() => setPage((p) => Math.min(pageCount - 1, p + 1))} disabled={page >= pageCount - 1}>
          Next
        </button>
      </div>
    </div>
  );
}
