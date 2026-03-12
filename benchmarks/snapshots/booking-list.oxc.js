import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
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
  const $ = _c(30);
  let bookings;
  let onCancel;
  let onReschedule;
  let onConfirm;
  let filter;
  if ($[0] !== bookings || $[1] !== onCancel || $[2] !== onReschedule || $[3] !== onConfirm || $[4] !== filter) {
    $[0] = bookings;
    $[1] = onCancel;
    $[2] = onReschedule;
    $[3] = onConfirm;
    $[4] = filter;
  } else {
  }
  ({ bookings, onCancel, onReschedule, onConfirm, filter } = t0);
  const t198 = useState;
  const t199 = null;
  const t200 = t198(t199);
  let expandedId;
  let setExpandedId;
  if ($[5] !== expandedId || $[6] !== setExpandedId) {
    $[5] = expandedId;
    $[6] = setExpandedId;
  } else {
  }
  ([expandedId, setExpandedId] = t200);
  const t204 = useState;
  const t205 = "";
  const t206 = t204(t205);
  let searchQuery;
  let setSearchQuery;
  if ($[7] !== searchQuery || $[8] !== setSearchQuery) {
    $[7] = searchQuery;
    $[8] = setSearchQuery;
  } else {
  }
  ([searchQuery, setSearchQuery] = t206);
  const t210 = useState;
  const t211 = "date";
  const t212 = t210(t211);
  let sortBy;
  let setSortBy;
  if ($[9] !== sortBy || $[10] !== setSortBy) {
    $[9] = sortBy;
    $[10] = setSortBy;
  } else {
  }
  ([sortBy, setSortBy] = t212);
  let now;
  if ($[11] !== now) {
    $[11] = now;
  } else {
  }
  const t217 = useMemo;
  const t218 = () => {
    const t0 = Date;
    const t1 = new t0();
    const t2 = t1.toISOString();
    return t2;
  };
  const t219 = [];
  const t220 = t217(t218, t219);
  now = t220;
  let filteredBookings;
  if ($[12] !== filteredBookings) {
    $[12] = filteredBookings;
  } else {
  }
  let sortedBookings;
  if ($[13] !== useMemo || $[14] !== bookings || $[15] !== filter || $[16] !== now || $[17] !== searchQuery || $[18] !== filteredBookings || $[19] !== sortedBookings) {
    const t223 = useMemo;
    const t224 = () => {
      let result;
      const t3 = bookings;
      result = t3;
      const t6 = filter;
      const t7 = "upcoming";
      const t8 = t6 === t7;
      const t10 = result;
      const t11 = (b) => {
        const t2 = b;
        const t3 = t2.startTime;
        const t5 = now;
        const t6 = t3 > t5;
        const t8 = b;
        const t9 = t8.status;
        const t10 = "cancelled";
        const t11 = t9 !== t10;
        return t12;
      };
      const t12 = t10.filter(t11);
      result = t12;
      const t16 = filter;
      const t17 = "past";
      const t18 = t16 === t17;
      const t26 = searchQuery;
      const t20 = result;
      const t21 = (b) => {
        const t2 = b;
        const t3 = t2.endTime;
        const t5 = now;
        const t6 = t3 < t5;
        return t6;
      };
      const t22 = t20.filter(t21);
      result = t22;
      let q;
      const t30 = searchQuery;
      const t31 = t30.toLowerCase();
      q = t31;
      const t34 = result;
      const t35 = (b) => {
        const t2 = b;
        const t3 = t2.title;
        const t4 = t3.toLowerCase();
        const t6 = q;
        const t7 = t4.includes(t6);
        const t9 = b;
        const t10 = t9.attendees;
        const t11 = (a) => {
          const t2 = a;
          const t3 = t2.name;
          const t4 = t3.toLowerCase();
          const t6 = q;
          const t7 = t4.includes(t6);
          const t9 = a;
          const t10 = t9.email;
          const t11 = t10.toLowerCase();
          const t13 = q;
          const t14 = t11.includes(t13);
          return t15;
        };
        const t12 = t10.some(t11);
        return t13;
      };
      const t36 = t34.filter(t35);
      result = t36;
      const t40 = result;
      return t40;
      const t41 = undefined;
      return t41;
    };
    const t225 = bookings;
    const t226 = filter;
    const t227 = now;
    const t228 = searchQuery;
    const t229 = [t225, t226, t227, t228];
    const t230 = t223(t224, t229);
    filteredBookings = t230;
    $[13] = useMemo;
    $[14] = bookings;
    $[15] = filter;
    $[16] = now;
    $[17] = searchQuery;
    $[18] = filteredBookings;
    $[19] = sortedBookings;
  } else {
  }
  let stats;
  if ($[20] !== useMemo || $[21] !== filteredBookings || $[22] !== sortBy || $[23] !== sortedBookings || $[24] !== stats) {
    const t233 = useMemo;
    const t234 = () => {
      const t1 = filteredBookings;
      const t2 = [...t1];
      const t3 = (a, b) => {
        const t3 = sortBy;
        const t4 = "date";
        const t5 = t3 === t4;
        const t7 = a;
        const t8 = t7.startTime;
        const t10 = b;
        const t11 = t10.startTime;
        const t12 = t8.localeCompare(t11);
        return t12;
        const t14 = a;
        const t15 = t14.title;
        const t17 = b;
        const t18 = t17.title;
        const t19 = t15.localeCompare(t18);
        return t19;
        const t20 = undefined;
        return t20;
      };
      const t4 = t2.sort(t3);
      return t4;
      const t5 = undefined;
      return t5;
    };
    const t235 = filteredBookings;
    const t236 = sortBy;
    const t237 = [t235, t236];
    const t238 = t233(t234, t237);
    sortedBookings = t238;
    $[20] = useMemo;
    $[21] = filteredBookings;
    $[22] = sortBy;
    $[23] = sortedBookings;
    $[24] = stats;
  } else {
  }
  let toggleExpanded;
  if ($[25] !== useMemo || $[26] !== sortedBookings || $[27] !== stats || $[28] !== toggleExpanded) {
    const t241 = useMemo;
    const t242 = () => {
      const t1 = sortedBookings;
      const t2 = t1.length;
      const t4 = sortedBookings;
      const t5 = (b) => {
        const t2 = b;
        const t3 = t2.status;
        const t4 = "confirmed";
        const t5 = t3 === t4;
        return t5;
      };
      const t6 = t4.filter(t5);
      const t7 = t6.length;
      const t9 = sortedBookings;
      const t10 = (b) => {
        const t2 = b;
        const t3 = t2.status;
        const t4 = "pending";
        const t5 = t3 === t4;
        return t5;
      };
      const t11 = t9.filter(t10);
      const t12 = t11.length;
      const t14 = sortedBookings;
      const t15 = (b) => {
        const t2 = b;
        const t3 = t2.status;
        const t4 = "cancelled";
        const t5 = t3 === t4;
        return t5;
      };
      const t16 = t14.filter(t15);
      const t17 = t16.length;
      const t18 = { total: t2, confirmed: t7, pending: t12, cancelled: t17 };
      return t18;
    };
    const t243 = sortedBookings;
    const t244 = [t243];
    const t245 = t241(t242, t244);
    stats = t245;
    $[25] = useMemo;
    $[26] = sortedBookings;
    $[27] = stats;
    $[28] = toggleExpanded;
  } else {
  }
  const t248 = useCallback;
  const t249 = (id) => {
    const t2 = setExpandedId;
    const t3 = (prev) => {
      const t2 = prev;
      const t4 = id;
      const t5 = t2 === t4;
      const t6 = null;
      const t8 = id;
      return t9;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t250 = [];
  const t251 = t248(t249, t250);
  toggleExpanded = t251;
  let handleCancel;
  if ($[29] !== handleCancel) {
    $[29] = handleCancel;
  } else {
  }
  const t254 = useCallback;
  const t255 = (id) => {
    const t2 = onCancel;
    const t4 = id;
    const t5 = t2(t4);
    const t7 = setExpandedId;
    const t8 = null;
    const t9 = t7(t8);
    const t10 = undefined;
    return t10;
  };
  const t256 = onCancel;
  const t257 = [t256];
  const t258 = t254(t255, t257);
  handleCancel = t258;
  const t260 = "div";
  const t261 = "space-y-4";
  const t262 = "div";
  const t263 = "flex justify-between items-center";
  const t264 = "div";
  const t265 = "flex gap-2 text-sm";
  const t266 = "span";
  const t267 = stats;
  const t268 = t267.total;
  const t269 = " total";
  const t270 = _jsxs(t266, { children: [t268, t269] });
  const t271 = "span";
  const t272 = "text-green-600";
  const t273 = stats;
  const t274 = t273.confirmed;
  const t275 = " confirmed";
  const t276 = _jsxs(t271, { className: t272, children: [t274, t275] });
  const t277 = "span";
  const t278 = "text-yellow-600";
  const t279 = stats;
  const t280 = t279.pending;
  const t281 = " pending";
  const t282 = _jsxs(t277, { className: t278, children: [t280, t281] });
  const t283 = "span";
  const t284 = "text-red-600";
  const t285 = stats;
  const t286 = t285.cancelled;
  const t287 = " cancelled";
  const t288 = _jsxs(t283, { className: t284, children: [t286, t287] });
  const t289 = _jsxs(t264, { className: t265, children: [t270, t276, t282, t288] });
  const t290 = "div";
  const t291 = "flex gap-2";
  const t292 = "input";
  const t293 = searchQuery;
  const t294 = (e) => {
    const t2 = setSearchQuery;
    const t4 = e;
    const t5 = t4.target;
    const t6 = t5.value;
    const t7 = t2(t6);
    return t7;
  };
  const t295 = "Search bookings...";
  const t296 = "border rounded px-2 py-1";
  const t297 = _jsx(t292, { value: t293, onChange: t294, placeholder: t295, className: t296 });
  const t298 = "select";
  const t299 = sortBy;
  const t300 = (e) => {
    const t2 = setSortBy;
    const t4 = e;
    const t5 = t4.target;
    const t6 = t5.value;
    const t7 = t2(t6);
    return t7;
  };
  const t301 = "option";
  const t302 = "date";
  const t303 = "Sort by date";
  const t304 = _jsx(t301, { value: t302, children: t303 });
  const t305 = "option";
  const t306 = "title";
  const t307 = "Sort by title";
  const t308 = _jsx(t305, { value: t306, children: t307 });
  const t309 = _jsxs(t298, { value: t299, onChange: t300, children: [t304, t308] });
  const t310 = _jsxs(t290, { className: t291, children: [t297, t309] });
  const t311 = _jsxs(t262, { className: t263, children: [t289, t310] });
  const t312 = sortedBookings;
  const t313 = t312.length;
  const t314 = 0;
  const t315 = t313 === t314;
}

