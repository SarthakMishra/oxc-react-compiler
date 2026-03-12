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
  const $ = _c(34);
  const { slots, selectedDate, onSelect, timezone } = t0;
  if ($[0] !== slots || $[1] !== selectedDate || $[2] !== onSelect || $[3] !== timezone) {
    $[0] = slots;
    $[1] = selectedDate;
    $[2] = onSelect;
    $[3] = timezone;
  } else {
  }
  const t132 = useState;
  const t133 = null;
  const t134 = t132(t133);
  let selectedSlot;
  let setSelectedSlot;
  if ($[4] !== selectedSlot || $[5] !== setSelectedSlot) {
    $[4] = selectedSlot;
    $[5] = setSelectedSlot;
  } else {
  }
  ([selectedSlot, setSelectedSlot] = t134);
  let availableSlots;
  if ($[6] !== availableSlots) {
    $[6] = availableSlots;
  } else {
  }
  let morningSlots;
  if ($[7] !== useMemo || $[8] !== slots || $[9] !== availableSlots || $[10] !== morningSlots) {
    const t139 = useMemo;
    const t140 = () => {
      const t1 = slots;
      const t2 = (s) => {
        const t2 = s;
        const t3 = t2.available;
        return t3;
      };
      const t3 = t1.filter(t2);
      return t3;
    };
    const t141 = slots;
    const t142 = [t141];
    const t143 = t139(t140, t142);
    availableSlots = t143;
    $[7] = useMemo;
    $[8] = slots;
    $[9] = availableSlots;
    $[10] = morningSlots;
  } else {
  }
  let afternoonSlots;
  if ($[11] !== useMemo || $[12] !== availableSlots || $[13] !== morningSlots || $[14] !== afternoonSlots) {
    const t146 = useMemo;
    const t147 = () => {
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
    const t148 = availableSlots;
    const t149 = [t148];
    const t150 = t146(t147, t149);
    morningSlots = t150;
    $[11] = useMemo;
    $[12] = availableSlots;
    $[13] = morningSlots;
    $[14] = afternoonSlots;
  } else {
  }
  let handleSelect;
  if ($[15] !== useMemo || $[16] !== availableSlots || $[17] !== afternoonSlots || $[18] !== handleSelect) {
    const t153 = useMemo;
    const t154 = () => {
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
    const t155 = availableSlots;
    const t156 = [t155];
    const t157 = t153(t154, t156);
    afternoonSlots = t157;
    $[15] = useMemo;
    $[16] = availableSlots;
    $[17] = afternoonSlots;
    $[18] = handleSelect;
  } else {
  }
  const t160 = useCallback;
  const t161 = (time) => {
    const t2 = setSelectedSlot;
    const t4 = time;
    const t5 = t2(t4);
    const t7 = onSelect;
    const t9 = time;
    const t10 = t7(t9);
    const t11 = undefined;
    return t11;
  };
  const t162 = onSelect;
  const t163 = [t162];
  const t164 = t160(t161, t163);
  handleSelect = t164;
  const t166 = "div";
  const t167 = "h3";
  const t168 = selectedDate;
  const t169 = " (";
  const t170 = timezone;
  const t171 = ")";
  const t172 = _jsxs(t167, { children: [t168, t169, t170, t171] });
  const t173 = availableSlots;
  const t174 = t173.length;
  const t175 = 0;
  const t176 = t174 === t175;
  let t81;
  if (t176) {
    const t265 = "p";
    const t266 = "No available slots";
    const t267 = _jsx(t265, { children: t266 });
    t81 = t267;
  } else {
    let t87;
    if ($[19] !== morningSlots || $[20] !== t87 || $[21] !== morningSlots) {
      const t216 = morningSlots;
      const t217 = t216.length;
      const t218 = 0;
      const t219 = t217 > t218;
      t87 = t219;
      $[19] = morningSlots;
      $[20] = t87;
      $[21] = morningSlots;
    } else {
    }
    const t256 = "div";
    const t257 = "h4";
    const t258 = "Morning";
    const t259 = _jsx(t257, { children: t258 });
    const t260 = morningSlots;
    const t261 = (slot) => {
      const t1 = "button";
      const t3 = slot;
      const t4 = t3.time;
      const t5 = () => {
        const t1 = handleSelect;
        const t3 = slot;
        const t4 = t3.time;
        const t5 = t1(t4);
        return t5;
      };
      const t7 = selectedSlot;
      const t9 = slot;
      const t10 = t9.time;
      const t11 = t7 === t10;
      let t12;
      if (t11) {
        const t14 = "selected";
        t12 = t14;
      } else {
        const t16 = "";
        t12 = t16;
      }
      const t19 = slot;
      const t20 = t19.time;
      const t21 = _jsx(t1, { key: t4, onClick: t5, className: t12, children: t20 });
      return t21;
    };
    const t262 = t260.map(t261);
    const t263 = _jsxs(t256, { children: [t259, t262] });
    t87 = t263;
    let t105;
    if ($[22] !== afternoonSlots || $[23] !== t105 || $[24] !== afternoonSlots) {
      const t231 = afternoonSlots;
      const t232 = t231.length;
      const t233 = 0;
      const t234 = t232 > t233;
      t105 = t234;
      $[22] = afternoonSlots;
      $[23] = t105;
      $[24] = afternoonSlots;
    } else {
    }
    const t247 = "div";
    const t248 = "h4";
    const t249 = "Afternoon";
    const t250 = _jsx(t248, { children: t249 });
    const t251 = afternoonSlots;
    const t252 = (slot) => {
      const t1 = "button";
      const t3 = slot;
      const t4 = t3.time;
      const t5 = () => {
        const t1 = handleSelect;
        const t3 = slot;
        const t4 = t3.time;
        const t5 = t1(t4);
        return t5;
      };
      const t7 = selectedSlot;
      const t9 = slot;
      const t10 = t9.time;
      const t11 = t7 === t10;
      let t12;
      if (t11) {
        const t14 = "selected";
        t12 = t14;
      } else {
        const t16 = "";
        t12 = t16;
      }
      const t19 = slot;
      const t20 = t19.time;
      const t21 = _jsx(t1, { key: t4, onClick: t5, className: t12, children: t20 });
      return t21;
    };
    const t253 = t251.map(t252);
    const t254 = _jsxs(t247, { children: [t250, t253] });
    t105 = t254;
    const t245 = _jsxs(_Fragment, { children: [t87, t105] });
    t81 = t245;
  }
  let t214;
  if ($[25] !== useCallback || $[26] !== onSelect || $[27] !== handleSelect || $[28] !== selectedDate || $[29] !== timezone || $[30] !== availableSlots || $[31] !== t175 || $[32] !== t81) {
    t214 = _jsxs(t166, { children: [t172, t81] });
    $[33] = t214;
    $[25] = useCallback;
    $[26] = onSelect;
    $[27] = handleSelect;
    $[28] = selectedDate;
    $[29] = timezone;
    $[30] = availableSlots;
    $[31] = t175;
    $[32] = t81;
  } else {
    t214 = $[33];
  }
  return t214;
}

