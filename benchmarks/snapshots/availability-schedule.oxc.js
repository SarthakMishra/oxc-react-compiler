import { c as _c } from "react/compiler-runtime";
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
  const $ = _c(21);
  const { initialSchedule, onSave, timezone } = t0;
  let t12;
  if ($[0] !== initialSchedule) {
    $[0] = initialSchedule;
  }
  t12 = initialSchedule;
  t12 = createInitialSchedule();
  if ($[1] !== schedule || $[2] !== useMemo) {
    const t224 = () => {
      let total = 0;
      const t6 = DAYS[Symbol.iterator]();
      const t7 = t6.next();
      const day = t7;
      const daySchedule = schedule[day];
      if (!daySchedule.enabled) {
      }
      const t26 = daySchedule.ranges[Symbol.iterator]();
      const t27 = t26.next();
      const range = t27;
      total = total + endH * 60 + endM - startH * 60 - startM / 60;
    };
    const totalHours = useMemo(t224, [schedule]);
    $[1] = schedule;
    $[2] = useMemo;
  }
  if ($[3] !== schedule || $[4] !== useMemo) {
    const t231 = () => {
      const t2 = DAYS[Symbol.iterator]();
      const t3 = t2.next();
      const day = t3;
      const daySchedule = schedule[day];
      if (!daySchedule.enabled) {
      }
      const t25 = (a, b) => {
        return a.start.localeCompare(b.start);
      };
      const sorted = [...daySchedule.ranges].sort(t25);
      let i = 1;
      if (i < sorted.length) {
        if (sorted[i].start < sorted[i - 1].end) {
          return true;
        }
      }
    };
    const hasOverlaps = useMemo(t231, [schedule]);
    $[3] = schedule;
    $[4] = useMemo;
  }
  if ($[5] !== useCallback) {
    const t238 = (action) => {
      const t5 = dispatch(action);
      const t9 = setIsDirty(true);
      return undefined;
    };
    const handleChange = useCallback(t238, []);
    $[5] = useCallback;
  }
  if ($[6] !== onSave || $[7] !== schedule || $[8] !== useCallback) {
    const t244 = () => {
      const t4 = onSave(schedule);
      const t8 = setIsDirty(false);
      return undefined;
    };
    const handleSave = useCallback(t244, [schedule, onSave]);
    $[6] = onSave;
    $[7] = schedule;
    $[8] = useCallback;
  }
  if ($[9] !== handleChange || $[10] !== useCallback) {
    const t252 = (day) => {
      const t7 = handleChange({ type: "COPY_TO_ALL", sourceDay: day });
      const t12 = setCopySource(day);
      const t14 = () => {
        return setCopySource(null);
      };
      const t16 = setTimeout(t14, 2000);
      return undefined;
    };
    const handleCopyToAll = useCallback(t252, [handleChange]);
    $[9] = handleChange;
    $[10] = useCallback;
  }
  if ($[11] !== handleChange || $[12] !== useCallback) {
    const t259 = () => {
      const t4 = handleChange({ type: "RESET" });
      return undefined;
    };
    const handleReset = useCallback(t259, [handleChange]);
    $[11] = handleChange;
    $[12] = useCallback;
  }
  t146 = !isDirty;
  t146 = hasOverlaps;
  t155 = isDirty;
  t155 = !hasOverlaps;
  if (t155) {
    t164 = "bg-blue-600 hover:bg-blue-700";
  } else {
    t164 = "bg-gray-300 cursor-not-allowed";
  }
  let t332;
  if ($[13] !== t164 || $[14] !== t146 || $[15] !== DAYS || $[16] !== handleReset || $[17] !== handleSave || $[18] !== timezone || $[19] !== totalHours) {
    $[13] = t164;
    $[14] = t146;
    $[15] = DAYS;
    $[16] = handleReset;
    $[17] = handleSave;
    $[18] = timezone;
    $[19] = totalHours;
    $[20] = t332;
  } else {
    t332 = $[20];
  }
  t175 = hasOverlaps;
  t175 = <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm">\n          Some time ranges overlap. Please fix before saving.\n        </div>;
  const t329 = (day) => {
    const daySchedule = schedule[day];
    if (daySchedule.enabled) {
      t15 = "bg-white";
    } else {
      t15 = "bg-gray-50";
    }
    const t31 = () => {
      return handleChange({ type: "TOGGLE_DAY", day });
    };
    if (daySchedule.enabled) {
      t37 = "font-medium";
    } else {
      t37 = "text-gray-400";
    }
    t47 = daySchedule.enabled;
    const t54 = () => {
      return handleCopyToAll(day);
    };
    if (copySource === day) {
      t61 = "Copied!";
    } else {
      t61 = "Copy to all";
    }
    t47 = <button onClick={t54} className="text-xs text-blue-500 hover:text-blue-700">{t61}</button>;
    t70 = daySchedule.enabled;
    const t81 = (range, i) => {
      const t11 = (e) => {
        return handleChange({ type: "UPDATE_RANGE", day, index: i, field: "start", value: e.target.value });
      };
      const t23 = (e) => {
        return handleChange({ type: "UPDATE_RANGE", day, index: i, field: "end", value: e.target.value });
      };
      t26 = daySchedule.ranges.length > 1;
      const t36 = () => {
        return handleChange({ type: "REMOVE_RANGE", day, index: i });
      };
      t26 = <button onClick={t36} className="text-red-400 hover:text-red-600 text-sm">\n                          ×\n                        </button>;
      return <div key={i} className="flex items-center gap-2"><input type="time" value={range.start} onChange={t11} className="border rounded px-2 py-1 text-sm" /><span className="text-gray-400">—</span><input type="time" value={range.end} onChange={t23} className="border rounded px-2 py-1 text-sm" />{t26}</div>;
    };
    const t84 = () => {
      return handleChange({ type: "ADD_RANGE", day });
    };
    t70 = <div className="mt-2 space-y-1">{daySchedule.ranges.map(t81)}<button onClick={t84} className="text-xs text-blue-500 hover:text-blue-700">\n                    + Add time range\n                  </button></div>;
    return <div key={day} className={`p-3 rounded border ${t15}`}><div className="flex items-center justify-between"><label className="flex items-center gap-2"><input type="checkbox" checked={daySchedule.enabled} onChange={t31} /><span className={t37}>{day}</span></label>{t47}</div>{t70}</div>;
  };
  return <div className="space-y-4">{t312}{t175}<div className="space-y-3">{DAYS.map(t329)}</div></div>;
}

