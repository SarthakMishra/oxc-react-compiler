import { c as _c } from "react/compiler-runtime";
// S tier - Inspired by cal.com time slot picker
import { useState, useMemo, useCallback } from 'react';
import { jsxs as _jsxs, jsx as _jsx, Fragment as _Fragment } from "react/jsx-runtime";
export function TimeSlotPicker(t0) {
  const $ = _c(20);
  const {
    slots,
    selectedDate,
    onSelect,
    timezone: t1
  } = t0;
  const timezone = t1 === undefined ? "UTC" : t1;
  const [selectedSlot, setSelectedSlot] = useState(null);
  let t2;
  if ($[0] !== slots) {
    t2 = slots.filter(_temp);
    $[0] = slots;
    $[1] = t2;
  } else {
    t2 = $[1];
  }
  const availableSlots = t2;
  let t3;
  if ($[2] !== availableSlots) {
    t3 = availableSlots.filter(_temp2);
    $[2] = availableSlots;
    $[3] = t3;
  } else {
    t3 = $[3];
  }
  const morningSlots = t3;
  let t4;
  if ($[4] !== availableSlots) {
    t4 = availableSlots.filter(_temp3);
    $[4] = availableSlots;
    $[5] = t4;
  } else {
    t4 = $[5];
  }
  const afternoonSlots = t4;
  let t5;
  if ($[6] !== onSelect) {
    t5 = time => {
      setSelectedSlot(time);
      onSelect(time);
    };
    $[6] = onSelect;
    $[7] = t5;
  } else {
    t5 = $[7];
  }
  const handleSelect = t5;
  let t6;
  if ($[8] !== selectedDate || $[9] !== timezone) {
    t6 = /*#__PURE__*/_jsxs("h3", {
      children: [selectedDate, " (", timezone, ")"]
    });
    $[8] = selectedDate;
    $[9] = timezone;
    $[10] = t6;
  } else {
    t6 = $[10];
  }
  let t7;
  if ($[11] !== afternoonSlots || $[12] !== availableSlots.length || $[13] !== handleSelect || $[14] !== morningSlots || $[15] !== selectedSlot) {
    t7 = availableSlots.length === 0 ? /*#__PURE__*/_jsx("p", {
      children: "No available slots"
    }) : /*#__PURE__*/_jsxs(_Fragment, {
      children: [morningSlots.length > 0 && /*#__PURE__*/_jsxs("div", {
        children: [/*#__PURE__*/_jsx("h4", {
          children: "Morning"
        }), morningSlots.map(slot => /*#__PURE__*/_jsx("button", {
          onClick: () => handleSelect(slot.time),
          className: selectedSlot === slot.time ? "selected" : "",
          children: slot.time
        }, slot.time))]
      }), afternoonSlots.length > 0 && /*#__PURE__*/_jsxs("div", {
        children: [/*#__PURE__*/_jsx("h4", {
          children: "Afternoon"
        }), afternoonSlots.map(slot_0 => /*#__PURE__*/_jsx("button", {
          onClick: () => handleSelect(slot_0.time),
          className: selectedSlot === slot_0.time ? "selected" : "",
          children: slot_0.time
        }, slot_0.time))]
      })]
    });
    $[11] = afternoonSlots;
    $[12] = availableSlots.length;
    $[13] = handleSelect;
    $[14] = morningSlots;
    $[15] = selectedSlot;
    $[16] = t7;
  } else {
    t7 = $[16];
  }
  let t8;
  if ($[17] !== t6 || $[18] !== t7) {
    t8 = /*#__PURE__*/_jsxs("div", {
      children: [t6, t7]
    });
    $[17] = t6;
    $[18] = t7;
    $[19] = t8;
  } else {
    t8 = $[19];
  }
  return t8;
}
function _temp3(s_1) {
  return parseInt(s_1.time) >= 12;
}
function _temp2(s_0) {
  return parseInt(s_0.time) < 12;
}
function _temp(s) {
  return s.available;
}