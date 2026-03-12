import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
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
  const $ = _c(41);
  const { data, columns } = t0;
  if ($[0] !== data || $[1] !== columns) {
    $[0] = data;
    $[1] = columns;
  } else {
  }
  const t172 = useState;
  const t173 = null;
  const t174 = t172(t173);
  let sortKey;
  let setSortKey;
  if ($[2] !== sortKey || $[3] !== setSortKey) {
    $[2] = sortKey;
    $[3] = setSortKey;
  } else {
  }
  ([sortKey, setSortKey] = t174);
  const t178 = useState;
  const t179 = "asc";
  const t180 = t178(t179);
  let sortDir;
  let setSortDir;
  if ($[4] !== sortDir || $[5] !== setSortDir) {
    $[4] = sortDir;
    $[5] = setSortDir;
  } else {
  }
  ([sortDir, setSortDir] = t180);
  const t184 = useState;
  const t185 = "";
  const t186 = t184(t185);
  let filter;
  let setFilter;
  if ($[6] !== filter || $[7] !== setFilter) {
    $[6] = filter;
    $[7] = setFilter;
  } else {
  }
  ([filter, setFilter] = t186);
  const t190 = useState;
  const t191 = 0;
  const t192 = t190(t191);
  let page;
  let setPage;
  if ($[8] !== page || $[9] !== setPage) {
    $[8] = page;
    $[9] = setPage;
  } else {
  }
  ([page, setPage] = t192);
  let pageSize;
  if ($[10] !== pageSize) {
    $[10] = pageSize;
  } else {
  }
  const t197 = 10;
  pageSize = t197;
  let filteredData;
  if ($[11] !== filteredData) {
    $[11] = filteredData;
  } else {
  }
  let sortedData;
  if ($[12] !== useMemo || $[13] !== data || $[14] !== columns || $[15] !== filter || $[16] !== filteredData || $[17] !== sortedData) {
    const t200 = useMemo;
    const t201 = () => {
      const t1 = filter;
      const t2 = !t1;
      if (t2) {
        const t4 = data;
        return t4;
      } else {
      }
      let lowerFilter;
      const t8 = filter;
      const t9 = t8.toLowerCase();
      lowerFilter = t9;
      const t12 = data;
      const t13 = (row) => {
        const t2 = columns;
        const t3 = (col) => {
          let val;
          const t4 = row;
          const t6 = col;
          const t7 = t6.key;
          const t8 = t4[t7];
          val = t8;
          let t10;
          const t13 = val;
          const t14 = null;
          const t15 = t13 != t14;
          t10 = t15;
          const t17 = String;
          const t19 = val;
          const t20 = t17(t19);
          const t21 = t20.toLowerCase();
          const t23 = lowerFilter;
          const t24 = t21.includes(t23);
          t10 = t24;
          return t10;
        };
        const t4 = t2.some(t3);
        return t4;
      };
      const t14 = t12.filter(t13);
      return t14;
    };
    const t202 = data;
    const t203 = columns;
    const t204 = filter;
    const t205 = [t202, t203, t204];
    const t206 = t200(t201, t205);
    filteredData = t206;
    $[12] = useMemo;
    $[13] = data;
    $[14] = columns;
    $[15] = filter;
    $[16] = filteredData;
    $[17] = sortedData;
  } else {
  }
  let pageCount;
  let pagedData;
  if ($[18] !== useMemo || $[19] !== filteredData || $[20] !== sortKey || $[21] !== sortDir || $[22] !== sortedData || $[23] !== pageCount || $[24] !== sortedData || $[25] !== pageSize || $[26] !== pagedData) {
    const t209 = useMemo;
    const t210 = () => {
      const t1 = sortKey;
      const t2 = !t1;
      if (t2) {
        const t4 = filteredData;
        return t4;
      } else {
      }
      const t6 = filteredData;
      const t7 = [...t6];
      const t8 = (a, b) => {
        let aVal;
        const t4 = String;
        let t5;
        const t8 = a;
        const t10 = sortKey;
        const t11 = t8[t10];
        t5 = t11;
        const t13 = "";
        t5 = t13;
        const t15 = t4(t5);
        aVal = t15;
        let bVal;
        const t19 = String;
        let t20;
        const t23 = b;
        const t25 = sortKey;
        const t26 = t23[t25];
        t20 = t26;
        const t28 = "";
        t20 = t28;
        const t30 = t19(t20);
        bVal = t30;
        let cmp;
        const t35 = aVal;
        const t37 = bVal;
        const t38 = t35.localeCompare(t37);
        cmp = t38;
        const t41 = sortDir;
        const t42 = "asc";
        const t43 = t41 === t42;
        let t44;
        if (t43) {
          const t47 = cmp;
          t44 = t47;
        } else {
          const t50 = cmp;
          const t51 = -t50;
          t44 = t51;
        }
        return t44;
      };
      const t9 = t7.sort(t8);
      return t9;
    };
    const t211 = filteredData;
    const t212 = sortKey;
    const t213 = sortDir;
    const t214 = [t211, t212, t213];
    const t215 = t209(t210, t214);
    sortedData = t215;
    const t218 = Math;
    const t219 = sortedData;
    const t220 = t219.length;
    const t221 = pageSize;
    const t222 = t220 / t221;
    const t223 = t218.ceil(t222);
    pageCount = t223;
    $[18] = useMemo;
    $[19] = filteredData;
    $[20] = sortKey;
    $[21] = sortDir;
    $[22] = sortedData;
    $[23] = pageCount;
    $[24] = sortedData;
    $[25] = pageSize;
    $[26] = pagedData;
  } else {
  }
  let handleSort;
  if ($[27] !== useMemo || $[28] !== sortedData || $[29] !== page || $[30] !== pagedData || $[31] !== handleSort) {
    const t226 = useMemo;
    const t227 = () => {
      const t1 = sortedData;
      const t3 = page;
      const t5 = pageSize;
      const t6 = t3 * t5;
      const t8 = page;
      const t9 = 1;
      const t10 = t8 + t9;
      const t12 = pageSize;
      const t13 = t10 * t12;
      const t14 = t1.slice(t6, t13);
      return t14;
    };
    const t228 = sortedData;
    const t229 = page;
    const t230 = [t228, t229];
    const t231 = t226(t227, t230);
    pagedData = t231;
    $[27] = useMemo;
    $[28] = sortedData;
    $[29] = page;
    $[30] = pagedData;
    $[31] = handleSort;
  } else {
  }
  const t234 = useCallback;
  const t235 = (key) => {
    const t2 = setSortKey;
    const t3 = (prev) => {
      const t2 = prev;
      const t4 = key;
      const t5 = t2 === t4;
      if (t5) {
        const t7 = setSortDir;
        const t8 = (d) => {
          const t2 = d;
          const t3 = "asc";
          const t4 = t2 === t3;
          let t5;
          if (t4) {
            const t7 = "desc";
            t5 = t7;
          } else {
            const t9 = "asc";
            t5 = t9;
          }
          return t5;
        };
        const t9 = t7(t8);
        const t11 = key;
        return t11;
      } else {
      }
      const t13 = setSortDir;
      const t14 = "asc";
      const t15 = t13(t14);
      const t17 = key;
      return t17;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t236 = [];
  const t237 = t234(t235, t236);
  handleSort = t237;
  let t285;
  if ($[32] !== filter || $[33] !== columns || $[34] !== pagedData || $[35] !== page || $[36] !== page || $[37] !== pageCount || $[38] !== page || $[39] !== pageCount) {
    const t239 = "div";
    const t240 = "input";
    const t241 = filter;
    const t242 = (e) => {
      const t2 = setFilter;
      const t4 = e;
      const t5 = t4.target;
      const t6 = t5.value;
      const t7 = t2(t6);
      const t9 = setPage;
      const t10 = 0;
      const t11 = t9(t10);
      const t12 = undefined;
      return t12;
    };
    const t243 = "Filter...";
    const t244 = _jsx(t240, { value: t241, onChange: t242, placeholder: t243 });
    const t245 = "table";
    const t246 = "thead";
    const t247 = "tr";
    const t248 = columns;
    const t249 = (col) => {
      const t1 = "th";
      const t3 = col;
      const t4 = t3.key;
      const t6 = col;
      const t7 = t6.sortable;
      let t8;
      if (t7) {
        const t10 = () => {
          const t1 = handleSort;
          const t3 = col;
          const t4 = t3.key;
          const t5 = t1(t4);
          return t5;
        };
        t8 = t10;
      } else {
        const t12 = undefined;
        t8 = t12;
      }
      const t15 = col;
      const t16 = t15.label;
      let t17;
      const t20 = sortKey;
      const t22 = col;
      const t23 = t22.key;
      const t24 = t20 === t23;
      t17 = t24;
      const t27 = sortDir;
      const t28 = "asc";
      const t29 = t27 === t28;
      let t30;
      if (t29) {
        const t32 = " ↑";
        t30 = t32;
      } else {
        const t34 = " ↓";
        t30 = t34;
      }
      t17 = t30;
      const t37 = _jsxs(t1, { key: t4, onClick: t8, children: [t16, t17] });
      return t37;
    };
    const t250 = t248.map(t249);
    const t251 = _jsx(t247, { children: t250 });
    const t252 = _jsx(t246, { children: t251 });
    const t253 = "tbody";
    const t254 = pagedData;
    const t255 = (row, i) => {
      const t2 = "tr";
      const t4 = i;
      const t6 = columns;
      const t7 = (col) => {
        const t1 = "td";
        const t3 = col;
        const t4 = t3.key;
        const t5 = String;
        let t6;
        const t9 = row;
        const t11 = col;
        const t12 = t11.key;
        const t13 = t9[t12];
        t6 = t13;
        const t15 = "";
        t6 = t15;
        const t17 = t5(t6);
        const t18 = _jsx(t1, { key: t4, children: t17 });
        return t18;
      };
      const t8 = t6.map(t7);
      const t9 = _jsx(t2, { key: t4, children: t8 });
      return t9;
    };
    const t256 = t254.map(t255);
    const t257 = _jsx(t253, { children: t256 });
    const t258 = _jsxs(t245, { children: [t252, t257] });
    const t259 = "div";
    const t260 = "button";
    const t261 = () => {
      const t1 = setPage;
      const t2 = (p) => {
        const t1 = Math;
        const t2 = 0;
        const t4 = p;
        const t5 = 1;
        const t6 = t4 - t5;
        const t7 = t1.max(t2, t6);
        return t7;
      };
      const t3 = t1(t2);
      return t3;
    };
    const t262 = page;
    const t263 = 0;
    const t264 = t262 === t263;
    const t265 = "\n          Prev\n        ";
    const t266 = _jsx(t260, { onClick: t261, disabled: t264, children: t265 });
    const t267 = "span";
    const t268 = "Page ";
    const t269 = page;
    const t270 = 1;
    const t271 = t269 + t270;
    const t272 = " of ";
    const t273 = pageCount;
    const t274 = _jsxs(t267, { children: [t268, t271, t272, t273] });
    const t275 = "button";
    const t276 = () => {
      const t1 = setPage;
      const t2 = (p) => {
        const t1 = Math;
        const t3 = pageCount;
        const t4 = 1;
        const t5 = t3 - t4;
        const t7 = p;
        const t8 = 1;
        const t9 = t7 + t8;
        const t10 = t1.min(t5, t9);
        return t10;
      };
      const t3 = t1(t2);
      return t3;
    };
    const t277 = page;
    const t278 = pageCount;
    const t279 = 1;
    const t280 = t278 - t279;
    const t281 = t277 >= t280;
    const t282 = "\n          Next\n        ";
    const t283 = _jsx(t275, { onClick: t276, disabled: t281, children: t282 });
    const t284 = _jsxs(t259, { children: [t266, t274, t283] });
    t285 = _jsxs(t239, { children: [t244, t258, t284] });
    $[40] = t285;
    $[32] = filter;
    $[33] = columns;
    $[34] = pagedData;
    $[35] = page;
    $[36] = page;
    $[37] = pageCount;
    $[38] = page;
    $[39] = pageCount;
  } else {
    t285 = $[40];
  }
  return t285;
}

