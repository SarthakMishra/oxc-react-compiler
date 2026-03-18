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
  const $ = _c(29);
  let t6;
  let t12;
  let t18;
  let t24;
  let t123;
  let t124;
  let t35;
  let t39;
  let filteredData;
  let t125;
  let t45;
  let t49;
  let sortedData;
  let t126;
  let t127;
  let t62;
  let t65;
  let t128;
  let t72;
  let t73;
  let t129;
  let t122;
  let { data, columns } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t6 = null;
    $[0] = t6;
  } else {
    t6 = $[0];
  }
  let sortKey;
  let setSortKey;
  ([sortKey, setSortKey] = useState(t6));
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t12 = "asc";
    $[1] = t12;
  } else {
    t12 = $[1];
  }
  let sortDir;
  let setSortDir;
  ([sortDir, setSortDir] = useState(t12));
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    t18 = "";
    $[2] = t18;
  } else {
    t18 = $[2];
  }
  let filter;
  let setFilter;
  ([filter, setFilter] = useState(t18));
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    t24 = 0;
    $[3] = t24;
  } else {
    t24 = $[3];
  }
  let page;
  let setPage;
  ([page, setPage] = useState(t24));
  if ($[4] !== columns || $[5] !== data) {
    t123 = 10;
    t35 = () => {
      if (!filter) {
        return data;
      }
      let lowerFilter;
      lowerFilter = filter.toLowerCase();
      let t9 = (row) => {
        let t3 = (col) => {
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
    t39 = [data, columns, filter];
    $[4] = columns;
    $[5] = data;
    $[6] = t123;
    $[7] = t124;
    $[8] = t35;
    $[9] = t39;
  } else {
    t123 = $[6];
    t124 = $[7];
    t35 = $[8];
    t39 = $[9];
  }
  let pageSize = t123;
  filteredData = t124;
  let t40 = useMemo(t35, t39);
  if ($[10] !== t40) {
    filteredData = t40;
    t45 = () => {
      if (!sortKey) {
        return filteredData;
      }
      let t7 = (a, b) => {
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
    $[10] = t40;
    $[11] = filteredData;
    $[12] = t125;
    $[13] = t45;
    $[14] = t49;
  } else {
    filteredData = $[11];
    t125 = $[12];
    t45 = $[13];
    t49 = $[14];
  }
  sortedData = t125;
  let t50 = useMemo(t45, t49);
  if ($[15] !== t50) {
    sortedData = t50;
    t126 = Math.ceil(sortedData.length / pageSize);
    t62 = () => {
      return sortedData.slice(page * pageSize, page + 1 * pageSize);
    };
    t65 = [sortedData, page];
    $[15] = t50;
    $[16] = sortedData;
    $[17] = t126;
    $[18] = t127;
    $[19] = t62;
    $[20] = t65;
  } else {
    sortedData = $[16];
    t126 = $[17];
    t127 = $[18];
    t62 = $[19];
    t65 = $[20];
  }
  let pageCount = t126;
  let pagedData = t127;
  pagedData = useMemo(t62, t65);
  if ($[21] === Symbol.for("react.memo_cache_sentinel")) {
    t72 = (key) => {
      let t3 = (prev) => {
        if (prev === key) {
          let t7 = (d) => {
            let t4;
            if (d === "asc") {
              t4 = "desc";
            } else {
              t4 = "asc";
            }
            return t4;
          };
          let t8 = setSortDir(t7);
          return key;
        }
        let t13 = setSortDir("asc");
        return key;
      };
      let t4 = setSortKey(t3);
      return undefined;
    };
    t73 = [];
    $[21] = t128;
    $[22] = t72;
    $[23] = t73;
  } else {
    t128 = $[21];
    t72 = $[22];
    t73 = $[23];
  }
  let handleSort = t128;
  let t74 = useCallback(t72, t73);
  if ($[24] !== t74) {
    t129 = t74;
    $[24] = t74;
    $[25] = t129;
  } else {
    t129 = $[25];
  }
  handleSort = t129;
  if ($[26] !== t50 || $[27] !== columns) {
    let t79 = (e) => {
      let t6 = setFilter(e.target.value);
      let t10 = setPage(0);
      return undefined;
    };
    let t86 = (col) => {
      let t6;
      if (col.sortable) {
        let t7 = () => {
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
    let t92 = (row, i) => {
      let t6 = (col) => {
        let t5;
        t5 = row[col.key];
        t5 = "";
        return <td key={col.key}>{String(t5)}</td>;
      };
      return <tr key={i}>{columns.map(t6)}</tr>;
    };
    let t98 = () => {
      let t2 = (p) => {
        return Math.max(0, p - 1);
      };
      return setPage(t2);
    };
    let t113 = () => {
      let t2 = (p) => {
        return Math.min(pageCount - 1, p + 1);
      };
      return setPage(t2);
    };
    t122 = (
      <div>
        <input value={filter} onChange={t79} placeholder="Filter..." />
        <table><thead><tr>{columns.map(t86)}</tr></thead><tbody>{pagedData.map(t92)}</tbody></table>
        <div><button onClick={t98} disabled={page === 0}>\n          Prev\n        </button><span>Page {page + 1} of {pageCount}</span><button onClick={t113} disabled={page >= pageCount - 1}>\n          Next\n        </button></div>
      </div>
    );
    $[26] = t50;
    $[27] = columns;
    $[28] = t122;
  } else {
    t122 = $[28];
  }
  return t122;
}

