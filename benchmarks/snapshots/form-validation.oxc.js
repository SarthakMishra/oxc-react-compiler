import { c as _c } from "react/compiler-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(7);
  const t172 = useState;
  const t173 = "";
  const t174 = "";
  const t175 = "";
  const t176 = { email: t173, password: t174, confirmPassword: t175 };
  const t177 = t172(t176);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t178 = Discriminant(4) */
    /* t179 = Discriminant(4) */
  } else {
  }
  /* t180 = Discriminant(6) */
  const t181 = useState;
  const t182 = false;
  const t183 = t181(t182);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t184 = Discriminant(4) */
    /* t185 = Discriminant(4) */
  } else {
  }
  /* t186 = Discriminant(6) */
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t187 = Discriminant(4) */
  } else {
  }
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    const t188 = useMemo;
    /* t189 = Discriminant(28) */
    const t190 = form;
    const t191 = [t190];
    const t192 = t188(t189, t191);
    const errors = t192;
    /* t194 = Discriminant(4) */
  } else {
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    const t195 = useMemo;
    /* t196 = Discriminant(28) */
    const t197 = errors;
    const t198 = [t197];
    const t199 = t195(t196, t198);
    const isValid = t199;
    /* t201 = Discriminant(4) */
  } else {
  }
  const t202 = useCallback;
  /* t203 = Discriminant(28) */
  const t204 = [];
  const t205 = t202(t203, t204);
  const handleChange = t205;
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    /* t207 = Discriminant(4) */
  } else {
  }
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    const t208 = useCallback;
    /* t209 = Discriminant(28) */
    const t210 = isValid;
    const t211 = form;
    const t212 = [t210, t211];
    const t213 = t208(t209, t212);
    const handleSubmit = t213;
  } else {
  }
  const t215 = "div";
  const t216 = "h2";
  /* t217 = Discriminant(8) */
  const t218 = <t216>{t217}</t216>;
  const t219 = "div";
  const t220 = "input";
  const t221 = "email";
  const t222 = form;
  const t223 = t222.email;
  const t224 = handleChange;
  const t225 = "email";
  const t226 = t224(t225);
  const t227 = "Email";
  const t228 = <t220 type={t221} value={t223} onChange={t226} placeholder={t227} />;
}

