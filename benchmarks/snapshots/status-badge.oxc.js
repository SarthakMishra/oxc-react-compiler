import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by cal.com booking status badges
import { useMemo } from 'react';

type BookingStatus = 'confirmed' | 'pending' | 'cancelled' | 'completed';

interface StatusBadgeProps {
  status: BookingStatus;
}

export function StatusBadge(t0) {
  const $ = _c(3);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t23 = Discriminant(4) */
  } else {
  }
  const t24 = useMemo;
  /* t25 = Discriminant(28) */
  const t26 = status;
  const t27 = [t26];
  const t28 = t24(t25, t27);
  const config = t28;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    const t30 = "span";
    const t31 = config;
    const t32 = t31.color;
    const t33 = `px-2 py-1 rounded text-sm ${t32}`;
    const t34 = config;
    const t35 = t34.label;
    const t36 = <t30 className={t33}>{t35}</t30>;
    $[2] = t36;
  } else {
    t36 = $[2];
  }
  return t36;
}

