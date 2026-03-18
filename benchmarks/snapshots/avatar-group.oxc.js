import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';

interface AvatarGroupProps {
  users: { name: string; image?: string }[];
  max?: number;
}

export function AvatarGroup(t0) {
  const $ = _c(13);
  let t34;
  let t8;
  let t11;
  let visible;
  let t35;
  let t19;
  let t20;
  let t23;
  let t24;
  let { users, max } = t0;
  if ($[0] !== max || $[1] !== users) {
    t8 = () => {
      return users.slice(0, max);
    };
    t11 = [users, max];
    $[0] = max;
    $[1] = users;
    $[2] = t34;
    $[3] = t8;
    $[4] = t11;
  } else {
    t34 = $[2];
    t8 = $[3];
    t11 = $[4];
  }
  visible = t34;
  let t12 = useMemo(t8, t11);
  if ($[5] !== t12) {
    visible = t12;
    t35 = users.length - max;
    t19 = "div";
    t20 = "flex -space-x-2";
    let t22 = (user, i) => {
      let t9;
      if (user.image) {
        t9 = <img src={user.image} alt={user.name} />;
      } else {
        t9 = user.name[0];
      }
      return <div key={i} className="rounded-full w-8 h-8 bg-gray-300" title={user.name}>{t9}</div>;
    };
    t23 = visible.map(t22);
    $[5] = t12;
    $[6] = visible;
    $[7] = t35;
    $[8] = t19;
    $[9] = t20;
    $[10] = t23;
  } else {
    visible = $[6];
    t35 = $[7];
    t19 = $[8];
    t20 = $[9];
    t23 = $[10];
  }
  let remaining = t35;
  if ($[11] !== t12) {
    t24 = remaining > 0;
    t24 = <div className="rounded-full w-8 h-8 bg-gray-200">+{remaining}</div>;
    return <div className={t20}>{t23}{t24}</div>;
    $[11] = t12;
    $[12] = t24;
  } else {
    t24 = $[12];
  }
}

