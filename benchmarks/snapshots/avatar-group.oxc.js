import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';

interface AvatarGroupProps {
  users: { name: string; image?: string }[];
  max?: number;
}

export function AvatarGroup(t0) {
  const $ = _c(15);
  const { users, max } = t0;
  if ($[0] !== users || $[1] !== max) {
    $[0] = users;
    $[1] = max;
  } else {
  }
  let visible;
  if ($[2] !== visible) {
    $[2] = visible;
  } else {
  }
  const t55 = useMemo;
  const t56 = () => {
    const t1 = users;
    const t2 = 0;
    const t4 = max;
    const t5 = t1.slice(t2, t4);
    return t5;
  };
  const t57 = users;
  const t58 = max;
  const t59 = [t57, t58];
  const t60 = t55(t56, t59);
  visible = t60;
  let remaining;
  const t63 = users;
  const t64 = t63.length;
  const t65 = max;
  const t66 = t64 - t65;
  remaining = t66;
  const t68 = "div";
  const t69 = "flex -space-x-2";
  const t70 = visible;
  const t71 = (user, i) => {
    const t2 = "div";
    const t4 = i;
    const t5 = "rounded-full w-8 h-8 bg-gray-300";
    const t7 = user;
    const t8 = t7.name;
    const t10 = user;
    const t11 = t10.image;
    let t12;
    if (t11) {
      const t14 = "img";
      const t16 = user;
      const t17 = t16.image;
      const t19 = user;
      const t20 = t19.name;
      const t21 = _jsx(t14, { src: t17, alt: t20 });
      t12 = t21;
    } else {
      const t24 = user;
      const t25 = t24.name;
      const t26 = 0;
      const t27 = t25[t26];
      t12 = t27;
    }
    const t29 = _jsx(t2, { key: t4, className: t5, title: t8, children: t12 });
    return t29;
  };
  const t72 = t70.map(t71);
  let t35;
  let t90;
  if ($[3] !== remaining || $[4] !== t35 || $[5] !== useMemo || $[6] !== users || $[7] !== max || $[8] !== visible || $[9] !== remaining || $[10] !== users || $[11] !== max || $[12] !== visible || $[13] !== remaining) {
    const t74 = remaining;
    const t75 = 0;
    const t76 = t74 > t75;
    t35 = t76;
    $[14] = t90;
    $[3] = remaining;
    $[4] = t35;
    $[5] = useMemo;
    $[6] = users;
    $[7] = max;
    $[8] = visible;
    $[9] = remaining;
    $[10] = users;
    $[11] = max;
    $[12] = visible;
    $[13] = remaining;
  } else {
    t90 = $[14];
  }
  const t78 = "div";
  const t79 = "rounded-full w-8 h-8 bg-gray-200";
  const t80 = "+";
  const t81 = remaining;
  const t82 = _jsxs(t78, { className: t79, children: [t80, t81] });
  t35 = t82;
  t90 = _jsxs(t68, { className: t69, children: [t72, t35] });
  return t90;
}

