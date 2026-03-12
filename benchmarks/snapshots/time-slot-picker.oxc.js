import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// S tier - Inspired by cal.com time slot picker
import { useState, useMemo, useCallback } from 'react';

interface TimeSlot {
  time: string;
  available: boolean;
}

interface TimeSlotPickerProps {
  slots: TimeSlot[];
  selectedDate: string;
  onSelect: (time: string) => void;
  timezone?: string;
}

export function TimeSlotPicker(t0) {
  const $ = _c(19);
  let slots;
  let selectedDate;
  let onSelect;
  let timezone;
  if ($[0] !== slots || $[1] !== selectedDate || $[2] !== onSelect || $[3] !== timezone) {
    $[0] = slots;
    $[1] = selectedDate;
    $[2] = onSelect;
    $[3] = timezone;
  } else {
  }
  ({ slots, selectedDate, onSelect, timezone } = t0);
  const t123 = useState;
  const t124 = null;
  const t125 = t123(t124);
  let selectedSlot;
  let setSelectedSlot;
  if ($[4] !== selectedSlot || $[5] !== setSelectedSlot) {
    $[4] = selectedSlot;
    $[5] = setSelectedSlot;
  } else {
  }
  ([selectedSlot, setSelectedSlot] = t125);
  let availableSlots;
  if ($[6] !== availableSlots) {
    $[6] = availableSlots;
  } else {
  }
  let morningSlots;
  if ($[7] !== useMemo || $[8] !== slots || $[9] !== availableSlots || $[10] !== morningSlots) {
    const t130 = useMemo;
    const t131 = () => {
      const t1 = slots;
      const t2 = (s) => {
        const t2 = s;
        const t3 = t2.available;
        return t3;
      };
      const t3 = t1.filter(t2);
      return t3;
    };
    const t132 = slots;
    const t133 = [t132];
    const t134 = t130(t131, t133);
    availableSlots = t134;
    $[7] = useMemo;
    $[8] = slots;
    $[9] = availableSlots;
    $[10] = morningSlots;
  } else {
  }
  let afternoonSlots;
  if ($[11] !== useMemo || $[12] !== availableSlots || $[13] !== morningSlots || $[14] !== afternoonSlots) {
    const t137 = useMemo;
    const t138 = () => {
      const t1 = availableSlots;
      const t2 = (s) => {
        const t1 = parseInt;
        const t3 = s;
        const t4 = t3.time;
        const t5 = t1(t4);
        const t6 = 12;
        const t7 = t5 < t6;
        return t7;
      };
      const t3 = t1.filter(t2);
      return t3;
    };
    const t139 = availableSlots;
    const t140 = [t139];
    const t141 = t137(t138, t140);
    morningSlots = t141;
    $[11] = useMemo;
    $[12] = availableSlots;
    $[13] = morningSlots;
    $[14] = afternoonSlots;
  } else {
  }
  let handleSelect;
  if ($[15] !== useMemo || $[16] !== availableSlots || $[17] !== afternoonSlots || $[18] !== handleSelect) {
    const t144 = useMemo;
    const t145 = () => {
      const t1 = availableSlots;
      const t2 = (s) => {
        const t1 = parseInt;
        const t3 = s;
        const t4 = t3.time;
        const t5 = t1(t4);
        const t6 = 12;
        const t7 = t5 >= t6;
        return t7;
      };
      const t3 = t1.filter(t2);
      return t3;
    };
    const t146 = availableSlots;
    const t147 = [t146];
    const t148 = t144(t145, t147);
    afternoonSlots = t148;
    $[15] = useMemo;
    $[16] = availableSlots;
    $[17] = afternoonSlots;
    $[18] = handleSelect;
  } else {
  }
  const t151 = useCallback;
  const t152 = (time) => {
    const t2 = setSelectedSlot;
    const t4 = time;
    const t5 = t2(t4);
    const t7 = onSelect;
    const t9 = time;
    const t10 = t7(t9);
    const t11 = undefined;
    return t11;
  };
  const t153 = onSelect;
  const t154 = [t153];
  const t155 = t151(t152, t154);
  handleSelect = t155;
  const t157 = "div";
  const t158 = "h3";
  const t159 = selectedDate;
  const t160 = " (";
  const t161 = timezone;
  const t162 = ")";
  const t163 = _jsxs(t158, { children: [t159, t160, t161, t162] });
  const t164 = availableSlots;
  const t165 = t164.length;
  const t166 = 0;
  const t167 = t165 === t166;
}

