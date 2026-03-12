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
  const t52 = useMemo;
  const t53 = () => {
    const t1 = users;
    const t2 = 0;
    const t4 = max;
    const t5 = t1.slice(t2, t4);
    return t5;
  };
  const t54 = users;
  const t55 = max;
  const t56 = [t54, t55];
  const t57 = t52(t53, t56);
  visible = t57;
  let remaining;
  const t60 = users;
  const t61 = t60.length;
  const t62 = max;
  const t63 = t61 - t62;
  remaining = t63;
  const t65 = "div";
  const t66 = "flex -space-x-2";
  const t67 = visible;
  const t68 = (user, i) => {
    const t2 = "div";
    const t4 = i;
    const t5 = "rounded-full w-8 h-8 bg-gray-300";
    const t7 = user;
    const t8 = t7.name;
    const t10 = user;
    const t11 = t10.image;
    if (t11) {
      const t12 = "img";
      const t14 = user;
      const t15 = t14.image;
      const t17 = user;
      const t18 = t17.name;
      const t19 = _jsx(t12, { src: t15, alt: t18 });
    } else {
      const t21 = user;
      const t22 = t21.name;
      const t23 = 0;
      const t24 = t22[t23];
    }
    const t26 = _jsx(t2, { key: t4, className: t5, title: t8, children: t25 });
    return t26;
  };
  const t69 = t67.map(t68);
  let t83;
  if ($[3] !== t45 || $[4] !== useMemo || $[5] !== users || $[6] !== max || $[7] !== visible || $[8] !== remaining || $[9] !== users || $[10] !== max || $[11] !== visible || $[12] !== remaining) {
    const t70 = remaining;
    $[13] = t83;
    $[3] = t45;
    $[4] = useMemo;
    $[5] = users;
    $[6] = max;
    $[7] = visible;
    $[8] = remaining;
    $[9] = users;
    $[10] = max;
    $[11] = visible;
    $[12] = remaining;
  } else {
    t83 = $[13];
  }
  const t71 = 0;
  const t73 = "div";
  const t74 = "rounded-full w-8 h-8 bg-gray-200";
  const t75 = "+";
  if ($[14] !== remaining) {
    const t76 = remaining;
    $[14] = remaining;
  } else {
  }
  t83 = _jsxs(t65, { className: t66, children: [t69, t45] });
  return t83;
}

