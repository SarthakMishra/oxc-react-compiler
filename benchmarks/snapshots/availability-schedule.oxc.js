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
  const $ = _c(51);
  const { initialSchedule, onSave, timezone } = t0;
  let t9;
  if ($[0] !== initialSchedule) {
    $[0] = initialSchedule;
    $[1] = t9;
  } else {
    t9 = $[1];
  }
  t9 = initialSchedule;
  t9 = createInitialSchedule();
  let t137;
  let t106;
  let t135;
  let t136;
  let t35;
  let t37;
  if ($[2] !== t38 || $[3] !== onSave) {
    $[2] = t38;
    $[3] = onSave;
    $[4] = t106;
    $[5] = t135;
    $[6] = t136;
    $[7] = t35;
    $[8] = t37;
    $[9] = t137;
  } else {
    t106 = $[4];
    t135 = $[5];
    t136 = $[6];
    t35 = $[7];
    t37 = $[8];
    t137 = $[9];
  }
  const totalHours = t136;
  const schedule = t137;
  let dispatch;
  let t20;
  if ($[10] === Symbol.for("react.memo_cache_sentinel")) {
    $[10] = t20;
  } else {
    t20 = $[10];
  }
  let t26;
  if ($[11] === Symbol.for("react.memo_cache_sentinel")) {
    $[11] = t26;
  } else {
    t26 = $[11];
  }
  let totalHours;
  t35 = () => {
    let total;
    total = 0;
    const t4 = DAYS[Symbol.iterator]();
    const t5 = t4.next();
    let day;
    day = t5;
    let daySchedule;
    daySchedule = schedule[day];
    if (!daySchedule.enabled) {
    }
    const t17 = daySchedule.ranges[Symbol.iterator]();
    const t18 = t17.next();
    let range;
    range = t18;
    let startH;
    let startM;
    let endH;
    let endM;
    total = total + endH * 60 + endM - startH * 60 - startM / 60;
  };
  const t38 = useMemo(t35, [schedule]);
  let t139;
  let t138;
  let t43;
  let t45;
  if ($[12] !== t38) {
    t138 = t38;
    t43 = () => {
      const t2 = DAYS[Symbol.iterator]();
      const t3 = t2.next();
      let day;
      day = t3;
      let daySchedule;
      daySchedule = schedule[day];
      if (!daySchedule.enabled) {
      }
      let sorted;
      const t17 = (a, b) => {
        return a.start.localeCompare(b.start);
      };
      sorted = [...daySchedule.ranges].sort(t17);
      let i;
      i = 1;
      if (i < sorted.length) {
        if (sorted[i].start < sorted[i - 1].end) {
          return true;
        }
      }
    };
    t45 = [schedule];
    $[12] = t38;
    $[13] = t138;
    $[14] = t139;
    $[15] = t43;
    $[16] = t45;
  } else {
    t138 = $[13];
    t139 = $[14];
    t43 = $[15];
    t45 = $[16];
  }
  totalHours = t138;
  const hasOverlaps = t139;
  const t46 = useMemo(t43, t45);
  let t141;
  let t140;
  let t52;
  let t53;
  if ($[17] !== t46) {
    t140 = t46;
    t52 = (action) => {
      const t4 = dispatch(action);
      const t8 = setIsDirty(true);
      return undefined;
    };
    t53 = [];
    $[17] = t46;
    $[18] = t140;
    $[19] = t141;
    $[20] = t52;
    $[21] = t53;
  } else {
    t140 = $[18];
    t141 = $[19];
    t52 = $[20];
    t53 = $[21];
  }
  const hasOverlaps = t140;
  const handleChange = t141;
  const t54 = useCallback(t52, t53);
  let t143;
  let t142;
  let t59;
  let t62;
  if ($[22] !== t54 || $[23] !== onSave) {
    t142 = t54;
    t59 = () => {
      const t4 = onSave(schedule);
      const t8 = setIsDirty(false);
      return undefined;
    };
    t62 = [schedule, onSave];
    $[22] = t54;
    $[23] = onSave;
    $[24] = t142;
    $[25] = t143;
    $[26] = t59;
    $[27] = t62;
  } else {
    t142 = $[24];
    t143 = $[25];
    t59 = $[26];
    t62 = $[27];
  }
  const handleChange = t142;
  const handleSave = t143;
  const t63 = useCallback(t59, t62);
  let t145;
  let t144;
  let t68;
  let t70;
  if ($[28] !== t63) {
    t144 = t63;
    t68 = (day) => {
      const t6 = handleChange({ type: "COPY_TO_ALL", sourceDay: day });
      const t10 = setCopySource(day);
      const t12 = () => {
        return setCopySource(null);
      };
      const t14 = setTimeout(t12, 2000);
      return undefined;
    };
    t70 = [handleChange];
    $[28] = t63;
    $[29] = t144;
    $[30] = t145;
    $[31] = t68;
    $[32] = t70;
  } else {
    t144 = $[29];
    t145 = $[30];
    t68 = $[31];
    t70 = $[32];
  }
  const handleSave = t144;
  const handleCopyToAll = t145;
  const t71 = useCallback(t68, t70);
  let t147;
  let t146;
  let t76;
  let t78;
  if ($[33] !== t71) {
    t146 = t71;
    t76 = () => {
      const t4 = handleChange({ type: "RESET" });
      return undefined;
    };
    t78 = [handleChange];
    $[33] = t71;
    $[34] = t146;
    $[35] = t147;
    $[36] = t76;
    $[37] = t78;
  } else {
    t146 = $[34];
    t147 = $[35];
    t76 = $[36];
    t78 = $[37];
  }
  const handleCopyToAll = t146;
  const handleReset = t147;
  const t79 = useCallback(t76, t78);
  let handleReset;
  let t81;
  let t82;
  let t83;
  let t84;
  let t96;
  let t97;
  let t98;
  let t103;
  let t104;
  if ($[38] !== t79 || $[39] !== timezone) {
    handleReset = t79;
    t81 = "div";
    t82 = "space-y-4";
    t83 = "div";
    t84 = "flex justify-between items-center";
    t96 = (
      <div>
        <h2 className="text-lg font-semibold">Availability</h2>
        <p className="text-sm text-gray-500">{totalHours} hours/week · Timezone: {timezone}</p>
      </div>
    );
    t97 = "div";
    t98 = "flex gap-2";
    t103 = (
      <button onClick={handleReset} className="text-sm text-gray-500 hover:text-gray-700">
        \n            Reset\n          
      </button>
    );
    t104 = "button";
    $[38] = t79;
    $[39] = timezone;
    $[40] = handleReset;
    $[41] = t81;
    $[42] = t82;
    $[43] = t83;
    $[44] = t84;
    $[45] = t96;
    $[46] = t97;
    $[47] = t98;
    $[48] = t103;
    $[49] = t104;
    $[50] = t106;
  } else {
    handleReset = $[40];
    t81 = $[41];
    t82 = $[42];
    t83 = $[43];
    t84 = $[44];
    t96 = $[45];
    t97 = $[46];
    t98 = $[47];
    t103 = $[48];
    t104 = $[49];
    t106 = $[50];
  }
  t106 = !isDirty;
  t106 = hasOverlaps;
  let t110;
  t110 = isDirty;
  t110 = !hasOverlaps;
  let t114;
  if (t110) {
    t114 = "bg-blue-600 hover:bg-blue-700";
  } else {
    t114 = "bg-gray-300 cursor-not-allowed";
  }
  let t122;
  t122 = hasOverlaps;
  t122 = <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm">\n          Some time ranges overlap. Please fix before saving.\n        </div>;
  const t132 = (day) => {
    let daySchedule;
    daySchedule = schedule[day];
    let t10;
    if (daySchedule.enabled) {
      t10 = "bg-white";
    } else {
      t10 = "bg-gray-50";
    }
    const t22 = () => {
      return handleChange({ type: "TOGGLE_DAY", day });
    };
    let t27;
    if (daySchedule.enabled) {
      t27 = "font-medium";
    } else {
      t27 = "text-gray-400";
    }
    let t33;
    t33 = daySchedule.enabled;
    const t37 = () => {
      return handleCopyToAll(day);
    };
    let t43;
    if (copySource === day) {
      t43 = "Copied!";
    } else {
      t43 = "Copy to all";
    }
    t33 = <button onClick={t37} className="text-xs text-blue-500 hover:text-blue-700">{t43}</button>;
    let t48;
    t48 = daySchedule.enabled;
    const t55 = (range, i) => {
      const t9 = (e) => {
        return handleChange({ type: "UPDATE_RANGE", day, index: i, field: "start", value: e.target.value });
      };
      const t20 = (e) => {
        return handleChange({ type: "UPDATE_RANGE", day, index: i, field: "end", value: e.target.value });
      };
      let t23;
      t23 = daySchedule.ranges.length > 1;
      const t31 = () => {
        return handleChange({ type: "REMOVE_RANGE", day, index: i });
      };
      t23 = <button onClick={t31} className="text-red-400 hover:text-red-600 text-sm">\n                          ×\n                        </button>;
      return <div key={i} className="flex items-center gap-2"><input type="time" value={range.start} onChange={t9} className="border rounded px-2 py-1 text-sm" /><span className="text-gray-400">—</span><input type="time" value={range.end} onChange={t20} className="border rounded px-2 py-1 text-sm" />{t23}</div>;
    };
    const t58 = () => {
      return handleChange({ type: "ADD_RANGE", day });
    };
    t48 = <div className="mt-2 space-y-1">{daySchedule.ranges.map(t55)}<button onClick={t58} className="text-xs text-blue-500 hover:text-blue-700">\n                    + Add time range\n                  </button></div>;
    return <div key={day} className={`p-3 rounded border ${t10}`}><div className="flex items-center justify-between"><label className="flex items-center gap-2"><input type="checkbox" checked={daySchedule.enabled} onChange={t22} /><span className={t27}>{day}</span></label>{t33}</div>{t48}</div>;
  };
  return <t81 className={t82}><t83 className={t84}>{t96}<t97 className={t98}>{t103}<t104 onClick={handleSave} disabled={t106} className={`px-4 py-2 rounded text-white text-sm ${t114}`}>\n            Save\n          </t104></t97></t83>{t122}<div className="space-y-3">{DAYS.map(t132)}</div></t81>;
}

