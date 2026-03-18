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
  const $ = _c(43);
  let t9;
  let t136;
  let t20;
  let t26;
  let t137;
  let t138;
  let t43;
  let t45;
  let t139;
  let t140;
  let t52;
  let t53;
  let t141;
  let t142;
  let t59;
  let t62;
  let t143;
  let t144;
  let t68;
  let t70;
  let t145;
  let t146;
  let t76;
  let t78;
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
  let t106;
  let { initialSchedule, onSave, timezone } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t9;
  } else {
    t9 = $[0];
  }
  t9 = initialSchedule;
  t9 = createInitialSchedule();
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    $[1] = t136;
  } else {
    t136 = $[1];
  }
  let schedule = t136;
  let dispatch;
  ([schedule, dispatch] = useReducer(scheduleReducer, t9));
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    t20 = false;
    $[2] = t20;
  } else {
    t20 = $[2];
  }
  let isDirty;
  let setIsDirty;
  ([isDirty, setIsDirty] = useState(t20));
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    t26 = null;
    $[3] = t26;
  } else {
    t26 = $[3];
  }
  let copySource;
  let setCopySource;
  ([copySource, setCopySource] = useState(t26));
  let totalHours;
  let t35 = () => {
    let total;
    total = 0;
    let t4 = DAYS[Symbol.iterator]();
    let t5 = t4.next();
    let day;
    day = t5;
    let daySchedule;
    daySchedule = schedule[day];
    if (!daySchedule.enabled) {
    }
    let t17 = daySchedule.ranges[Symbol.iterator]();
    let t18 = t17.next();
    let range;
    range = t18;
    let startH;
    let startM;
    ([startH, startM] = range.start.split(":").map(Number));
    let endH;
    let endM;
    ([endH, endM] = range.end.split(":").map(Number));
    total = total + endH * 60 + endM - startH * 60 - startM / 60;
  };
  let t38 = useMemo(t35, [schedule]);
  if ($[4] !== t38) {
    t137 = t38;
    t43 = () => {
      let t2 = DAYS[Symbol.iterator]();
      let t3 = t2.next();
      let day;
      day = t3;
      let daySchedule;
      daySchedule = schedule[day];
      if (!daySchedule.enabled) {
      }
      let sorted;
      let t17 = (a, b) => {
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
    $[4] = t38;
    $[5] = t137;
    $[6] = t138;
    $[7] = t43;
    $[8] = t45;
  } else {
    t137 = $[5];
    t138 = $[6];
    t43 = $[7];
    t45 = $[8];
  }
  totalHours = t137;
  let hasOverlaps = t138;
  let t46 = useMemo(t43, t45);
  if ($[9] !== t46) {
    t139 = t46;
    t52 = (action) => {
      let t4 = dispatch(action);
      let t8 = setIsDirty(true);
      return undefined;
    };
    t53 = [];
    $[9] = t46;
    $[10] = t139;
    $[11] = t140;
    $[12] = t52;
    $[13] = t53;
  } else {
    t139 = $[10];
    t140 = $[11];
    t52 = $[12];
    t53 = $[13];
  }
  hasOverlaps = t139;
  let handleChange = t140;
  let t54 = useCallback(t52, t53);
  if ($[14] !== t54 || $[15] !== onSave) {
    t141 = t54;
    t59 = () => {
      let t4 = onSave(schedule);
      let t8 = setIsDirty(false);
      return undefined;
    };
    t62 = [schedule, onSave];
    $[14] = t54;
    $[15] = onSave;
    $[16] = t141;
    $[17] = t142;
    $[18] = t59;
    $[19] = t62;
  } else {
    t141 = $[16];
    t142 = $[17];
    t59 = $[18];
    t62 = $[19];
  }
  handleChange = t141;
  let handleSave = t142;
  let t63 = useCallback(t59, t62);
  if ($[20] !== t63) {
    t143 = t63;
    t68 = (day) => {
      let t6 = handleChange({ type: "COPY_TO_ALL", sourceDay: day });
      let t10 = setCopySource(day);
      let t12 = () => {
        return setCopySource(null);
      };
      let t14 = setTimeout(t12, 2000);
      return undefined;
    };
    t70 = [handleChange];
    $[20] = t63;
    $[21] = t143;
    $[22] = t144;
    $[23] = t68;
    $[24] = t70;
  } else {
    t143 = $[21];
    t144 = $[22];
    t68 = $[23];
    t70 = $[24];
  }
  handleSave = t143;
  let handleCopyToAll = t144;
  let t71 = useCallback(t68, t70);
  if ($[25] !== t71) {
    t145 = t71;
    t76 = () => {
      let t4 = handleChange({ type: "RESET" });
      return undefined;
    };
    t78 = [handleChange];
    $[25] = t71;
    $[26] = t145;
    $[27] = t146;
    $[28] = t76;
    $[29] = t78;
  } else {
    t145 = $[26];
    t146 = $[27];
    t76 = $[28];
    t78 = $[29];
  }
  handleCopyToAll = t145;
  handleReset = t146;
  let t79 = useCallback(t76, t78);
  if ($[30] !== t79 || $[31] !== timezone) {
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
    $[30] = t79;
    $[31] = timezone;
    $[32] = handleReset;
    $[33] = t81;
    $[34] = t82;
    $[35] = t83;
    $[36] = t84;
    $[37] = t96;
    $[38] = t97;
    $[39] = t98;
    $[40] = t103;
    $[41] = t104;
    $[42] = t106;
  } else {
    handleReset = $[32];
    t81 = $[33];
    t82 = $[34];
    t83 = $[35];
    t84 = $[36];
    t96 = $[37];
    t97 = $[38];
    t98 = $[39];
    t103 = $[40];
    t104 = $[41];
    t106 = $[42];
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
  let t132 = (day) => {
    let daySchedule;
    daySchedule = schedule[day];
    let t10;
    if (daySchedule.enabled) {
      t10 = "bg-white";
    } else {
      t10 = "bg-gray-50";
    }
    let t22 = () => {
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
    let t37 = () => {
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
    let t55 = (range, i) => {
      let t9 = (e) => {
        return handleChange({ type: "UPDATE_RANGE", day, index: i, field: "start", value: e.target.value });
      };
      let t20 = (e) => {
        return handleChange({ type: "UPDATE_RANGE", day, index: i, field: "end", value: e.target.value });
      };
      let t23;
      t23 = daySchedule.ranges.length > 1;
      let t31 = () => {
        return handleChange({ type: "REMOVE_RANGE", day, index: i });
      };
      t23 = <button onClick={t31} className="text-red-400 hover:text-red-600 text-sm">\n                          ×\n                        </button>;
      return <div key={i} className="flex items-center gap-2"><input type="time" value={range.start} onChange={t9} className="border rounded px-2 py-1 text-sm" /><span className="text-gray-400">—</span><input type="time" value={range.end} onChange={t20} className="border rounded px-2 py-1 text-sm" />{t23}</div>;
    };
    let t58 = () => {
      return handleChange({ type: "ADD_RANGE", day });
    };
    t48 = <div className="mt-2 space-y-1">{daySchedule.ranges.map(t55)}<button onClick={t58} className="text-xs text-blue-500 hover:text-blue-700">\n                    + Add time range\n                  </button></div>;
    return <div key={day} className={`p-3 rounded border ${t10}`}><div className="flex items-center justify-between"><label className="flex items-center gap-2"><input type="checkbox" checked={daySchedule.enabled} onChange={t22} /><span className={t27}>{day}</span></label>{t33}</div>{t48}</div>;
  };
  return <div className={t82}><div className={t84}>{t96}<div className={t98}>{t103}<button onClick={handleSave} disabled={t106} className={`px-4 py-2 rounded text-white text-sm ${t114}`}>\n            Save\n          </button></div></div>{t122}<div className="space-y-3">{DAYS.map(t132)}</div></div>;
}

