import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(18);
  const t172 = useState;
  const t173 = "";
  const t174 = "";
  const t175 = "";
  const t176 = { email: t173, password: t174, confirmPassword: t175 };
  const t177 = t172(t176);
  let form;
  let setForm;
  if ($[0] !== form || $[1] !== setForm) {
    $[0] = form;
    $[1] = setForm;
  } else {
  }
  ([form, setForm] = t177);
  const t181 = useState;
  const t182 = false;
  const t183 = t181(t182);
  let submitted;
  let setSubmitted;
  if ($[2] !== submitted || $[3] !== setSubmitted) {
    $[2] = submitted;
    $[3] = setSubmitted;
  } else {
  }
  ([submitted, setSubmitted] = t183);
  let errors;
  if ($[4] !== errors) {
    $[4] = errors;
  } else {
  }
  let isValid;
  if ($[5] !== useMemo || $[6] !== form || $[7] !== errors || $[8] !== isValid) {
    const t188 = useMemo;
    const t189 = () => {
      let errs;
      const t2 = {};
      errs = t2;
      const t5 = form;
      const t6 = t5.email;
      const t7 = "@";
      const t8 = t6.includes(t7);
      const t9 = !t8;
      const t10 = "Invalid email address";
      const t12 = errs;
      t12.email = t10;
      const t15 = form;
      const t16 = t15.password;
      const t17 = t16.length;
      const t18 = 8;
      const t19 = t17 < t18;
      const t20 = "Password must be at least 8 characters";
      const t22 = errs;
      t22.password = t20;
      const t25 = form;
      const t26 = t25.password;
      const t28 = form;
      const t29 = t28.confirmPassword;
      const t30 = t26 !== t29;
      const t31 = "Passwords do not match";
      const t33 = errs;
      t33.confirmPassword = t31;
      const t36 = errs;
      return t36;
      const t37 = undefined;
      return t37;
    };
    const t190 = form;
    const t191 = [t190];
    const t192 = t188(t189, t191);
    errors = t192;
    $[5] = useMemo;
    $[6] = form;
    $[7] = errors;
    $[8] = isValid;
  } else {
  }
  let handleChange;
  if ($[9] !== useMemo || $[10] !== errors || $[11] !== isValid || $[12] !== handleChange) {
    const t195 = useMemo;
    const t196 = () => {
      const t0 = Object;
      const t2 = errors;
      const t3 = t0.keys(t2);
      const t4 = t3.length;
      const t5 = 0;
      const t6 = t4 === t5;
      return t6;
    };
    const t197 = errors;
    const t198 = [t197];
    const t199 = t195(t196, t198);
    isValid = t199;
    $[9] = useMemo;
    $[10] = errors;
    $[11] = isValid;
    $[12] = handleChange;
  } else {
  }
  const t202 = useCallback;
  const t203 = (field) => {
    const t1 = (e) => {
      const t2 = setForm;
      const t3 = (prev) => {
        const t2 = prev;
        const t4 = e;
        const t5 = t4.target;
        const t6 = t5.value;
        const t8 = field;
        const t9 = { ...t2, [t8]: t6 };
        return t9;
      };
      const t4 = t2(t3);
      const t5 = undefined;
      return t5;
    };
    return t1;
  };
  const t204 = [];
  const t205 = t202(t203, t204);
  handleChange = t205;
  let handleSubmit;
  if ($[13] !== handleSubmit) {
    $[13] = handleSubmit;
  } else {
  }
  if ($[14] !== useCallback || $[15] !== isValid || $[16] !== form || $[17] !== handleSubmit) {
    const t208 = useCallback;
    const t209 = () => {
      const t1 = setSubmitted;
      const t2 = true;
      const t3 = t1(t2);
      const t5 = isValid;
      const t6 = console;
      const t7 = "Form submitted:";
      const t9 = form;
      const t10 = t6.log(t7, t9);
      const t11 = undefined;
      return t11;
    };
    const t210 = isValid;
    const t211 = form;
    const t212 = [t210, t211];
    const t213 = t208(t209, t212);
    handleSubmit = t213;
    $[14] = useCallback;
    $[15] = isValid;
    $[16] = form;
    $[17] = handleSubmit;
  } else {
  }
  const t215 = "div";
  const t216 = "h2";
  const t217 = "Sign Up";
  const t218 = _jsx(t216, { children: t217 });
  const t219 = "div";
  const t220 = "input";
  const t221 = "email";
  const t222 = form;
  const t223 = t222.email;
  const t224 = handleChange;
  const t225 = "email";
  const t226 = t224(t225);
  const t227 = "Email";
  const t228 = _jsx(t220, { type: t221, value: t223, onChange: t226, placeholder: t227 });
}

