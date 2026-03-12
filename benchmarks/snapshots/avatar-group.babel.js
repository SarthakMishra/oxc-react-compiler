import { c as _c } from "react/compiler-runtime";
// XS tier - Inspired by shadcn/ui avatar component
import { useMemo } from 'react';
import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
export function AvatarGroup(t0) {
  const $ = _c(10);
  const {
    users,
    max: t1
  } = t0;
  const max = t1 === undefined ? 3 : t1;
  let t2;
  if ($[0] !== max || $[1] !== users) {
    t2 = users.slice(0, max);
    $[0] = max;
    $[1] = users;
    $[2] = t2;
  } else {
    t2 = $[2];
  }
  const visible = t2;
  const remaining = users.length - max;
  let t3;
  if ($[3] !== visible) {
    t3 = visible.map(_temp);
    $[3] = visible;
    $[4] = t3;
  } else {
    t3 = $[4];
  }
  let t4;
  if ($[5] !== remaining) {
    t4 = remaining > 0 && /*#__PURE__*/_jsxs("div", {
      className: "rounded-full w-8 h-8 bg-gray-200",
      children: ["+", remaining]
    });
    $[5] = remaining;
    $[6] = t4;
  } else {
    t4 = $[6];
  }
  let t5;
  if ($[7] !== t3 || $[8] !== t4) {
    t5 = /*#__PURE__*/_jsxs("div", {
      className: "flex -space-x-2",
      children: [t3, t4]
    });
    $[7] = t3;
    $[8] = t4;
    $[9] = t5;
  } else {
    t5 = $[9];
  }
  return t5;
}
function _temp(user, i) {
  return /*#__PURE__*/_jsx("div", {
    className: "rounded-full w-8 h-8 bg-gray-300",
    title: user.name,
    children: user.image ? /*#__PURE__*/_jsx("img", {
      src: user.image,
      alt: user.name
    }) : user.name[0]
  }, i);
}