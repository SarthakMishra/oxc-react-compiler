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
  const $ = _c(6);
  const t109 = useState;
  const t110 = null;
  const t111 = t109(t110);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t112 = Discriminant(4) */
    /* t113 = Discriminant(4) */
  } else {
  }
  /* t114 = Discriminant(6) */
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t115 = Discriminant(4) */
  } else {
  }
  const t116 = useMemo;
  /* t117 = Discriminant(28) */
  const t118 = slots;
  const t119 = [t118];
  const t120 = t116(t117, t119);
  const availableSlots = t120;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t122 = Discriminant(4) */
  } else {
  }
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    const t123 = useMemo;
    /* t124 = Discriminant(28) */
    const t125 = availableSlots;
    const t126 = [t125];
    const t127 = t123(t124, t126);
    const morningSlots = t127;
    /* t129 = Discriminant(4) */
  } else {
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    const t130 = useMemo;
    /* t131 = Discriminant(28) */
    const t132 = availableSlots;
    const t133 = [t132];
    const t134 = t130(t131, t133);
    const afternoonSlots = t134;
    /* t136 = Discriminant(4) */
  } else {
  }
  const t137 = useCallback;
  /* t138 = Discriminant(28) */
  const t139 = onSelect;
  const t140 = [t139];
  const t141 = t137(t138, t140);
  const handleSelect = t141;
  const t143 = "div";
  const t144 = "h3";
  const t145 = selectedDate;
  /* t146 = Discriminant(8) */
  const t147 = timezone;
  /* t148 = Discriminant(8) */
  const t149 = <t144>{t145}{t146}{t147}{t148}</t144>;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    const t150 = availableSlots;
    const t151 = t150.length;
    const t152 = 0;
    const t153 = t151 === t152;
  } else {
  }
}

