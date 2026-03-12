import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// XS tier - Inspired by cal.com booking status badges
import { useMemo } from 'react';

type BookingStatus = 'confirmed' | 'pending' | 'cancelled' | 'completed';

interface StatusBadgeProps {
  status: BookingStatus;
}

export function StatusBadge(t0) {
  const $ = _c(8);
  let status;
  if ($[0] !== status) {
    $[0] = status;
  } else {
  }
  ({ status } = t0);
  let config;
  if ($[1] !== config) {
    $[1] = config;
  } else {
  }
  let t41;
  if ($[2] !== useMemo || $[3] !== status || $[4] !== config || $[5] !== config || $[6] !== config) {
    const t29 = useMemo;
    const t30 = () => {
      const t1 = status;
      const t2 = "confirmed";
      const t3 = "pending";
      const t4 = "cancelled";
      const t5 = "completed";
      const t18 = undefined;
      return t18;
      const t6 = "Confirmed";
      const t7 = "bg-green-100 text-green-800";
      const t8 = { label: t6, color: t7 };
      return t8;
      const t9 = "Pending";
      const t10 = "bg-yellow-100 text-yellow-800";
      const t11 = { label: t9, color: t10 };
      return t11;
      const t12 = "Cancelled";
      const t13 = "bg-red-100 text-red-800";
      const t14 = { label: t12, color: t13 };
      return t14;
      const t15 = "Completed";
      const t16 = "bg-gray-100 text-gray-800";
      const t17 = { label: t15, color: t16 };
      return t17;
    };
    const t31 = status;
    const t32 = [t31];
    const t33 = t29(t30, t32);
    config = t33;
    const t35 = "span";
    const t36 = config;
    const t37 = t36.color;
    const t38 = `px-2 py-1 rounded text-sm ${t37}`;
    const t39 = config;
    const t40 = t39.label;
    t41 = _jsx(t35, { className: t38, children: t40 });
    $[7] = t41;
    $[2] = useMemo;
    $[3] = status;
    $[4] = config;
    $[5] = config;
    $[6] = config;
  } else {
    t41 = $[7];
  }
  return t41;
}

