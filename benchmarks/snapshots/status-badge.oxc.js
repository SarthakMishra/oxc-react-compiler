import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by cal.com booking status badges
import { useMemo } from 'react';

type BookingStatus = 'confirmed' | 'pending' | 'cancelled' | 'completed';

interface StatusBadgeProps {
  status: BookingStatus;
}

export function StatusBadge(t0) {
  const $ = _c(7);
  let t19;
  let t7;
  let t9;
  let config;
  let t18;
  let { status } = t0;
  if ($[0] !== status) {
    t7 = () => {
      switch (status) {
        case "confirmed":
          return { label: "Confirmed", color: "bg-green-100 text-green-800" };
        case "pending":
          return { label: "Pending", color: "bg-yellow-100 text-yellow-800" };
        case "cancelled":
          return { label: "Cancelled", color: "bg-red-100 text-red-800" };
        case "completed":
          return { label: "Completed", color: "bg-gray-100 text-gray-800" };
      }
      return undefined;
    };
    t9 = [status];
    $[0] = status;
    $[1] = t19;
    $[2] = t7;
    $[3] = t9;
  } else {
    t19 = $[1];
    t7 = $[2];
    t9 = $[3];
  }
  config = t19;
  let t10 = useMemo(t7, t9);
  if ($[4] !== t10) {
    config = t10;
    t18 = <span className={`px-2 py-1 rounded text-sm ${config.color}`}>{config.label}</span>;
    $[4] = t10;
    $[5] = config;
    $[6] = t18;
  } else {
    config = $[5];
    t18 = $[6];
  }
  return t18;
}

