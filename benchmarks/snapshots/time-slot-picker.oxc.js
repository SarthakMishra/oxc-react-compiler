import { c as _c } from "react/compiler-runtime";
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
  const $ = _c(17);
  const { slots, selectedDate, onSelect, timezone } = t0;
  let morningSlots;
  if ($[0] !== slots || $[1] !== useMemo) {
    const t140 = () => {
      const t2 = (s) => {
        return s.available;
      };
      return slots.filter(t2);
    };
    const availableSlots = useMemo(t140, [slots]);
    $[0] = slots;
    $[1] = useMemo;
  }
  let afternoonSlots;
  if ($[2] !== availableSlots || $[3] !== useMemo) {
    const t147 = () => {
      const t2 = (s) => {
        return parseInt(s.time) < 12;
      };
      return availableSlots.filter(t2);
    };
    morningSlots = useMemo(t147, [availableSlots]);
    $[2] = availableSlots;
    $[3] = useMemo;
  }
  let handleSelect;
  if ($[4] !== availableSlots || $[5] !== useMemo) {
    const t154 = () => {
      const t2 = (s) => {
        return parseInt(s.time) >= 12;
      };
      return availableSlots.filter(t2);
    };
    afternoonSlots = useMemo(t154, [availableSlots]);
    $[4] = availableSlots;
    $[5] = useMemo;
  }
  const t161 = (time) => {
    const t5 = setSelectedSlot(time);
    const t10 = onSelect(time);
    return undefined;
  };
  handleSelect = useCallback(t161, [onSelect]);
  if (availableSlots.length === 0) {
    if ($[6] !== availableSlots || $[7] !== onSelect || $[8] !== useCallback) {
      t81 = <p>No available slots</p>;
      $[6] = availableSlots;
      $[7] = onSelect;
      $[8] = useCallback;
    }
  } else {
    if ($[9] !== morningSlots || $[10] !== morningSlots) {
      t87 = morningSlots.length > 0;
      $[9] = morningSlots;
      $[10] = morningSlots;
    }
    const t261 = (slot) => {
      const t5 = () => {
        return handleSelect(slot.time);
      };
      if (selectedSlot === slot.time) {
        t12 = "selected";
      } else {
        t12 = "";
      }
      return <button key={slot.time} onClick={t5} className={t12}>{slot.time}</button>;
    };
    t87 = <div><h4>Morning</h4>{morningSlots.map(t261)}</div>;
    t105 = afternoonSlots.length > 0;
    const t252 = (slot) => {
      const t5 = () => {
        return handleSelect(slot.time);
      };
      if (selectedSlot === slot.time) {
        t12 = "selected";
      } else {
        t12 = "";
      }
      return <button key={slot.time} onClick={t5} className={t12}>{slot.time}</button>;
    };
    t105 = <div><h4>Afternoon</h4>{afternoonSlots.map(t252)}</div>;
    t81 = <>{t87}{t105}</>;
  }
  let t214;
  if ($[11] !== t87 || $[12] !== afternoonSlots || $[13] !== afternoonSlots || $[14] !== selectedDate || $[15] !== timezone) {
    t214 = (
      <t166>
        {t172}
        {t81}
      </t166>
    );
    $[11] = t87;
    $[12] = afternoonSlots;
    $[13] = afternoonSlots;
    $[14] = selectedDate;
    $[15] = timezone;
    $[16] = t214;
  } else {
    t214 = $[16];
  }
  return t214;
}

