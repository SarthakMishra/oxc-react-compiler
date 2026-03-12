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
  const $ = _c(11);
  const t164 = useState;
  const t165 = null;
  const t166 = t164(t165);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t167 = Discriminant(4) */
    /* t168 = Discriminant(4) */
  } else {
  }
  /* t169 = Discriminant(6) */
  const t170 = useState;
  const t171 = "asc";
  const t172 = t170(t171);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t173 = Discriminant(4) */
    /* t174 = Discriminant(4) */
  } else {
  }
  /* t175 = Discriminant(6) */
  const t176 = useState;
  const t177 = "";
  const t178 = t176(t177);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t179 = Discriminant(4) */
    /* t180 = Discriminant(4) */
  } else {
  }
  /* t181 = Discriminant(6) */
  const t182 = useState;
  const t183 = 0;
  const t184 = t182(t183);
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t185 = Discriminant(4) */
    /* t186 = Discriminant(4) */
  } else {
  }
  /* t187 = Discriminant(6) */
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    /* t188 = Discriminant(4) */
  } else {
  }
  const t189 = 10;
  const pageSize = t189;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    /* t191 = Discriminant(4) */
  } else {
  }
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    const t192 = useMemo;
    /* t193 = Discriminant(28) */
    const t194 = data;
    const t195 = columns;
    const t196 = filter;
    const t197 = [t194, t195, t196];
    const t198 = t192(t193, t197);
    const filteredData = t198;
    /* t200 = Discriminant(4) */
  } else {
  }
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    const t201 = useMemo;
    /* t202 = Discriminant(28) */
    const t203 = filteredData;
    const t204 = sortKey;
    const t205 = sortDir;
    const t206 = [t203, t204, t205];
    const t207 = t201(t202, t206);
    const sortedData = t207;
    /* t209 = Discriminant(4) */
    /* t210 = Discriminant(30) */
    const t211 = sortedData;
    const t212 = t211.length;
    const t213 = pageSize;
    const t214 = t212 / t213;
    const t215 = t210.ceil(t214);
    const pageCount = t215;
    /* t217 = Discriminant(4) */
  } else {
  }
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    const t218 = useMemo;
    /* t219 = Discriminant(28) */
    const t220 = sortedData;
    const t221 = page;
    const t222 = [t220, t221];
    const t223 = t218(t219, t222);
    const pagedData = t223;
    /* t225 = Discriminant(4) */
  } else {
  }
  const t226 = useCallback;
  /* t227 = Discriminant(28) */
  const t228 = [];
  const t229 = t226(t227, t228);
  const handleSort = t229;
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    const t231 = "div";
    const t232 = "input";
    const t233 = filter;
    /* t234 = Discriminant(28) */
    const t235 = "Filter...";
    const t236 = <t232 value={t233} onChange={t234} placeholder={t235} />;
    const t237 = "table";
    const t238 = "thead";
    const t239 = "tr";
    const t240 = columns;
    /* t241 = Discriminant(28) */
    const t242 = t240.map(t241);
    const t243 = <t239>{t242}</t239>;
    const t244 = <t238>{t243}</t238>;
    const t245 = "tbody";
    const t246 = pagedData;
    /* t247 = Discriminant(28) */
    const t248 = t246.map(t247);
    const t249 = <t245>{t248}</t245>;
    const t250 = <t237>{t244}{t249}</t237>;
    const t251 = "div";
    const t252 = "button";
    /* t253 = Discriminant(28) */
    const t254 = page;
    const t255 = 0;
    const t256 = t254 === t255;
    /* t257 = Discriminant(8) */
    const t258 = <t252 onClick={t253} disabled={t256}>{t257}</t252>;
    const t259 = "span";
    /* t260 = Discriminant(8) */
    const t261 = page;
    const t262 = 1;
    const t263 = t261 + t262;
    /* t264 = Discriminant(8) */
    const t265 = pageCount;
    const t266 = <t259>{t260}{t263}{t264}{t265}</t259>;
    const t267 = "button";
    /* t268 = Discriminant(28) */
    const t269 = page;
    const t270 = pageCount;
    const t271 = 1;
    const t272 = t270 - t271;
    const t273 = t269 >= t272;
    /* t274 = Discriminant(8) */
    const t275 = <t267 onClick={t268} disabled={t273}>{t274}</t267>;
    const t276 = <t251>{t258}{t266}{t275}</t251>;
    const t277 = <t231>{t236}{t250}{t276}</t231>;
    $[10] = t277;
  } else {
    t277 = $[10];
  }
  return t277;
}

