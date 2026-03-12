import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(39);
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
      if (t9) {
        const t10 = "Invalid email address";
        const t12 = errs;
        t12.email = t10;
      } else {
      }
      const t15 = form;
      const t16 = t15.password;
      const t17 = t16.length;
      const t18 = 8;
      const t19 = t17 < t18;
      if (t19) {
        const t20 = "Password must be at least 8 characters";
        const t22 = errs;
        t22.password = t20;
      } else {
      }
      const t25 = form;
      const t26 = t25.password;
      const t28 = form;
      const t29 = t28.confirmPassword;
      const t30 = t26 !== t29;
      if (t30) {
        const t31 = "Passwords do not match";
        const t33 = errs;
        t33.confirmPassword = t31;
      } else {
      }
      const t36 = errs;
      return t36;
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
      if (t5) {
        const t6 = console;
        const t7 = "Form submitted:";
        const t9 = form;
        const t10 = t6.log(t7, t9);
      } else {
      }
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
  if ($[18] !== errors) {
    const t331 = errors;
    $[18] = errors;
  } else {
  }
  const t326 = "span";
  const t327 = "error";
  if ($[19] !== errors) {
    const t328 = errors;
    const t329 = t328.email;
    $[19] = errors;
  } else {
  }
  let t302;
  if ($[20] !== t123 || $[21] !== form || $[22] !== handleChange || $[23] !== t149 || $[24] !== handleSubmit || $[25] !== t159 || $[26] !== t168 || $[27] !== form || $[28] !== handleChange || $[29] !== t97 || $[30] !== form || $[31] !== handleChange) {
    const t240 = _jsxs(t219, { children: [t228, t97] });
    const t241 = "div";
    const t242 = "input";
    const t243 = "password";
    const t244 = form;
    const t245 = t244.password;
    const t246 = handleChange;
    const t247 = "password";
    const t248 = t246(t247);
    const t249 = "Password";
    const t250 = _jsx(t242, { type: t243, value: t245, onChange: t248, placeholder: t249 });
    $[32] = t302;
    $[20] = t123;
    $[21] = form;
    $[22] = handleChange;
    $[23] = t149;
    $[24] = handleSubmit;
    $[25] = t159;
    $[26] = t168;
    $[27] = form;
    $[28] = handleChange;
    $[29] = t97;
    $[30] = form;
    $[31] = handleChange;
  } else {
    t302 = $[32];
  }
  if ($[33] !== errors) {
    const t314 = errors;
    $[33] = errors;
  } else {
  }
  const t318 = "span";
  const t319 = "error";
  if ($[34] !== errors) {
    const t320 = errors;
    const t321 = t320.password;
    $[34] = errors;
  } else {
  }
  const t262 = _jsxs(t241, { children: [t250, t123] });
  const t263 = "div";
  const t264 = "input";
  const t265 = "password";
  const t266 = form;
  const t267 = t266.confirmPassword;
  const t268 = handleChange;
  const t269 = "confirmPassword";
  const t270 = t268(t269);
  const t271 = "Confirm Password";
  const t272 = _jsx(t264, { type: t265, value: t267, onChange: t270, placeholder: t271 });
  if ($[35] !== errors) {
    const t304 = errors;
    $[35] = errors;
  } else {
  }
  const t308 = "span";
  const t309 = "error";
  if ($[36] !== errors) {
    const t310 = errors;
    const t311 = t310.confirmPassword;
    $[36] = errors;
  } else {
  }
  const t284 = _jsxs(t263, { children: [t272, t149] });
  const t285 = "button";
  const t286 = handleSubmit;
  if ($[37] !== isValid) {
    const t288 = isValid;
    $[37] = isValid;
  } else {
  }
  if ($[38] !== isValid) {
    const t293 = isValid;
    $[38] = isValid;
  } else {
  }
  if (t165) {
  } else {
  }
  const t301 = _jsx(t285, { onClick: t286, disabled: t159, children: t168 });
  t302 = _jsxs(t215, { children: [t218, t240, t262, t284, t301] });
  return t302;
}

