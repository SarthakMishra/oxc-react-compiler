import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// L tier - Inspired by cal.com availability schedule editor
import { useState, useMemo, useCallback, useReducer } from 'react';

interface TimeRange {
  start: string; // HH:MM
  end: string;
}

interface DaySchedule {
  enabled: boolean;
  ranges: TimeRange[];
}

type WeekSchedule = Record<string, DaySchedule>;

const DAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

const DEFAULT_RANGE: TimeRange = { start: '09:00', end: '17:00' };

type Action =
  | { type: 'TOGGLE_DAY'; day: string }
  | { type: 'ADD_RANGE'; day: string }
  | { type: 'REMOVE_RANGE'; day: string; index: number }
  | { type: 'UPDATE_RANGE'; day: string; index: number; field: 'start' | 'end'; value: string }
  | { type: 'COPY_TO_ALL'; sourceDay: string }
  | { type: 'RESET' };

function scheduleReducer(state: WeekSchedule, action: Action): WeekSchedule {
  switch (action.type) {
    case 'TOGGLE_DAY':
      return {
        ...state,
        [action.day]: {
          ...state[action.day],
          enabled: !state[action.day].enabled,
        },
      };
    case 'ADD_RANGE':
      return {
        ...state,
        [action.day]: {
          ...state[action.day],
          ranges: [...state[action.day].ranges, { ...DEFAULT_RANGE }],
        },
      };
    case 'REMOVE_RANGE':
      return {
        ...state,
        [action.day]: {
          ...state[action.day],
          ranges: state[action.day].ranges.filter((_, i) => i !== action.index),
        },
      };
    case 'UPDATE_RANGE': {
      const ranges = [...state[action.day].ranges];
      ranges[action.index] = { ...ranges[action.index], [action.field]: action.value };
      return { ...state, [action.day]: { ...state[action.day], ranges } };
    }
    case 'COPY_TO_ALL': {
      const source = state[action.sourceDay];
      const next = { ...state };
      for (const day of DAYS) {
        if (day !== action.sourceDay) {
          next[day] = { enabled: source.enabled, ranges: source.ranges.map((r) => ({ ...r })) };
        }
      }
      return next;
    }
    case 'RESET': {
      const initial: WeekSchedule = {};
      for (const day of DAYS) {
        initial[day] = { enabled: day !== 'Saturday' && day !== 'Sunday', ranges: [{ ...DEFAULT_RANGE }] };
      }
      return initial;
    }
    default:
      return state;
  }
}

function createInitialSchedule(): WeekSchedule {
  const schedule: WeekSchedule = {};
  for (const day of DAYS) {
    schedule[day] = {
      enabled: day !== 'Saturday' && day !== 'Sunday',
      ranges: [{ ...DEFAULT_RANGE }],
    };
  }
  return schedule;
}

interface AvailabilityScheduleProps {
  initialSchedule?: WeekSchedule;
  onSave: (schedule: WeekSchedule) => void;
  timezone?: string;
}

