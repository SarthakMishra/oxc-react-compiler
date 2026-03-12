import { c as _c } from "react/compiler-runtime";
import { useState, useMemo, useCallback } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function DataTable(t0) {
  const $ = _c(56);
  const {
    data,
    columns
  } = t0;
  const [sortKey, setSortKey] = useState(null);
  const [sortDir, setSortDir] = useState("asc");
  const [filter, setFilter] = useState("");
  const [page, setPage] = useState(0);
  let t1;
  bb0: {
    if (!filter) {
      t1 = data;
      break bb0;
    }
    let t2;
    if ($[0] !== columns || $[1] !== data || $[2] !== filter) {
      const lowerFilter = filter.toLowerCase();
      t2 = data.filter(row => columns.some(col => {
        const val = row[col.key];
        return val != null && String(val).toLowerCase().includes(lowerFilter);
      }));
      $[0] = columns;
      $[1] = data;
      $[2] = filter;
      $[3] = t2;
    } else {
      t2 = $[3];
    }
    t1 = t2;
  }
  const filteredData = t1;
  let t2;
  bb1: {
    if (!sortKey) {
      t2 = filteredData;
      break bb1;
    }
    let t3;
    if ($[4] !== filteredData || $[5] !== sortDir || $[6] !== sortKey) {
      let t4;
      if ($[8] !== sortDir || $[9] !== sortKey) {
        t4 = (a, b) => {
          const aVal = String(a[sortKey] ?? "");
          const bVal = String(b[sortKey] ?? "");
          const cmp = aVal.localeCompare(bVal);
          return sortDir === "asc" ? cmp : -cmp;
        };
        $[8] = sortDir;
        $[9] = sortKey;
        $[10] = t4;
      } else {
        t4 = $[10];
      }
      t3 = [...filteredData].sort(t4);
      $[4] = filteredData;
      $[5] = sortDir;
      $[6] = sortKey;
      $[7] = t3;
    } else {
      t3 = $[7];
    }
    t2 = t3;
  }
  const sortedData = t2;
  const pageCount = Math.ceil(sortedData.length / 10);
  let t3;
  if ($[11] !== page || $[12] !== sortedData) {
    t3 = sortedData.slice(page * 10, (page + 1) * 10);
    $[11] = page;
    $[12] = sortedData;
    $[13] = t3;
  } else {
    t3 = $[13];
  }
  const pagedData = t3;
  let t4;
  if ($[14] === Symbol.for("react.memo_cache_sentinel")) {
    t4 = key => {
      setSortKey(prev => {
        if (prev === key) {
          setSortDir(_temp);
          return key;
        }
        setSortDir("asc");
        return key;
      });
    };
    $[14] = t4;
  } else {
    t4 = $[14];
  }
  const handleSort = t4;
  let t5;
  if ($[15] === Symbol.for("react.memo_cache_sentinel")) {
    t5 = e => {
      setFilter(e.target.value);
      setPage(0);
    };
    $[15] = t5;
  } else {
    t5 = $[15];
  }
  let t6;
  if ($[16] !== filter) {
    t6 = /*#__PURE__*/_jsx("input", {
      value: filter,
      onChange: t5,
      placeholder: "Filter..."
    });
    $[16] = filter;
    $[17] = t6;
  } else {
    t6 = $[17];
  }
  let t7;
  if ($[18] !== columns || $[19] !== sortDir || $[20] !== sortKey) {
    let t8;
    if ($[22] !== sortDir || $[23] !== sortKey) {
      t8 = col_0 => /*#__PURE__*/_jsxs("th", {
        onClick: col_0.sortable ? () => handleSort(col_0.key) : undefined,
        children: [col_0.label, sortKey === col_0.key && (sortDir === "asc" ? " \u2191" : " \u2193")]
      }, col_0.key);
      $[22] = sortDir;
      $[23] = sortKey;
      $[24] = t8;
    } else {
      t8 = $[24];
    }
    t7 = columns.map(t8);
    $[18] = columns;
    $[19] = sortDir;
    $[20] = sortKey;
    $[21] = t7;
  } else {
    t7 = $[21];
  }
  let t8;
  if ($[25] !== t7) {
    t8 = /*#__PURE__*/_jsx("thead", {
      children: /*#__PURE__*/_jsx("tr", {
        children: t7
      })
    });
    $[25] = t7;
    $[26] = t8;
  } else {
    t8 = $[26];
  }
  let t9;
  if ($[27] !== columns || $[28] !== pagedData) {
    let t10;
    if ($[30] !== columns) {
      t10 = (row_0, i) => /*#__PURE__*/_jsx("tr", {
        children: columns.map(col_1 => /*#__PURE__*/_jsx("td", {
          children: String(row_0[col_1.key] ?? "")
        }, col_1.key))
      }, i);
      $[30] = columns;
      $[31] = t10;
    } else {
      t10 = $[31];
    }
    t9 = pagedData.map(t10);
    $[27] = columns;
    $[28] = pagedData;
    $[29] = t9;
  } else {
    t9 = $[29];
  }
  let t10;
  if ($[32] !== t9) {
    t10 = /*#__PURE__*/_jsx("tbody", {
      children: t9
    });
    $[32] = t9;
    $[33] = t10;
  } else {
    t10 = $[33];
  }
  let t11;
  if ($[34] !== t10 || $[35] !== t8) {
    t11 = /*#__PURE__*/_jsxs("table", {
      children: [t8, t10]
    });
    $[34] = t10;
    $[35] = t8;
    $[36] = t11;
  } else {
    t11 = $[36];
  }
  let t12;
  if ($[37] === Symbol.for("react.memo_cache_sentinel")) {
    t12 = () => setPage(_temp2);
    $[37] = t12;
  } else {
    t12 = $[37];
  }
  const t13 = page === 0;
  let t14;
  if ($[38] !== t13) {
    t14 = /*#__PURE__*/_jsx("button", {
      onClick: t12,
      disabled: t13,
      children: "Prev"
    });
    $[38] = t13;
    $[39] = t14;
  } else {
    t14 = $[39];
  }
  const t15 = page + 1;
  let t16;
  if ($[40] !== pageCount || $[41] !== t15) {
    t16 = /*#__PURE__*/_jsxs("span", {
      children: ["Page ", t15, " of ", pageCount]
    });
    $[40] = pageCount;
    $[41] = t15;
    $[42] = t16;
  } else {
    t16 = $[42];
  }
  let t17;
  if ($[43] !== pageCount) {
    t17 = () => setPage(p_0 => Math.min(pageCount - 1, p_0 + 1));
    $[43] = pageCount;
    $[44] = t17;
  } else {
    t17 = $[44];
  }
  const t18 = page >= pageCount - 1;
  let t19;
  if ($[45] !== t17 || $[46] !== t18) {
    t19 = /*#__PURE__*/_jsx("button", {
      onClick: t17,
      disabled: t18,
      children: "Next"
    });
    $[45] = t17;
    $[46] = t18;
    $[47] = t19;
  } else {
    t19 = $[47];
  }
  let t20;
  if ($[48] !== t14 || $[49] !== t16 || $[50] !== t19) {
    t20 = /*#__PURE__*/_jsxs("div", {
      children: [t14, t16, t19]
    });
    $[48] = t14;
    $[49] = t16;
    $[50] = t19;
    $[51] = t20;
  } else {
    t20 = $[51];
  }
  let t21;
  if ($[52] !== t11 || $[53] !== t20 || $[54] !== t6) {
    t21 = /*#__PURE__*/_jsxs("div", {
      children: [t6, t11, t20]
    });
    $[52] = t11;
    $[53] = t20;
    $[54] = t6;
    $[55] = t21;
  } else {
    t21 = $[55];
  }
  return t21;
}
function _temp2(p) {
  return Math.max(0, p - 1);
}
function _temp(d) {
  return d === "asc" ? "desc" : "asc";
}