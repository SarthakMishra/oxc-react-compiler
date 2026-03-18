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
  const $ = _c(26);
  let t8;
  let availableSlots;
  let t90;
  let t25;
  let t27;
  let t91;
  let t92;
  let t33;
  let t35;
  let t93;
  let t42;
  let t44;
  let t94;
  let t47;
  let t53;
  let t58;
  let t62;
  let t75;
  let { slots, selectedDate, onSelect, timezone } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = null;
    $[0] = t8;
  } else {
    t8 = $[0];
  }
  let selectedSlot;
  let setSelectedSlot;
  ([selectedSlot, setSelectedSlot] = useState(t8));
  let t17 = () => {
    let t2 = (s) => {
      return s.available;
    };
    return slots.filter(t2);
  };
  let t20 = useMemo(t17, [slots]);
  if ($[1] !== t20) {
    availableSlots = t20;
    t25 = () => {
      let t2 = (s) => {
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
  let morningSlots = t90;
  let t28 = useMemo(t25, t27);
  if ($[6] !== t28) {
    t91 = t28;
    t33 = () => {
      let t2 = (s) => {
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
  morningSlots = t91;
  let afternoonSlots = t92;
  afternoonSlots = useMemo(t33, t35);
  if ($[11] !== onSelect) {
    t42 = (time) => {
      let t4 = setSelectedSlot(time);
      let t8 = onSelect(time);
      return undefined;
    };
    t44 = [onSelect];
    $[11] = onSelect;
    $[12] = t93;
    $[13] = t42;
    $[14] = t44;
  } else {
    t93 = $[12];
    t42 = $[13];
    t44 = $[14];
  }
  let handleSelect = t93;
  let t45 = useCallback(t42, t44);
  if ($[15] !== t45) {
    t94 = t45;
    $[15] = t45;
    $[16] = t94;
  } else {
    t94 = $[16];
  }
  handleSelect = t94;
  if ($[17] !== availableSlots.length || $[18] !== selectedDate || $[19] !== timezone) {
    if (availableSlots.length === 0) {
      if ($[20] === Symbol.for("react.memo_cache_sentinel")) {
        t58 = <p>No available slots</p>;
        $[20] = t58;
      } else {
        t58 = $[20];
      }
    } else {
      t62 = morningSlots.length > 0;
      let t72 = (slot) => {
        let t4 = () => {
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
      t75 = afternoonSlots.length > 0;
      let t85 = (slot) => {
        let t4 = () => {
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
    $[17] = availableSlots.length;
    $[18] = selectedDate;
    $[19] = timezone;
    $[20] = t47;
    $[21] = t53;
    $[22] = t58;
    $[23] = t62;
    $[24] = t75;
  } else {
    t47 = $[20];
    t53 = $[21];
    t58 = $[22];
    t62 = $[23];
    t75 = $[24];
  }
}

