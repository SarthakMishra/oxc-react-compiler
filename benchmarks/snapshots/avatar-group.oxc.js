import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';

interface AvatarGroupProps {
  users: { name: string; image?: string }[];
  max?: number;
}

export function AvatarGroup(t0) {
  const $ = _c(6);
  const { users, max } = t0;
  const t56 = () => {
    return users.slice(0, max);
  };
  const visible = useMemo(t56, [users, max]);
  const remaining = users.length - max;
  const t71 = (user, i) => {
    if (user.image) {
      t12 = <img src={user.image} alt={user.name} />;
    } else {
      t12 = user.name[0];
    }
    return <div key={i} className="rounded-full w-8 h-8 bg-gray-300" title={user.name}>{t12}</div>;
  };
  let t90;
  if ($[0] !== max || $[1] !== max || $[2] !== useMemo || $[3] !== users || $[4] !== users) {
    t35 = remaining > 0;
    $[0] = max;
    $[1] = max;
    $[2] = useMemo;
    $[3] = users;
    $[4] = users;
    $[5] = t90;
  } else {
    t90 = $[5];
  }
  t35 = <div className="rounded-full w-8 h-8 bg-gray-200">+{remaining}</div>;
  return <div className="flex -space-x-2">{visible.map(t71)}{t35}</div>;
}

