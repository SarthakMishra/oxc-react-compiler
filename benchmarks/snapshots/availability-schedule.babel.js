import { c as _c } from "react/compiler-runtime";
// L tier - Inspired by cal.com availability schedule editor
import { useState, useMemo, useCallback, useReducer } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
const DAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
const DEFAULT_RANGE = {
  start: '09:00',
  end: '17:00'
};
function scheduleReducer(state, action) {
  switch (action.type) {
    case 'TOGGLE_DAY':
      return {
        ...state,
        [action.day]: {
          ...state[action.day],
          enabled: !state[action.day].enabled
        }
      };
    case 'ADD_RANGE':
      return {
        ...state,
        [action.day]: {
          ...state[action.day],
          ranges: [...state[action.day].ranges, {
            ...DEFAULT_RANGE
          }]
        }
      };
    case 'REMOVE_RANGE':
      return {
        ...state,
        [action.day]: {
          ...state[action.day],
          ranges: state[action.day].ranges.filter((_, i) => i !== action.index)
        }
      };
    case 'UPDATE_RANGE':
      {
        const ranges = [...state[action.day].ranges];
        ranges[action.index] = {
          ...ranges[action.index],
          [action.field]: action.value
        };
        return {
          ...state,
          [action.day]: {
            ...state[action.day],
            ranges
          }
        };
      }
    case 'COPY_TO_ALL':
      {
        const source = state[action.sourceDay];
        const next = {
          ...state
        };
        for (const day of DAYS) {
          if (day !== action.sourceDay) {
            next[day] = {
              enabled: source.enabled,
              ranges: source.ranges.map(r => ({
                ...r
              }))
            };
          }
        }
        return next;
      }
    case 'RESET':
      {
        const initial = {};
        for (const day of DAYS) {
          initial[day] = {
            enabled: day !== 'Saturday' && day !== 'Sunday',
            ranges: [{
              ...DEFAULT_RANGE
            }]
          };
        }
        return initial;
      }
    default:
      return state;
  }
}
function createInitialSchedule() {
  const schedule = {};
  for (const day of DAYS) {
    schedule[day] = {
      enabled: day !== 'Saturday' && day !== 'Sunday',
      ranges: [{
        ...DEFAULT_RANGE
      }]
    };
  }
  return schedule;
}
export function AvailabilitySchedule(t0) {
  const $ = _c(31);
  const {
    initialSchedule,
    onSave,
    timezone: t1
  } = t0;
  const timezone = t1 === undefined ? "UTC" : t1;
  let t2;
  if ($[0] !== initialSchedule) {
    t2 = initialSchedule || createInitialSchedule();
    $[0] = initialSchedule;
    $[1] = t2;
  } else {
    t2 = $[1];
  }
  const [schedule, dispatch] = useReducer(scheduleReducer, t2);
  const [isDirty, setIsDirty] = useState(false);
  const [copySource, setCopySource] = useState(null);
  let total = 0;
  for (const day of DAYS) {
    const daySchedule = schedule[day];
    if (!daySchedule.enabled) {
      continue;
    }
    for (const range of daySchedule.ranges) {
      const [startH, startM] = range.start.split(":").map(Number);
      const [endH, endM] = range.end.split(":").map(Number);
      total = total + (endH * 60 + endM - startH * 60 - startM) / 60;
    }
  }
  const totalHours = Math.round(total * 10) / 10;
  let t3;
  bb0: {
    for (const day_0 of DAYS) {
      const daySchedule_0 = schedule[day_0];
      if (!daySchedule_0.enabled) {
        continue;
      }
      const sorted = [...daySchedule_0.ranges].sort(_temp);
      for (let i = 1; i < sorted.length; i++) {
        if (sorted[i].start < sorted[i - 1].end) {
          t3 = true;
          break bb0;
        }
      }
    }
    t3 = false;
  }
  const hasOverlaps = t3;
  let t4;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    t4 = action => {
      dispatch(action);
      setIsDirty(true);
    };
    $[2] = t4;
  } else {
    t4 = $[2];
  }
  const handleChange = t4;
  let t5;
  if ($[3] !== onSave || $[4] !== schedule) {
    t5 = () => {
      onSave(schedule);
      setIsDirty(false);
    };
    $[3] = onSave;
    $[4] = schedule;
    $[5] = t5;
  } else {
    t5 = $[5];
  }
  const handleSave = t5;
  let t6;
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    t6 = day_1 => {
      handleChange({
        type: "COPY_TO_ALL",
        sourceDay: day_1
      });
      setCopySource(day_1);
      setTimeout(() => setCopySource(null), 2000);
    };
    $[6] = t6;
  } else {
    t6 = $[6];
  }
  const handleCopyToAll = t6;
  let t7;
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    t7 = () => {
      handleChange({
        type: "RESET"
      });
    };
    $[7] = t7;
  } else {
    t7 = $[7];
  }
  const handleReset = t7;
  let t8;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = /*#__PURE__*/_jsx("h2", {
      className: "text-lg font-semibold",
      children: "Availability"
    });
    $[8] = t8;
  } else {
    t8 = $[8];
  }
  let t9;
  if ($[9] !== timezone || $[10] !== totalHours) {
    t9 = /*#__PURE__*/_jsxs("div", {
      children: [t8, /*#__PURE__*/_jsxs("p", {
        className: "text-sm text-gray-500",
        children: [totalHours, " hours/week \xB7 Timezone: ", timezone]
      })]
    });
    $[9] = timezone;
    $[10] = totalHours;
    $[11] = t9;
  } else {
    t9 = $[11];
  }
  let t10;
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    t10 = /*#__PURE__*/_jsx("button", {
      onClick: handleReset,
      className: "text-sm text-gray-500 hover:text-gray-700",
      children: "Reset"
    });
    $[12] = t10;
  } else {
    t10 = $[12];
  }
  const t11 = !isDirty || hasOverlaps;
  const t12 = `px-4 py-2 rounded text-white text-sm ${isDirty && !hasOverlaps ? "bg-blue-600 hover:bg-blue-700" : "bg-gray-300 cursor-not-allowed"}`;
  let t13;
  if ($[13] !== handleSave || $[14] !== t11 || $[15] !== t12) {
    t13 = /*#__PURE__*/_jsxs("div", {
      className: "flex gap-2",
      children: [t10, /*#__PURE__*/_jsx("button", {
        onClick: handleSave,
        disabled: t11,
        className: t12,
        children: "Save"
      })]
    });
    $[13] = handleSave;
    $[14] = t11;
    $[15] = t12;
    $[16] = t13;
  } else {
    t13 = $[16];
  }
  let t14;
  if ($[17] !== t13 || $[18] !== t9) {
    t14 = /*#__PURE__*/_jsxs("div", {
      className: "flex justify-between items-center",
      children: [t9, t13]
    });
    $[17] = t13;
    $[18] = t9;
    $[19] = t14;
  } else {
    t14 = $[19];
  }
  let t15;
  if ($[20] !== hasOverlaps) {
    t15 = hasOverlaps && /*#__PURE__*/_jsx("div", {
      className: "bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm",
      children: "Some time ranges overlap. Please fix before saving."
    });
    $[20] = hasOverlaps;
    $[21] = t15;
  } else {
    t15 = $[21];
  }
  let t16;
  if ($[22] !== copySource || $[23] !== schedule) {
    t16 = DAYS.map(day_2 => {
      const daySchedule_1 = schedule[day_2];
      return /*#__PURE__*/_jsxs("div", {
        className: `p-3 rounded border ${daySchedule_1.enabled ? "bg-white" : "bg-gray-50"}`,
        children: [/*#__PURE__*/_jsxs("div", {
          className: "flex items-center justify-between",
          children: [/*#__PURE__*/_jsxs("label", {
            className: "flex items-center gap-2",
            children: [/*#__PURE__*/_jsx("input", {
              type: "checkbox",
              checked: daySchedule_1.enabled,
              onChange: () => handleChange({
                type: "TOGGLE_DAY",
                day: day_2
              })
            }), /*#__PURE__*/_jsx("span", {
              className: daySchedule_1.enabled ? "font-medium" : "text-gray-400",
              children: day_2
            })]
          }), daySchedule_1.enabled && /*#__PURE__*/_jsx("button", {
            onClick: () => handleCopyToAll(day_2),
            className: "text-xs text-blue-500 hover:text-blue-700",
            children: copySource === day_2 ? "Copied!" : "Copy to all"
          })]
        }), daySchedule_1.enabled && /*#__PURE__*/_jsxs("div", {
          className: "mt-2 space-y-1",
          children: [daySchedule_1.ranges.map((range_0, i_0) => /*#__PURE__*/_jsxs("div", {
            className: "flex items-center gap-2",
            children: [/*#__PURE__*/_jsx("input", {
              type: "time",
              value: range_0.start,
              onChange: e => handleChange({
                type: "UPDATE_RANGE",
                day: day_2,
                index: i_0,
                field: "start",
                value: e.target.value
              }),
              className: "border rounded px-2 py-1 text-sm"
            }), /*#__PURE__*/_jsx("span", {
              className: "text-gray-400",
              children: "\u2014"
            }), /*#__PURE__*/_jsx("input", {
              type: "time",
              value: range_0.end,
              onChange: e_0 => handleChange({
                type: "UPDATE_RANGE",
                day: day_2,
                index: i_0,
                field: "end",
                value: e_0.target.value
              }),
              className: "border rounded px-2 py-1 text-sm"
            }), daySchedule_1.ranges.length > 1 && /*#__PURE__*/_jsx("button", {
              onClick: () => handleChange({
                type: "REMOVE_RANGE",
                day: day_2,
                index: i_0
              }),
              className: "text-red-400 hover:text-red-600 text-sm",
              children: "\xD7"
            })]
          }, i_0)), /*#__PURE__*/_jsx("button", {
            onClick: () => handleChange({
              type: "ADD_RANGE",
              day: day_2
            }),
            className: "text-xs text-blue-500 hover:text-blue-700",
            children: "+ Add time range"
          })]
        })]
      }, day_2);
    });
    $[22] = copySource;
    $[23] = schedule;
    $[24] = t16;
  } else {
    t16 = $[24];
  }
  let t17;
  if ($[25] !== t16) {
    t17 = /*#__PURE__*/_jsx("div", {
      className: "space-y-3",
      children: t16
    });
    $[25] = t16;
    $[26] = t17;
  } else {
    t17 = $[26];
  }
  let t18;
  if ($[27] !== t14 || $[28] !== t15 || $[29] !== t17) {
    t18 = /*#__PURE__*/_jsxs("div", {
      className: "space-y-4",
      children: [t14, t15, t17]
    });
    $[27] = t14;
    $[28] = t15;
    $[29] = t17;
    $[30] = t18;
  } else {
    t18 = $[30];
  }
  return t18;
}
function _temp(a, b) {
  return a.start.localeCompare(b.start);
}