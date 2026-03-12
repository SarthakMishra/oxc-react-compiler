import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useState } from 'react';

export function SimpleCounter() {
  const $ = _c(4);
  const t20 = useState;
  const t21 = 0;
  const t22 = t20(t21);
  let count;
  let setCount;
  if ($[0] !== count || $[1] !== setCount) {
    $[0] = count;
    $[1] = setCount;
  } else {
  }
  ([count, setCount] = t22);
  let t34;
  if ($[2] !== count) {
    const t26 = "div";
    const t27 = "span";
    const t28 = count;
    const t29 = _jsx(t27, { children: t28 });
    const t30 = "button";
    const t31 = () => {
      const t1 = setCount;
      const t3 = count;
      const t4 = 1;
      const t5 = t3 + t4;
      const t6 = t1(t5);
      return t6;
    };
    const t32 = "+";
    const t33 = _jsx(t30, { onClick: t31, children: t32 });
    t34 = _jsxs(t26, { children: [t29, t33] });
    $[3] = t34;
    $[2] = count;
  } else {
    t34 = $[3];
  }
  return t34;
}

