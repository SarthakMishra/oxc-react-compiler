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
  const $ = _c(29);
  const { slots, selectedDate, onSelect, timezone } = t0;
  let t8;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t8;
  } else {
    t8 = $[0];
  }
  let availableSlots;
  const t17 = () => {
    const t2 = (s) => {
      return s.available;
    };
    return slots.filter(t2);
  };
  const t20 = useMemo(t17, [slots]);
  let t90;
  let t25;
  let t27;
  if ($[1] !== t20) {
    availableSlots = t20;
    t25 = () => {
      const t2 = (s) => {
        return parseInt(s.time) < 12;
      };
      return availableSlots.filter(t2);
    };
    t27 = [availableSlots];
    $[1] = t20;
    $[2] = availableSlots;
    $[3] = t90;
    $[4] = t25;
    $[5] = t27;
  } else {
    availableSlots = $[2];
    t90 = $[3];
    t25 = $[4];
    t27 = $[5];
  }
  const morningSlots = t90;
  const t28 = useMemo(t25, t27);
  let t92;
  let t91;
  let t33;
  let t35;
  if ($[6] !== t28) {
    t91 = t28;
    t33 = () => {
      const t2 = (s) => {
        return parseInt(s.time) >= 12;
      };
      return availableSlots.filter(t2);
    };
    t35 = [availableSlots];
    $[6] = t28;
    $[7] = t91;
    $[8] = t92;
    $[9] = t33;
    $[10] = t35;
  } else {
    t91 = $[7];
    t92 = $[8];
    t33 = $[9];
    t35 = $[10];
  }
  const morningSlots = t91;
  const afternoonSlots = t92;
  const t36 = useMemo(t33, t35);
  let t93;
  if ($[11] !== t36) {
    t93 = t36;
    $[11] = t36;
    $[12] = t93;
  } else {
    t93 = $[12];
  }
  const afternoonSlots = t93;
  let t94;
  let t42;
  let t44;
  if ($[13] !== onSelect) {
    t42 = (time) => {
      const t4 = setSelectedSlot(time);
      const t8 = onSelect(time);
      return undefined;
    };
    t44 = [onSelect];
    $[13] = onSelect;
    $[14] = t94;
    $[15] = t42;
    $[16] = t44;
  } else {
    t94 = $[14];
    t42 = $[15];
    t44 = $[16];
  }
  const handleSelect = t94;
  const t45 = useCallback(t42, t44);
  let t95;
  if ($[17] !== t45) {
    t95 = t45;
    $[17] = t45;
    $[18] = t95;
  } else {
    t95 = $[18];
  }
  const handleSelect = t95;
  let t58;
  if (availableSlots.length === 0) {
    let t89;
    let t96;
    let t17;
    let t19;
    if ($[19] !== t20 || $[20] !== afternoonSlots.length || $[21] !== morningSlots.length || $[22] !== selectedDate || $[23] !== slots || $[24] !== timezone) {
      t58 = <p>No available slots</p>;
      $[19] = t20;
      $[20] = afternoonSlots.length;
      $[21] = morningSlots.length;
      $[22] = selectedDate;
      $[23] = slots;
      $[24] = timezone;
      $[25] = t89;
      $[26] = t96;
      $[27] = t17;
      $[28] = t19;
    } else {
      t89 = $[25];
      t96 = $[26];
      t17 = $[27];
      t19 = $[28];
    }
    availableSlots = t96;
  } else {
    let t62;
    t62 = morningSlots.length > 0;
    const t72 = (slot) => {
      const t4 = () => {
        return handleSelect(slot.time);
      };
      let t10;
      if (selectedSlot === slot.time) {
        t10 = "selected";
      } else {
        t10 = "";
      }
      return <button key={slot.time} onClick={t4} className={t10}>{slot.time}</button>;
    };
    t62 = <div><h4>Morning</h4>{morningSlots.map(t72)}</div>;
    let t75;
    t75 = afternoonSlots.length > 0;
    const t85 = (slot) => {
      const t4 = () => {
        return handleSelect(slot.time);
      };
      let t10;
      if (selectedSlot === slot.time) {
        t10 = "selected";
      } else {
        t10 = "";
      }
      return <button key={slot.time} onClick={t4} className={t10}>{slot.time}</button>;
    };
    t75 = <div><h4>Afternoon</h4>{afternoonSlots.map(t85)}</div>;
    t58 = <>{t62}{t75}</>;
  }
  return <div><h3>{selectedDate} ({timezone})</h3>{t58}</div>;
}

