import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';

interface AvatarGroupProps {
  users: { name: string; image?: string }[];
  max?: number;
}

export function AvatarGroup(t0) {
  const $ = _c(3);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t43 = Discriminant(4) */
  } else {
  }
  const t44 = useMemo;
  /* t45 = Discriminant(28) */
  const t46 = users;
  const t47 = max;
  const t48 = [t46, t47];
  const t49 = t44(t45, t48);
  const visible = t49;
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t51 = Discriminant(4) */
  } else {
  }
  const t52 = users;
  const t53 = t52.length;
  const t54 = max;
  const t55 = t53 - t54;
  const remaining = t55;
  const t57 = "div";
  const t58 = "flex -space-x-2";
  const t59 = visible;
  /* t60 = Discriminant(28) */
  const t61 = t59.map(t60);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    const t62 = remaining;
  } else {
  }
  const t63 = 0;
}