export function AvailabilitySchedule(t0) {
  const $ = _c(18);
  const { initialSchedule, onSave, timezone } = t0;
  if ($[0] !== initialSchedule || $[1] !== onSave || $[2] !== timezone) {
    $[0] = initialSchedule;
    $[1] = onSave;
    $[2] = timezone;
  } else {
  }
  const t198 = useReducer;
  const t199 = scheduleReducer;
  let t12;
  if ($[3] !== initialSchedule || $[4] !== t12) {
    $[3] = initialSchedule;
    $[4] = t12;
  } else {
  }
  const t201 = initialSchedule;
  t12 = t201;
  const t340 = createInitialSchedule;
  const t341 = t340();
  t12 = t341;
  const t206 = t198(t199, t12);
  let schedule;
  let dispatch;
  ([schedule, dispatch] = t206);
  const t210 = useState;
  const t211 = false;
  const t212 = t210(t211);
  let isDirty;
  let setIsDirty;
  ([isDirty, setIsDirty] = t212);
  const t216 = useState;
  const t217 = null;
  const t218 = t216(t217);
  let copySource;
  let setCopySource;
  ([copySource, setCopySource] = t218);
  let totalHours;
  const t223 = useMemo;
  const t224 = () => {
    let total;
    const t2 = 0;
    total = t2;
    const t5 = DAYS;
    const t6 = t5[Symbol.iterator]();
    const t7 = t6.next();
    let day;
    day = t7;
    let daySchedule;
    const t14 = schedule;
    const t16 = day;
    const t17 = t14[t16];
    daySchedule = t17;
    const t20 = daySchedule;
    const t21 = t20.enabled;
    const t22 = !t21;
    if (t22) {
    } else {
    }
    const t24 = daySchedule;
    const t25 = t24.ranges;
    const t26 = t25[Symbol.iterator]();
    const t27 = t26.next();
    let range;
    range = t27;
    const t32 = range;
    const t33 = t32.start;
    const t34 = ":";
    const t35 = t33.split(t34);
    const t36 = Number;
    const t37 = t35.map(t36);
    let startH;
    let startM;
    ([startH, startM] = t37);
    const t44 = range;
    const t45 = t44.end;
    const t46 = ":";
    const t47 = t45.split(t46);
    const t48 = Number;
    const t49 = t47.map(t48);
    let endH;
    let endM;
    ([endH, endM] = t49);
    const t56 = endH;
    const t57 = 60;
    const t58 = t56 * t57;
    const t60 = endM;
    const t61 = t58 + t60;
    const t63 = startH;
    const t64 = 60;
    const t65 = t63 * t64;
    const t66 = t61 - t65;
    const t68 = startM;
    const t69 = t66 - t68;
    const t70 = 60;
    const t71 = t69 / t70;
    const t73 = total;
    const t74 = t73 + t71;
    total = t74;
  };
  const t225 = schedule;
  const t226 = [t225];
  const t227 = t223(t224, t226);
  totalHours = t227;
  let hasOverlaps;
  const t230 = useMemo;
  const t231 = () => {
    const t1 = DAYS;
    const t2 = t1[Symbol.iterator]();
    const t3 = t2.next();
    let day;
    day = t3;
    let daySchedule;
    const t10 = schedule;
    const t12 = day;
    const t13 = t10[t12];
    daySchedule = t13;
    const t16 = daySchedule;
    const t17 = t16.enabled;
    const t18 = !t17;
    if (t18) {
    } else {
    }
    let sorted;
    const t22 = daySchedule;
    const t23 = t22.ranges;
    const t24 = [...t23];
    const t25 = (a, b) => {
      const t3 = a;
      const t4 = t3.start;
      const t6 = b;
      const t7 = t6.start;
      const t8 = t4.localeCompare(t7);
      return t8;
    };
    const t26 = t24.sort(t25);
    sorted = t26;
    let i;
    const t30 = 1;
    i = t30;
    const t33 = i;
    const t35 = sorted;
    const t36 = t35.length;
    const t37 = t33 < t36;
    if (t37) {
      const t39 = sorted;
      const t41 = i;
      const t42 = t39[t41];
      const t43 = t42.start;
      const t45 = sorted;
      const t47 = i;
      const t48 = 1;
      const t49 = t47 - t48;
      const t50 = t45[t49];
      const t51 = t50.end;
      const t52 = t43 < t51;
      if (t52) {
        const t53 = true;
        return t53;
      } else {
      }
      const t55 = i++;
    } else {
    }
  };
  const t232 = schedule;
  const t233 = [t232];
  const t234 = t230(t231, t233);
  hasOverlaps = t234;
  let handleChange;
  const t237 = useCallback;
  const t238 = (action) => {
    const t2 = dispatch;
    const t4 = action;
    const t5 = t2(t4);
    const t7 = setIsDirty;
    const t8 = true;
    const t9 = t7(t8);
    const t10 = undefined;
    return t10;
  };
  const t239 = [];
  const t240 = t237(t238, t239);
  handleChange = t240;
  let handleSave;
  if ($[5] !== useCallback || $[6] !== schedule || $[7] !== onSave || $[8] !== handleSave) {
    const t243 = useCallback;
    const t244 = () => {
      const t1 = onSave;
      const t3 = schedule;
      const t4 = t1(t3);
      const t6 = setIsDirty;
      const t7 = false;
      const t8 = t6(t7);
      const t9 = undefined;
      return t9;
    };
    const t245 = schedule;
    const t246 = onSave;
    const t247 = [t245, t246];
    const t248 = t243(t244, t247);
    handleSave = t248;
    $[5] = useCallback;
    $[6] = schedule;
    $[7] = onSave;
    $[8] = handleSave;
  } else {
  }
  let handleCopyToAll;
  const t251 = useCallback;
  const t252 = (day) => {
    const t2 = handleChange;
    const t3 = "COPY_TO_ALL";
    const t5 = day;
    const t6 = { type: t3, sourceDay: t5 };
    const t7 = t2(t6);
    const t9 = setCopySource;
    const t11 = day;
    const t12 = t9(t11);
    const t13 = setTimeout;
    const t14 = () => {
      const t1 = setCopySource;
      const t2 = null;
      const t3 = t1(t2);
      return t3;
    };
    const t15 = 2000;
    const t16 = t13(t14, t15);
    const t17 = undefined;
    return t17;
  };
  const t253 = handleChange;
  const t254 = [t253];
  const t255 = t251(t252, t254);
  handleCopyToAll = t255;
  let handleReset;
  const t258 = useCallback;
  const t259 = () => {
    const t1 = handleChange;
    const t2 = "RESET";
    const t3 = { type: t2 };
    const t4 = t1(t3);
    const t5 = undefined;
    return t5;
  };
  const t260 = handleChange;
  const t261 = [t260];
  const t262 = t258(t259, t261);
  handleReset = t262;
  const t264 = "div";
  const t265 = "space-y-4";
  const t266 = "div";
  const t267 = "flex justify-between items-center";
  const t268 = "div";
  const t269 = "h2";
  const t270 = "text-lg font-semibold";
  const t271 = "Availability";
  const t272 = _jsx(t269, { className: t270, children: t271 });
  const t273 = "p";
  const t274 = "text-sm text-gray-500";
  const t275 = totalHours;
  const t276 = " hours/week · Timezone: ";
  const t277 = timezone;
  const t278 = _jsxs(t273, { className: t274, children: [t275, t276, t277] });
  const t279 = _jsxs(t268, { children: [t272, t278] });
  const t280 = "div";
  const t281 = "flex gap-2";
  const t282 = "button";
  const t283 = handleReset;
  const t284 = "text-sm text-gray-500 hover:text-gray-700";
  const t285 = "\n            Reset\n          ";
  const t286 = _jsx(t282, { onClick: t283, className: t284, children: t285 });
  const t287 = "button";
  const t288 = handleSave;
  let t146;
  const t290 = isDirty;
  const t291 = !t290;
  t146 = t291;
  const t338 = hasOverlaps;
  t146 = t338;
  let t155;
  const t296 = isDirty;
  t155 = t296;
  const t335 = hasOverlaps;
  const t336 = !t335;
  t155 = t336;
  let t164;
  if (t155) {
    const t333 = "bg-blue-600 hover:bg-blue-700";
    t164 = t333;
  } else {
    const t302 = "bg-gray-300 cursor-not-allowed";
    t164 = t302;
  }
  let t332;
  if ($[9] !== DAYS || $[10] !== t175 || $[11] !== totalHours || $[12] !== timezone || $[13] !== handleReset || $[14] !== handleSave || $[15] !== t164 || $[16] !== t146) {
    const t308 = `px-4 py-2 rounded text-white text-sm ${t164}`;
    const t309 = "\n            Save\n          ";
    const t310 = _jsx(t287, { onClick: t288, disabled: t146, className: t308, children: t309 });
    const t311 = _jsxs(t280, { className: t281, children: [t286, t310] });
    const t312 = _jsxs(t266, { className: t267, children: [t279, t311] });
    $[17] = t332;
    $[9] = DAYS;
    $[10] = t175;
    $[11] = totalHours;
    $[12] = timezone;
    $[13] = handleReset;
    $[14] = handleSave;
    $[15] = t164;
    $[16] = t146;
  } else {
    t332 = $[17];
  }
  let t175;
  const t314 = hasOverlaps;
  t175 = t314;
  const t316 = "div";
  const t317 = "bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm";
  const t318 = "\n          Some time ranges overlap. Please fix before saving.\n        ";
  const t319 = _jsx(t316, { className: t317, children: t318 });
  t175 = t319;
  const t326 = "div";
  const t327 = "space-y-3";
  const t328 = DAYS;
  const t329 = (day) => {
    let daySchedule;
    const t4 = schedule;
    const t6 = day;
    const t7 = t4[t6];
    daySchedule = t7;
    const t9 = "div";
    const t11 = day;
    const t13 = daySchedule;
    const t14 = t13.enabled;
    let t15;
    if (t14) {
      const t17 = "bg-white";
      t15 = t17;
    } else {
      const t19 = "bg-gray-50";
      t15 = t19;
    }
    const t21 = `p-3 rounded border ${t15}`;
    const t22 = "div";
    const t23 = "flex items-center justify-between";
    const t24 = "label";
    const t25 = "flex items-center gap-2";
    const t26 = "input";
    const t27 = "checkbox";
    const t29 = daySchedule;
    const t30 = t29.enabled;
    const t31 = () => {
      const t1 = handleChange;
      const t2 = "TOGGLE_DAY";
      const t4 = day;
      const t5 = { type: t2, day };
      const t6 = t1(t5);
      return t6;
    };
    const t32 = _jsx(t26, { type: t27, checked: t30, onChange: t31 });
    const t33 = "span";
    const t35 = daySchedule;
    const t36 = t35.enabled;
    let t37;
    if (t36) {
      const t39 = "font-medium";
      t37 = t39;
    } else {
      const t41 = "text-gray-400";
      t37 = t41;
    }
    const t44 = day;
    const t45 = _jsx(t33, { className: t37, children: t44 });
    const t46 = _jsxs(t24, { className: t25, children: [t32, t45] });
    let t47;
    const t50 = daySchedule;
    const t51 = t50.enabled;
    t47 = t51;
    const t53 = "button";
    const t54 = () => {
      const t1 = handleCopyToAll;
      const t3 = day;
      const t4 = t1(t3);
      return t4;
    };
    const t55 = "text-xs text-blue-500 hover:text-blue-700";
    const t57 = copySource;
    const t59 = day;
    const t60 = t57 === t59;
    let t61;
    if (t60) {
      const t63 = "Copied!";
      t61 = t63;
    } else {
      const t65 = "Copy to all";
      t61 = t65;
    }
    const t67 = _jsx(t53, { onClick: t54, className: t55, children: t61 });
    t47 = t67;
    const t69 = _jsxs(t22, { className: t23, children: [t46, t47] });
    let t70;
    const t73 = daySchedule;
    const t74 = t73.enabled;
    t70 = t74;
    const t76 = "div";
    const t77 = "mt-2 space-y-1";
    const t79 = daySchedule;
    const t80 = t79.ranges;
    const t81 = (range, i) => {
      const t2 = "div";
      const t4 = i;
      const t5 = "flex items-center gap-2";
      const t6 = "input";
      const t7 = "time";
      const t9 = range;
      const t10 = t9.start;
      const t11 = (e) => {
        const t2 = handleChange;
        const t3 = "UPDATE_RANGE";
        const t5 = day;
        const t7 = i;
        const t8 = "start";
        const t10 = e;
        const t11 = t10.target;
        const t12 = t11.value;
        const t13 = { type: t3, day, index: t7, field: t8, value: t12 };
        const t14 = t2(t13);
        return t14;
      };
      const t12 = "border rounded px-2 py-1 text-sm";
      const t13 = _jsx(t6, { type: t7, value: t10, onChange: t11, className: t12 });
      const t14 = "span";
      const t15 = "text-gray-400";
      const t16 = "—";
      const t17 = _jsx(t14, { className: t15, children: t16 });
      const t18 = "input";
      const t19 = "time";
      const t21 = range;
      const t22 = t21.end;
      const t23 = (e) => {
        const t2 = handleChange;
        const t3 = "UPDATE_RANGE";
        const t5 = day;
        const t7 = i;
        const t8 = "end";
        const t10 = e;
        const t11 = t10.target;
        const t12 = t11.value;
        const t13 = { type: t3, day, index: t7, field: t8, value: t12 };
        const t14 = t2(t13);
        return t14;
      };
      const t24 = "border rounded px-2 py-1 text-sm";
      const t25 = _jsx(t18, { type: t19, value: t22, onChange: t23, className: t24 });
      let t26;
      const t29 = daySchedule;
      const t30 = t29.ranges;
      const t31 = t30.length;
      const t32 = 1;
      const t33 = t31 > t32;
      t26 = t33;
      const t35 = "button";
      const t36 = () => {
        const t1 = handleChange;
        const t2 = "REMOVE_RANGE";
        const t4 = day;
        const t6 = i;
        const t7 = { type: t2, day, index: t6 };
        const t8 = t1(t7);
        return t8;
      };
      const t37 = "text-red-400 hover:text-red-600 text-sm";
      const t38 = "\n                          ×\n                        ";
      const t39 = _jsx(t35, { onClick: t36, className: t37, children: t38 });
      t26 = t39;
      const t41 = _jsxs(t2, { key: t4, className: t5, children: [t13, t17, t25, t26] });
      return t41;
    };
    const t82 = t80.map(t81);
    const t83 = "button";
    const t84 = () => {
      const t1 = handleChange;
      const t2 = "ADD_RANGE";
      const t4 = day;
      const t5 = { type: t2, day };
      const t6 = t1(t5);
      return t6;
    };
    const t85 = "text-xs text-blue-500 hover:text-blue-700";
    const t86 = "\n                    + Add time range\n                  ";
    const t87 = _jsx(t83, { onClick: t84, className: t85, children: t86 });
    const t88 = _jsxs(t76, { className: t77, children: [t82, t87] });
    t70 = t88;
    const t90 = _jsxs(t9, { key: t11, className: t21, children: [t69, t70] });
    return t90;
  };
  const t330 = t328.map(t329);
  const t331 = _jsx(t326, { className: t327, children: t330 });
  t332 = _jsxs(t264, { className: t265, children: [t312, t175, t331] });
  return t332;
}

