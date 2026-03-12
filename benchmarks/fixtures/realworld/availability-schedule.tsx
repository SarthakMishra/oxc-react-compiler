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

export function AvailabilitySchedule({ initialSchedule, onSave, timezone = 'UTC' }: AvailabilityScheduleProps) {
  const [schedule, dispatch] = useReducer(scheduleReducer, initialSchedule || createInitialSchedule());
  const [isDirty, setIsDirty] = useState(false);
  const [copySource, setCopySource] = useState<string | null>(null);

  const totalHours = useMemo(() => {
    let total = 0;
    for (const day of DAYS) {
      const daySchedule = schedule[day];
      if (!daySchedule.enabled) continue;
      for (const range of daySchedule.ranges) {
        const [startH, startM] = range.start.split(':').map(Number);
        const [endH, endM] = range.end.split(':').map(Number);
        total += (endH * 60 + endM - startH * 60 - startM) / 60;
      }
    }
    return Math.round(total * 10) / 10;
  }, [schedule]);

  const hasOverlaps = useMemo(() => {
    for (const day of DAYS) {
      const daySchedule = schedule[day];
      if (!daySchedule.enabled) continue;
      const sorted = [...daySchedule.ranges].sort((a, b) => a.start.localeCompare(b.start));
      for (let i = 1; i < sorted.length; i++) {
        if (sorted[i].start < sorted[i - 1].end) return true;
      }
    }
    return false;
  }, [schedule]);

  const handleChange = useCallback(
    (action: Action) => {
      dispatch(action);
      setIsDirty(true);
    },
    []
  );

  const handleSave = useCallback(() => {
    onSave(schedule);
    setIsDirty(false);
  }, [schedule, onSave]);

  const handleCopyToAll = useCallback(
    (day: string) => {
      handleChange({ type: 'COPY_TO_ALL', sourceDay: day });
      setCopySource(day);
      setTimeout(() => setCopySource(null), 2000);
    },
    [handleChange]
  );

  const handleReset = useCallback(() => {
    handleChange({ type: 'RESET' });
  }, [handleChange]);

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-lg font-semibold">Availability</h2>
          <p className="text-sm text-gray-500">
            {totalHours} hours/week · Timezone: {timezone}
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={handleReset} className="text-sm text-gray-500 hover:text-gray-700">
            Reset
          </button>
          <button
            onClick={handleSave}
            disabled={!isDirty || hasOverlaps}
            className={`px-4 py-2 rounded text-white text-sm ${
              isDirty && !hasOverlaps ? 'bg-blue-600 hover:bg-blue-700' : 'bg-gray-300 cursor-not-allowed'
            }`}
          >
            Save
          </button>
        </div>
      </div>

      {hasOverlaps && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm">
          Some time ranges overlap. Please fix before saving.
        </div>
      )}

      <div className="space-y-3">
        {DAYS.map((day) => {
          const daySchedule = schedule[day];
          return (
            <div key={day} className={`p-3 rounded border ${daySchedule.enabled ? 'bg-white' : 'bg-gray-50'}`}>
              <div className="flex items-center justify-between">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={daySchedule.enabled}
                    onChange={() => handleChange({ type: 'TOGGLE_DAY', day })}
                  />
                  <span className={daySchedule.enabled ? 'font-medium' : 'text-gray-400'}>{day}</span>
                </label>
                {daySchedule.enabled && (
                  <button
                    onClick={() => handleCopyToAll(day)}
                    className="text-xs text-blue-500 hover:text-blue-700"
                  >
                    {copySource === day ? 'Copied!' : 'Copy to all'}
                  </button>
                )}
              </div>

              {daySchedule.enabled && (
                <div className="mt-2 space-y-1">
                  {daySchedule.ranges.map((range, i) => (
                    <div key={i} className="flex items-center gap-2">
                      <input
                        type="time"
                        value={range.start}
                        onChange={(e) =>
                          handleChange({ type: 'UPDATE_RANGE', day, index: i, field: 'start', value: e.target.value })
                        }
                        className="border rounded px-2 py-1 text-sm"
                      />
                      <span className="text-gray-400">—</span>
                      <input
                        type="time"
                        value={range.end}
                        onChange={(e) =>
                          handleChange({ type: 'UPDATE_RANGE', day, index: i, field: 'end', value: e.target.value })
                        }
                        className="border rounded px-2 py-1 text-sm"
                      />
                      {daySchedule.ranges.length > 1 && (
                        <button
                          onClick={() => handleChange({ type: 'REMOVE_RANGE', day, index: i })}
                          className="text-red-400 hover:text-red-600 text-sm"
                        >
                          ×
                        </button>
                      )}
                    </div>
                  ))}
                  <button
                    onClick={() => handleChange({ type: 'ADD_RANGE', day })}
                    className="text-xs text-blue-500 hover:text-blue-700"
                  >
                    + Add time range
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
