import { c as _c } from "react/compiler-runtime";
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

export function DataTable(t0) {
  const $ = _c(22);
  const { data, columns } = t0;
  const pageSize = 10;
  let sortedData;
  if ($[0] !== columns || $[1] !== data || $[2] !== filter || $[3] !== useMemo) {
    const t201 = () => {
      if (!filter) {
        return data;
      }
      const lowerFilter = filter.toLowerCase();
      const t13 = (row) => {
        const t3 = (col) => {
          const val = row[col.key];
          t10 = val != null;
          t10 = String(val).toLowerCase().includes(lowerFilter);
          return t10;
        };
        return columns.some(t3);
      };
      return data.filter(t13);
    };
    const filteredData = useMemo(t201, [data, columns, filter]);
    $[0] = columns;
    $[1] = data;
    $[2] = filter;
    $[3] = useMemo;
  }
  let pageCount;
  let pagedData;
  if ($[4] !== filteredData || $[5] !== pageSize || $[6] !== sortDir || $[7] !== sortKey || $[8] !== useMemo) {
    const t210 = () => {
      if (!sortKey) {
        return filteredData;
      }
      const t8 = (a, b) => {
        t5 = a[sortKey];
        t5 = "";
        const aVal = String(t5);
        t20 = b[sortKey];
        t20 = "";
        const bVal = String(t20);
        const cmp = aVal.localeCompare(bVal);
        if (sortDir === "asc") {
          t44 = cmp;
        } else {
          t44 = -cmp;
        }
        return t44;
      };
      return [...filteredData].sort(t8);
    };
    sortedData = useMemo(t210, [filteredData, sortKey, sortDir]);
    pageCount = Math.ceil(sortedData.length / pageSize);
    $[4] = filteredData;
    $[5] = pageSize;
    $[6] = sortDir;
    $[7] = sortKey;
    $[8] = useMemo;
  }
  let handleSort;
  if ($[9] !== page || $[10] !== sortedData || $[11] !== useMemo) {
    const t227 = () => {
      return sortedData.slice(page * pageSize, page + 1 * pageSize);
    };
    pagedData = useMemo(t227, [sortedData, page]);
    $[9] = page;
    $[10] = sortedData;
    $[11] = useMemo;
  }
  let t285;
  if ($[12] !== columns || $[13] !== filter || $[14] !== page || $[15] !== page || $[16] !== page || $[17] !== pageCount || $[18] !== pageCount || $[19] !== pagedData || $[20] !== useCallback) {
    const t235 = (key) => {
      const t3 = (prev) => {
        if (prev === key) {
          const t8 = (d) => {
            if (d === "asc") {
              t5 = "desc";
            } else {
              t5 = "asc";
            }
            return t5;
          };
          const t9 = setSortDir(t8);
          return key;
        }
        const t15 = setSortDir("asc");
        return key;
      };
      const t4 = setSortKey(t3);
      return undefined;
    };
    handleSort = useCallback(t235, []);
    const t242 = (e) => {
      const t7 = setFilter(e.target.value);
      const t11 = setPage(0);
      return undefined;
    };
    const t249 = (col) => {
      if (col.sortable) {
        const t10 = () => {
          return handleSort(col.key);
        };
        t8 = t10;
      } else {
        t8 = undefined;
      }
      t17 = sortKey === col.key;
      if (sortDir === "asc") {
        t30 = " ↑";
      } else {
        t30 = " ↓";
      }
      t17 = t30;
      return <th key={col.key} onClick={t8}>{col.label}{t17}</th>;
    };
    const t255 = (row, i) => {
      const t7 = (col) => {
        t6 = row[col.key];
        t6 = "";
        return <td key={col.key}>{String(t6)}</td>;
      };
      return <tr key={i}>{columns.map(t7)}</tr>;
    };
    const t261 = () => {
      const t2 = (p) => {
        return Math.max(0, p - 1);
      };
      return setPage(t2);
    };
    const t276 = () => {
      const t2 = (p) => {
        return Math.min(pageCount - 1, p + 1);
      };
      return setPage(t2);
    };
    t285 = (
      <div>
        <input value={filter} onChange={t242} placeholder="Filter..." />
        <table><thead><tr>{columns.map(t249)}</tr></thead><tbody>{pagedData.map(t255)}</tbody></table>
        <div><button onClick={t261} disabled={page === 0}>\n          Prev\n        </button><span>Page {page + 1} of {pageCount}</span><button onClick={t276} disabled={page >= pageCount - 1}>\n          Next\n        </button></div>
      </div>
    );
    $[12] = columns;
    $[13] = filter;
    $[14] = page;
    $[15] = page;
    $[16] = page;
    $[17] = pageCount;
    $[18] = pageCount;
    $[19] = pagedData;
    $[20] = useCallback;
    $[21] = t285;
  } else {
    t285 = $[21];
  }
  return t285;
}

