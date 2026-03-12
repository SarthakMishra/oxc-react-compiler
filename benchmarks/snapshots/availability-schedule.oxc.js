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
  const t172 = useReducer;
  const t173 = scheduleReducer;
}

