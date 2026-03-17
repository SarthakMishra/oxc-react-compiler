import { c as _c } from "react/compiler-runtime";
import { useState } from 'react';

export function SimpleCounter() {
  const $ = _c(3);
  let t15;
  let t2;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = 0;
    $[0] = t15;
    $[1] = t2;
  } else {
    t15 = $[0];
    t2 = $[1];
  }
  let t16;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    $[2] = t16;
  } else {
    t16 = $[2];
  }
  const count = t16;
  let setCount;
  const t12 = () => {
    return setCount(count + 1);
  };
  return <div><span>{count}</span><button onClick={t12}>+</button></div>;
}

