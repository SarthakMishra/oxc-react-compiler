import { c as _c } from "react/compiler-runtime";
import { useState } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function SimpleCounter() {
  const $ = _c(2);
  const [count, setCount] = useState(0);
  let t0;
  if ($[0] !== count) {
    t0 = /*#__PURE__*/_jsxs("div", {
      children: [/*#__PURE__*/_jsx("span", {
        children: count
      }), /*#__PURE__*/_jsx("button", {
        onClick: () => setCount(count + 1),
        children: "+"
      })]
    });
    $[0] = count;
    $[1] = t0;
  } else {
    t0 = $[1];
  }
  return t0;
}