import { c as _c } from "react/compiler-runtime";
import { useState } from 'react';

export function SimpleCounter() {
  const $ = _c(3);
  const t20 = useState;
  const t21 = 0;
  const t22 = t20(t21);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t23 = Discriminant(4) */
    /* t24 = Discriminant(4) */
  } else {
  }
  /* t25 = Discriminant(6) */
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    const t26 = "div";
    const t27 = "span";
    const t28 = count;
    const t29 = <t27>{t28}</t27>;
    const t30 = "button";
    /* t31 = Discriminant(28) */
    /* t32 = Discriminant(8) */
    const t33 = <t30 onClick={t31}>{t32}</t30>;
    const t34 = <t26>{t29}{t33}</t26>;
    $[2] = t34;
  } else {
    t34 = $[2];
  }
  return t34;
}

