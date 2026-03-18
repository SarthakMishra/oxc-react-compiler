import { c as _c } from "react/compiler-runtime";
import { useState } from 'react';

export function SimpleCounter() {
  const $ = _c(4);
  let t2;
  let t15;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = 0;
    $[0] = t2;
  } else {
    t2 = $[0];
  }
  let t3 = useState(t2);
  let count;
  let setCount;
  ([count, setCount] = t3);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    let t12 = () => {
      return setCount(count + 1);
    };
    t15 = (
      <div>
        <span>{count}</span>
        <button onClick={t12}>+</button>
      </div>
    );
    $[1] = t15;
    $[2] = count;
    $[3] = setCount;
  } else {
    t15 = $[1];
    count = $[2];
    setCount = $[3];
  }
  return t15;
}

