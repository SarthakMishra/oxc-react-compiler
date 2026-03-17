import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';

interface AvatarGroupProps {
  users: { name: string; image?: string }[];
  max?: number;
}

export function AvatarGroup(t0) {
  const $ = _c(12);
  const { users, max } = t0;
  let t34;
  let t33;
  let t8;
  let t11;
  if ($[0] !== t24 || $[1] !== max || $[2] !== users) {
    t8 = () => {
      return users.slice(0, max);
    };
    t11 = [users, max];
    $[0] = t24;
    $[1] = max;
    $[2] = users;
    $[3] = t33;
    $[4] = t34;
    $[5] = t8;
    $[6] = t11;
  } else {
    t33 = $[3];
    t34 = $[4];
    t8 = $[5];
    t11 = $[6];
  }
  const visible = t34;
  const t12 = useMemo(t8, t11);
  let t35;
  if ($[7] !== t12) {
    t35 = t12;
    $[7] = t12;
    $[8] = t35;
  } else {
    t35 = $[8];
  }
  const visible = t35;
  let remaining;
  remaining = users.length - max;
  const t22 = (user, i) => {
    let t9;
    if (user.image) {
      t9 = <img src={user.image} alt={user.name} />;
    } else {
      t9 = user.name[0];
    }
    return <div key={i} className="rounded-full w-8 h-8 bg-gray-300" title={user.name}>{t9}</div>;
  };
  let t24;
  if ($[9] !== max || $[10] !== users.length) {
    t24 = remaining > 0;
    $[9] = max;
    $[10] = users.length;
    $[11] = t24;
  } else {
    t24 = $[11];
  }
  t24 = <div className="rounded-full w-8 h-8 bg-gray-200">+{remaining}</div>;
  return <div className="flex -space-x-2">{visible.map(t22)}{t24}</div>;
}

