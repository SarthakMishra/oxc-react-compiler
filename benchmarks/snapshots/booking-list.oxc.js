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
  const $ = _c(45);
  const { bookings, onCancel, onReschedule, onConfirm, filter } = t0;
  if ($[0] !== bookings || $[1] !== onCancel || $[2] !== onReschedule || $[3] !== onConfirm || $[4] !== filter) {
    $[0] = bookings;
    $[1] = onCancel;
    $[2] = onReschedule;
    $[3] = onConfirm;
    $[4] = filter;
  } else {
  }
  const t201 = useState;
  const t202 = null;
  const t203 = t201(t202);
  let expandedId;
  let setExpandedId;
  if ($[5] !== expandedId || $[6] !== setExpandedId) {
    $[5] = expandedId;
    $[6] = setExpandedId;
  } else {
  }
  ([expandedId, setExpandedId] = t203);
  const t207 = useState;
  const t208 = "";
  const t209 = t207(t208);
  let searchQuery;
  let setSearchQuery;
  if ($[7] !== searchQuery || $[8] !== setSearchQuery) {
    $[7] = searchQuery;
    $[8] = setSearchQuery;
  } else {
  }
  ([searchQuery, setSearchQuery] = t209);
  const t213 = useState;
  const t214 = "date";
  const t215 = t213(t214);
  let sortBy;
  let setSortBy;
  if ($[9] !== sortBy || $[10] !== setSortBy) {
    $[9] = sortBy;
    $[10] = setSortBy;
  } else {
  }
  ([sortBy, setSortBy] = t215);
  let now;
  if ($[11] !== now) {
    $[11] = now;
  } else {
  }
  const t220 = useMemo;
  const t221 = () => {
    const t0 = Date;
    const t1 = new t0();
    const t2 = t1.toISOString();
    return t2;
  };
  const t222 = [];
  const t223 = t220(t221, t222);
  now = t223;
  let filteredBookings;
  if ($[12] !== filteredBookings) {
    $[12] = filteredBookings;
  } else {
  }
  let sortedBookings;
  if ($[13] !== useMemo || $[14] !== bookings || $[15] !== filter || $[16] !== now || $[17] !== searchQuery || $[18] !== filteredBookings || $[19] !== sortedBookings) {
    const t226 = useMemo;
    const t227 = () => {
      let result;
      const t3 = bookings;
      result = t3;
      const t6 = filter;
      const t7 = "upcoming";
      const t8 = t6 === t7;
      if (t8) {
        const t10 = result;
        const t11 = (b) => {
          let t1;
          const t4 = b;
          const t5 = t4.startTime;
          const t7 = now;
          const t8 = t5 > t7;
          t1 = t8;
          const t11 = b;
          const t12 = t11.status;
          const t13 = "cancelled";
          const t14 = t12 !== t13;
          t1 = t14;
          return t1;
        };
        const t12 = t10.filter(t11);
        result = t12;
      } else {
        const t16 = filter;
        const t17 = "past";
        const t18 = t16 === t17;
        if (t18) {
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
        } else {
        }
      }
      const t26 = searchQuery;
      if (t26) {
        let q;
        const t30 = searchQuery;
        const t31 = t30.toLowerCase();
        q = t31;
        const t34 = result;
        const t35 = (b) => {
          let t1;
          const t4 = b;
          const t5 = t4.title;
          const t6 = t5.toLowerCase();
          const t8 = q;
          const t9 = t6.includes(t8);
          t1 = t9;
          const t12 = b;
          const t13 = t12.attendees;
          const t14 = (a) => {
            let t1;
            const t4 = a;
            const t5 = t4.name;
            const t6 = t5.toLowerCase();
            const t8 = q;
            const t9 = t6.includes(t8);
            t1 = t9;
            const t12 = a;
            const t13 = t12.email;
            const t14 = t13.toLowerCase();
            const t16 = q;
            const t17 = t14.includes(t16);
            t1 = t17;
            return t1;
          };
          const t15 = t13.some(t14);
          t1 = t15;
          return t1;
        };
        const t36 = t34.filter(t35);
        result = t36;
      } else {
      }
      const t40 = result;
      return t40;
    };
    const t228 = bookings;
    const t229 = filter;
    const t230 = now;
    const t231 = searchQuery;
    const t232 = [t228, t229, t230, t231];
    const t233 = t226(t227, t232);
    filteredBookings = t233;
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
    const t236 = useMemo;
    const t237 = () => {
      const t1 = filteredBookings;
      const t2 = [...t1];
      const t3 = (a, b) => {
        const t3 = sortBy;
        const t4 = "date";
        const t5 = t3 === t4;
        if (t5) {
          const t7 = a;
          const t8 = t7.startTime;
          const t10 = b;
          const t11 = t10.startTime;
          const t12 = t8.localeCompare(t11);
          return t12;
        } else {
        }
        const t14 = a;
        const t15 = t14.title;
        const t17 = b;
        const t18 = t17.title;
        const t19 = t15.localeCompare(t18);
        return t19;
      };
      const t4 = t2.sort(t3);
      return t4;
    };
    const t238 = filteredBookings;
    const t239 = sortBy;
    const t240 = [t238, t239];
    const t241 = t236(t237, t240);
    sortedBookings = t241;
    $[20] = useMemo;
    $[21] = filteredBookings;
    $[22] = sortBy;
    $[23] = sortedBookings;
    $[24] = stats;
  } else {
  }
  let toggleExpanded;
  if ($[25] !== useMemo || $[26] !== sortedBookings || $[27] !== stats || $[28] !== toggleExpanded) {
    const t244 = useMemo;
    const t245 = () => {
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
    const t246 = sortedBookings;
    const t247 = [t246];
    const t248 = t244(t245, t247);
    stats = t248;
    $[25] = useMemo;
    $[26] = sortedBookings;
    $[27] = stats;
    $[28] = toggleExpanded;
  } else {
  }
  const t251 = useCallback;
  const t252 = (id) => {
    const t2 = setExpandedId;
    const t3 = (prev) => {
      const t2 = prev;
      const t4 = id;
      const t5 = t2 === t4;
      let t6;
      if (t5) {
        const t8 = null;
        t6 = t8;
      } else {
        const t11 = id;
        t6 = t11;
      }
      return t6;
    };
    const t4 = t2(t3);
    const t5 = undefined;
    return t5;
  };
  const t253 = [];
  const t254 = t251(t252, t253);
  toggleExpanded = t254;
  let handleCancel;
  if ($[29] !== handleCancel) {
    $[29] = handleCancel;
  } else {
  }
  let t178;
  if ($[30] !== useCallback || $[31] !== onCancel || $[32] !== handleCancel || $[33] !== sortedBookings || $[34] !== t317 || $[35] !== t178) {
    const t257 = useCallback;
    const t258 = (id) => {
      const t2 = onCancel;
      const t4 = id;
      const t5 = t2(t4);
      const t7 = setExpandedId;
      const t8 = null;
      const t9 = t7(t8);
      const t10 = undefined;
      return t10;
    };
    const t259 = onCancel;
    const t260 = [t259];
    const t261 = t257(t258, t260);
    handleCancel = t261;
    const t263 = "div";
    const t264 = "space-y-4";
    const t265 = "div";
    const t266 = "flex justify-between items-center";
    const t267 = "div";
    const t268 = "flex gap-2 text-sm";
    const t269 = "span";
    const t270 = stats;
    const t271 = t270.total;
    const t272 = " total";
    const t273 = _jsxs(t269, { children: [t271, t272] });
    const t274 = "span";
    const t275 = "text-green-600";
    const t276 = stats;
    const t277 = t276.confirmed;
    const t278 = " confirmed";
    const t279 = _jsxs(t274, { className: t275, children: [t277, t278] });
    const t280 = "span";
    const t281 = "text-yellow-600";
    const t282 = stats;
    const t283 = t282.pending;
    const t284 = " pending";
    const t285 = _jsxs(t280, { className: t281, children: [t283, t284] });
    const t286 = "span";
    const t287 = "text-red-600";
    const t288 = stats;
    const t289 = t288.cancelled;
    const t290 = " cancelled";
    const t291 = _jsxs(t286, { className: t287, children: [t289, t290] });
    const t292 = _jsxs(t267, { className: t268, children: [t273, t279, t285, t291] });
    const t293 = "div";
    const t294 = "flex gap-2";
    const t295 = "input";
    const t296 = searchQuery;
    const t297 = (e) => {
      const t2 = setSearchQuery;
      const t4 = e;
      const t5 = t4.target;
      const t6 = t5.value;
      const t7 = t2(t6);
      return t7;
    };
    const t298 = "Search bookings...";
    const t299 = "border rounded px-2 py-1";
    const t300 = _jsx(t295, { value: t296, onChange: t297, placeholder: t298, className: t299 });
    const t301 = "select";
    const t302 = sortBy;
    const t303 = (e) => {
      const t2 = setSortBy;
      const t4 = e;
      const t5 = t4.target;
      const t6 = t5.value;
      const t7 = t2(t6);
      return t7;
    };
    const t304 = "option";
    const t305 = "date";
    const t306 = "Sort by date";
    const t307 = _jsx(t304, { value: t305, children: t306 });
    const t308 = "option";
    const t309 = "title";
    const t310 = "Sort by title";
    const t311 = _jsx(t308, { value: t309, children: t310 });
    const t312 = _jsxs(t301, { value: t302, onChange: t303, children: [t307, t311] });
    const t313 = _jsxs(t293, { className: t294, children: [t300, t312] });
    const t314 = _jsxs(t265, { className: t266, children: [t292, t313] });
    const t315 = sortedBookings;
    const t316 = t315.length;
    const t317 = 0;
    const t318 = t316 === t317;
    $[30] = useCallback;
    $[31] = onCancel;
    $[32] = handleCancel;
    $[33] = sortedBookings;
    $[34] = t317;
    $[35] = t178;
  } else {
  }
  if (t318) {
    const t340 = "p";
    const t341 = "text-gray-500 text-center py-8";
    const t342 = "No bookings found";
    const t343 = _jsx(t340, { className: t341, children: t342 });
    t178 = t343;
  } else {
    let t339;
    if ($[36] !== t178 || $[37] !== sortedBookings || $[38] !== stats || $[39] !== stats || $[40] !== stats || $[41] !== stats || $[42] !== searchQuery || $[43] !== sortBy) {
      const t320 = "ul";
      const t321 = "divide-y";
      const t322 = sortedBookings;
      const t323 = (booking) => {
        const t1 = "li";
        const t3 = booking;
        const t4 = t3.id;
        const t5 = "py-3";
        const t6 = "div";
        const t7 = "flex justify-between items-start cursor-pointer";
        const t8 = () => {
          const t1 = toggleExpanded;
          const t3 = booking;
          const t4 = t3.id;
          const t5 = t1(t4);
          return t5;
        };
        const t9 = "div";
        const t10 = "h3";
        const t11 = "font-medium";
        const t13 = booking;
        const t14 = t13.title;
        const t15 = _jsx(t10, { className: t11, children: t14 });
        const t16 = "p";
        const t17 = "text-sm text-gray-500";
        const t19 = booking;
        const t20 = t19.startTime;
        const t21 = " - ";
        const t23 = booking;
        const t24 = t23.endTime;
        const t25 = _jsxs(t16, { className: t17, children: [t20, t21, t24] });
        const t26 = "p";
        const t27 = "text-sm";
        const t29 = booking;
        const t30 = t29.attendees;
        const t31 = t30.length;
        const t32 = " attendee(s)";
        const t33 = _jsxs(t26, { className: t27, children: [t31, t32] });
        const t34 = _jsxs(t9, { children: [t15, t25, t33] });
        const t35 = "span";
        const t37 = booking;
        const t38 = t37.status;
        const t39 = "confirmed";
        const t40 = t38 === t39;
        let t41;
        if (t40) {
          const t43 = "bg-green-100";
          t41 = t43;
        } else {
          const t46 = booking;
          const t47 = t46.status;
          const t48 = "pending";
          const t49 = t47 === t48;
          let t50;
          if (t49) {
            const t52 = "bg-yellow-100";
            t50 = t52;
          } else {
            const t54 = "bg-red-100";
            t50 = t54;
          }
          t41 = t50;
        }
        const t57 = `px-2 py-1 rounded text-xs ${t41}`;
        const t59 = booking;
        const t60 = t59.status;
        const t61 = _jsx(t35, { className: t57, children: t60 });
        const t62 = _jsxs(t6, { className: t7, onClick: t8, children: [t34, t61] });
        let t63;
        const t66 = expandedId;
        const t68 = booking;
        const t69 = t68.id;
        const t70 = t66 === t69;
        t63 = t70;
        const t72 = "div";
        const t73 = "mt-2 pl-4 border-l-2";
        let t74;
        const t77 = booking;
        const t78 = t77.location;
        t74 = t78;
        const t80 = "p";
        const t81 = "text-sm";
        const t82 = "📍 ";
        const t84 = booking;
        const t85 = t84.location;
        const t86 = _jsxs(t80, { className: t81, children: [t82, t85] });
        t74 = t86;
        let t88;
        const t91 = booking;
        const t92 = t91.notes;
        t88 = t92;
        const t94 = "p";
        const t95 = "text-sm text-gray-600";
        const t97 = booking;
        const t98 = t97.notes;
        const t99 = _jsx(t94, { className: t95, children: t98 });
        t88 = t99;
        const t101 = "div";
        const t102 = "mt-2";
        const t103 = "h4";
        const t104 = "text-xs font-semibold uppercase";
        const t105 = "Attendees";
        const t106 = _jsx(t103, { className: t104, children: t105 });
        const t108 = booking;
        const t109 = t108.attendees;
        const t110 = (a, i) => {
          const t2 = "div";
          const t4 = i;
          const t5 = "text-sm flex justify-between";
          const t6 = "span";
          const t8 = a;
          const t9 = t8.name;
          const t10 = " (";
          const t12 = a;
          const t13 = t12.email;
          const t14 = ")";
          const t15 = _jsxs(t6, { children: [t9, t10, t13, t14] });
          const t16 = "span";
          const t18 = a;
          const t19 = t18.status;
          const t20 = _jsx(t16, { children: t19 });
          const t21 = _jsxs(t2, { key: t4, className: t5, children: [t15, t20] });
          return t21;
        };
        const t111 = t109.map(t110);
        const t112 = _jsxs(t101, { className: t102, children: [t106, t111] });
        const t113 = "div";
        const t114 = "mt-2 flex gap-2";
        let t115;
        const t118 = booking;
        const t119 = t118.status;
        const t120 = "pending";
        const t121 = t119 === t120;
        t115 = t121;
        const t123 = "button";
        const t124 = () => {
          const t1 = onConfirm;
          const t3 = booking;
          const t4 = t3.id;
          const t5 = t1(t4);
          return t5;
        };
        const t125 = "text-green-600";
        const t126 = "Confirm";
        const t127 = _jsx(t123, { onClick: t124, className: t125, children: t126 });
        t115 = t127;
        const t129 = "button";
        const t130 = () => {
          const t1 = onReschedule;
          const t3 = booking;
          const t4 = t3.id;
          const t5 = t1(t4);
          return t5;
        };
        const t131 = "text-blue-600";
        const t132 = "Reschedule";
        const t133 = _jsx(t129, { onClick: t130, className: t131, children: t132 });
        let t134;
        const t137 = booking;
        const t138 = t137.status;
        const t139 = "cancelled";
        const t140 = t138 !== t139;
        t134 = t140;
        const t142 = "button";
        const t143 = () => {
          const t1 = handleCancel;
          const t3 = booking;
          const t4 = t3.id;
          const t5 = t1(t4);
          return t5;
        };
        const t144 = "text-red-600";
        const t145 = "Cancel";
        const t146 = _jsx(t142, { onClick: t143, className: t144, children: t145 });
        t134 = t146;
        const t148 = _jsxs(t113, { className: t114, children: [t115, t133, t134] });
        const t149 = _jsxs(t72, { className: t73, children: [t74, t88, t112, t148] });
        t63 = t149;
        const t151 = _jsxs(t1, { key: t4, className: t5, children: [t62, t63] });
        return t151;
      };
      const t324 = t322.map(t323);
      const t325 = _jsx(t320, { className: t321, children: t324 });
      t178 = t325;
      $[44] = t339;
      $[36] = t178;
      $[37] = sortedBookings;
      $[38] = stats;
      $[39] = stats;
      $[40] = stats;
      $[41] = stats;
      $[42] = searchQuery;
      $[43] = sortBy;
    } else {
      t339 = $[44];
    }
  }
  t339 = _jsxs(t263, { className: t264, children: [t314, t178] });
  return t339;
}

