import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by cal.com booking status badges
import { useMemo } from 'react';

type BookingStatus = 'confirmed' | 'pending' | 'cancelled' | 'completed';

interface StatusBadgeProps {
  status: BookingStatus;
}

export function StatusBadge(t0) {
  const $ = _c(3);
  const { status } = t0;
  let t41;
  if ($[0] !== status || $[1] !== useMemo) {
    const t30 = () => {
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
    const config = useMemo(t30, [status]);
    t41 = <span className={`px-2 py-1 rounded text-sm ${config.color}`}>{config.label}</span>;
    $[0] = status;
    $[1] = useMemo;
    $[2] = t41;
  } else {
    t41 = $[2];
  }
  return t41;
}

