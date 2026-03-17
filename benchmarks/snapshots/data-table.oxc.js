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
  const $ = _c(33);
  const { data, columns } = t0;
  let t122;
  let t123;
  let t124;
  let t125;
  let t126;
  let t35;
  let t39;
  let t6;
  if ($[0] !== t50 || $[1] !== columns || $[2] !== data) {
    t6 = null;
    $[0] = t50;
    $[1] = columns;
    $[2] = data;
    $[3] = t122;
    $[4] = t123;
    $[5] = t124;
    $[6] = t125;
    $[7] = t126;
    $[8] = t35;
    $[9] = t39;
    $[10] = t6;
  } else {
    t122 = $[3];
    t123 = $[4];
    t124 = $[5];
    t125 = $[6];
    t126 = $[7];
    t35 = $[8];
    t39 = $[9];
    t6 = $[10];
  }
  const sortDir = t123;
  const page = t124;
  const pageSize = t125;
  const filteredData = t126;
  let t127;
  if ($[11] === Symbol.for("react.memo_cache_sentinel")) {
    $[11] = t127;
  } else {
    t127 = $[11];
  }
  const sortKey = t127;
  let setSortKey;
  let t12;
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    $[12] = t12;
  } else {
    t12 = $[12];
  }
  let t18;
  if ($[13] === Symbol.for("react.memo_cache_sentinel")) {
    $[13] = t18;
  } else {
    t18 = $[13];
  }
  let t24;
  if ($[14] === Symbol.for("react.memo_cache_sentinel")) {
    $[14] = t24;
  } else {
    t24 = $[14];
  }
  let pageSize;
  pageSize = 10;
  let filteredData;
  t35 = () => {
    if (!filter) {
      return data;
    }
    let lowerFilter;
    lowerFilter = filter.toLowerCase();
    const t9 = (row) => {
      const t3 = (col) => {
        let val;
        val = row[col.key];
        let t7;
        t7 = val != null;
        t7 = String(val).toLowerCase().includes(lowerFilter);
        return t7;
      };
      return columns.some(t3);
    };
    return data.filter(t9);
  };
  const t40 = useMemo(t35, [data, columns, filter]);
  let t128;
  let t45;
  let t49;
  if ($[15] !== t40) {
    filteredData = t40;
    t45 = () => {
      if (!sortKey) {
        return filteredData;
      }
      const t7 = (a, b) => {
        let aVal;
        let t4;
        t4 = a[sortKey];
        t4 = "";
        aVal = String(t4);
        let bVal;
        let t13;
        t13 = b[sortKey];
        t13 = "";
        bVal = String(t13);
        let cmp;
        cmp = aVal.localeCompare(bVal);
        let t27;
        if (sortDir === "asc") {
          t27 = cmp;
        } else {
          t27 = -cmp;
        }
        return t27;
      };
      return [...filteredData].sort(t7);
    };
    t49 = [filteredData, sortKey, sortDir];
    $[15] = t40;
    $[16] = filteredData;
    $[17] = t128;
    $[18] = t45;
    $[19] = t49;
  } else {
    filteredData = $[16];
    t128 = $[17];
    t45 = $[18];
    t49 = $[19];
  }
  const sortedData = t128;
  const t50 = useMemo(t45, t49);
  let t129;
  let t130;
  let sortedData;
  let t62;
  let t65;
  if ($[20] !== t50) {
    sortedData = t50;
    t129 = Math.ceil(sortedData.length / pageSize);
    t62 = () => {
      return sortedData.slice(page * pageSize, page + 1 * pageSize);
    };
    t65 = [sortedData, page];
    $[20] = t50;
    $[21] = sortedData;
    $[22] = t129;
    $[23] = t130;
    $[24] = t62;
    $[25] = t65;
  } else {
    sortedData = $[21];
    t129 = $[22];
    t130 = $[23];
    t62 = $[24];
    t65 = $[25];
  }
  const pageCount = t129;
  const pagedData = t130;
  const t66 = useMemo(t62, t65);
  let t131;
  if ($[26] !== t66) {
    t131 = t66;
    $[26] = t66;
    $[27] = t131;
  } else {
    t131 = $[27];
  }
  const pagedData = t131;
  let t132;
  let t72;
  let t73;
  if ($[28] === Symbol.for("react.memo_cache_sentinel")) {
    t72 = (key) => {
      const t3 = (prev) => {
        if (prev === key) {
          const t7 = (d) => {
            let t4;
            if (d === "asc") {
              t4 = "desc";
            } else {
              t4 = "asc";
            }
            return t4;
          };
          const t8 = setSortDir(t7);
          return key;
        }
        const t13 = setSortDir("asc");
        return key;
      };
      const t4 = setSortKey(t3);
      return undefined;
    };
    t73 = [];
    $[28] = t132;
    $[29] = t72;
    $[30] = t73;
  } else {
    t132 = $[28];
    t72 = $[29];
    t73 = $[30];
  }
  const handleSort = t132;
  const t74 = useCallback(t72, t73);
  let t133;
  if ($[31] !== t74) {
    t133 = t74;
    $[31] = t74;
    $[32] = t133;
  } else {
    t133 = $[32];
  }
  const handleSort = t133;
  const t79 = (e) => {
    const t6 = setFilter(e.target.value);
    const t10 = setPage(0);
    return undefined;
  };
  const t86 = (col) => {
    let t6;
    if (col.sortable) {
      const t7 = () => {
        return handleSort(col.key);
      };
      t6 = t7;
    } else {
      t6 = undefined;
    }
    let t11;
    t11 = sortKey === col.key;
    let t21;
    if (sortDir === "asc") {
      t21 = " ↑";
    } else {
      t21 = " ↓";
    }
    t11 = t21;
    return <th key={col.key} onClick={t6}>{col.label}{t11}</th>;
  };
  const t92 = (row, i) => {
    const t6 = (col) => {
      let t5;
      t5 = row[col.key];
      t5 = "";
      return <td key={col.key}>{String(t5)}</td>;
    };
    return <tr key={i}>{columns.map(t6)}</tr>;
  };
  const t98 = () => {
    const t2 = (p) => {
      return Math.max(0, p - 1);
    };
    return setPage(t2);
  };
  const t113 = () => {
    const t2 = (p) => {
      return Math.min(pageCount - 1, p + 1);
    };
    return setPage(t2);
  };
  return <div><input value={filter} onChange={t79} placeholder="Filter..." /><table><thead><tr>{columns.map(t86)}</tr></thead><tbody>{pagedData.map(t92)}</tbody></table><div><button onClick={t98} disabled={page === 0}>\n          Prev\n        </button><span>Page {page + 1} of {pageCount}</span><button onClick={t113} disabled={page >= pageCount - 1}>\n          Next\n        </button></div></div>;
}

