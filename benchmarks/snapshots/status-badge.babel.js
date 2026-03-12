import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by cal.com booking status badges
import { useMemo } from 'react';
import { jsx as _jsx } from "react/jsx-runtime";
export function StatusBadge(t0) {
  const $ = _c(7);
  const {
    status
  } = t0;
  let t1;
  bb0: {
    switch (status) {
      case "confirmed":
        {
          let t2;
          if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
            t2 = {
              label: "Confirmed",
              color: "bg-green-100 text-green-800"
            };
            $[0] = t2;
          } else {
            t2 = $[0];
          }
          t1 = t2;
          break bb0;
        }
      case "pending":
        {
          let t2;
          if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
            t2 = {
              label: "Pending",
              color: "bg-yellow-100 text-yellow-800"
            };
            $[1] = t2;
          } else {
            t2 = $[1];
          }
          t1 = t2;
          break bb0;
        }
      case "cancelled":
        {
          let t2;
          if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
            t2 = {
              label: "Cancelled",
              color: "bg-red-100 text-red-800"
            };
            $[2] = t2;
          } else {
            t2 = $[2];
          }
          t1 = t2;
          break bb0;
        }
      case "completed":
        {
          let t2;
          if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
            t2 = {
              label: "Completed",
              color: "bg-gray-100 text-gray-800"
            };
            $[3] = t2;
          } else {
            t2 = $[3];
          }
          t1 = t2;
          break bb0;
        }
    }
    t1 = undefined;
  }
  const config = t1;
  const t2 = `px-2 py-1 rounded text-sm ${config.color}`;
  let t3;
  if ($[4] !== config.label || $[5] !== t2) {
    t3 = /*#__PURE__*/_jsx("span", {
      className: t2,
      children: config.label
    });
    $[4] = config.label;
    $[5] = t2;
    $[6] = t3;
  } else {
    t3 = $[6];
  }
  return t3;
}