import { c as _c } from "react/compiler-runtime";
// M tier - Inspired by cal.com BookingListItem with scheduling logic
import { useState, useMemo, useCallback, useEffect } from 'react';

interface Booking {
  id: string;
  title: string;
  startTime: string;
  endTime: string;
  attendees: { name: string; email: string; status: 'accepted' | 'pending' | 'declined' }[];
  status: 'confirmed' | 'pending' | 'cancelled';
  location?: string;
  notes?: string;
}

interface BookingListProps {
  bookings: Booking[];
  onCancel: (id: string) => void;
  onReschedule: (id: string) => void;
  onConfirm: (id: string) => void;
  filter?: 'all' | 'upcoming' | 'past';
}

export function BookingList(t0) {
  const $ = _c(9);
  const t181 = useState;
  const t182 = null;
  const t183 = t181(t182);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t184 = Discriminant(4) */
    /* t185 = Discriminant(4) */
  } else {
  }
  /* t186 = Discriminant(6) */
  const t187 = useState;
  const t188 = "";
  const t189 = t187(t188);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t190 = Discriminant(4) */
    /* t191 = Discriminant(4) */
  } else {
  }
  /* t192 = Discriminant(6) */
  const t193 = useState;
  const t194 = "date";
  const t195 = t193(t194);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t196 = Discriminant(4) */
    /* t197 = Discriminant(4) */
  } else {
  }
  /* t198 = Discriminant(6) */
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t199 = Discriminant(4) */
  } else {
  }
  const t200 = useMemo;
  /* t201 = Discriminant(28) */
  const t202 = [];
  const t203 = t200(t201, t202);
  const now = t203;
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    /* t205 = Discriminant(4) */
  } else {
  }
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    const t206 = useMemo;
    /* t207 = Discriminant(28) */
    const t208 = bookings;
    const t209 = filter;
    const t210 = now;
    const t211 = searchQuery;
    const t212 = [t208, t209, t210, t211];
    const t213 = t206(t207, t212);
    const filteredBookings = t213;
    /* t215 = Discriminant(4) */
  } else {
  }
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    const t216 = useMemo;
    /* t217 = Discriminant(28) */
    const t218 = filteredBookings;
    const t219 = sortBy;
    const t220 = [t218, t219];
    const t221 = t216(t217, t220);
    const sortedBookings = t221;
    /* t223 = Discriminant(4) */
  } else {
  }
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    const t224 = useMemo;
    /* t225 = Discriminant(28) */
    const t226 = sortedBookings;
    const t227 = [t226];
    const t228 = t224(t225, t227);
    const stats = t228;
    /* t230 = Discriminant(4) */
  } else {
  }
  const t231 = useCallback;
  /* t232 = Discriminant(28) */
  const t233 = [];
  const t234 = t231(t232, t233);
  const toggleExpanded = t234;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    /* t236 = Discriminant(4) */
  } else {
  }
  const t237 = useCallback;
  /* t238 = Discriminant(28) */
  const t239 = onCancel;
  const t240 = [t239];
  const t241 = t237(t238, t240);
  const handleCancel = t241;
  const t243 = "div";
  const t244 = "space-y-4";
  const t245 = "div";
  const t246 = "flex justify-between items-center";
  const t247 = "div";
  const t248 = "flex gap-2 text-sm";
  const t249 = "span";
  const t250 = stats;
  const t251 = t250.total;
  /* t252 = Discriminant(8) */
  const t253 = <t249>{t251}{t252}</t249>;
  const t254 = "span";
  const t255 = "text-green-600";
  const t256 = stats;
  const t257 = t256.confirmed;
  /* t258 = Discriminant(8) */
  const t259 = <t254 className={t255}>{t257}{t258}</t254>;
  const t260 = "span";
  const t261 = "text-yellow-600";
  const t262 = stats;
  const t263 = t262.pending;
  /* t264 = Discriminant(8) */
  const t265 = <t260 className={t261}>{t263}{t264}</t260>;
  const t266 = "span";
  const t267 = "text-red-600";
  const t268 = stats;
  const t269 = t268.cancelled;
  /* t270 = Discriminant(8) */
  const t271 = <t266 className={t267}>{t269}{t270}</t266>;
  const t272 = <t247 className={t248}>{t253}{t259}{t265}{t271}</t247>;
  const t273 = "div";
  const t274 = "flex gap-2";
  const t275 = "input";
  const t276 = searchQuery;
  /* t277 = Discriminant(28) */
  const t278 = "Search bookings...";
  const t279 = "border rounded px-2 py-1";
  const t280 = <t275 value={t276} onChange={t277} placeholder={t278} className={t279} />;
  const t281 = "select";
  const t282 = sortBy;
  /* t283 = Discriminant(28) */
  const t284 = "option";
  const t285 = "date";
  /* t286 = Discriminant(8) */
  const t287 = <t284 value={t285}>{t286}</t284>;
  const t288 = "option";
  const t289 = "title";
  /* t290 = Discriminant(8) */
  const t291 = <t288 value={t289}>{t290}</t288>;
  const t292 = <t281 value={t282} onChange={t283}>{t287}{t291}</t281>;
  const t293 = <t273 className={t274}>{t280}{t292}</t273>;
  const t294 = <t245 className={t246}>{t272}{t293}</t245>;
  const t295 = sortedBookings;
  const t296 = t295.length;
  const t297 = 0;
  const t298 = t296 === t297;
}

