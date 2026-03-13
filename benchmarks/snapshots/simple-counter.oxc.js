import { c as _c } from "react/compiler-runtime";
import { useState } from 'react';

export function SimpleCounter() {
  const $ = _c(2);
  let t34;
  if ($[0] !== count) {
    const t31 = () => {
      return setCount(count + 1);
    };
    t34 = (
      <div>
        <span>{count}</span>
        <button onClick={t31}>+</button>
      </div>
    );
    $[0] = count;
    $[1] = t34;
  } else {
    t34 = $[1];
  }
  return t34;
}

