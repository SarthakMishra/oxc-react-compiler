import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(53);
  const t199 = useState;
  const t200 = "";
  const t201 = "";
  const t202 = "";
  const t203 = { email: t200, password: t201, confirmPassword: t202 };
  const t204 = t199(t203);
  let form;
  let setForm;
  if ($[0] !== form || $[1] !== setForm) {
    $[0] = form;
    $[1] = setForm;
  } else {
  }
  ([form, setForm] = t204);
  const t208 = useState;
  const t209 = false;
  const t210 = t208(t209);
  let submitted;
  let setSubmitted;
  if ($[2] !== submitted || $[3] !== setSubmitted) {
    $[2] = submitted;
    $[3] = setSubmitted;
  } else {
  }
  ([submitted, setSubmitted] = t210);
  let errors;
  if ($[4] !== errors) {
    $[4] = errors;
  } else {
  }
  let isValid;
  if ($[5] !== useMemo || $[6] !== form || $[7] !== errors || $[8] !== isValid) {
    const t215 = useMemo;
    const t216 = () => {
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
    const t217 = form;
    const t218 = [t217];
    const t219 = t215(t216, t218);
    errors = t219;
    $[5] = useMemo;
    $[6] = form;
    $[7] = errors;
    $[8] = isValid;
  } else {
  }
  let handleChange;
  if ($[9] !== useMemo || $[10] !== errors || $[11] !== isValid || $[12] !== handleChange) {
    const t222 = useMemo;
    const t223 = () => {
      const t0 = Object;
      const t2 = errors;
      const t3 = t0.keys(t2);
      const t4 = t3.length;
      const t5 = 0;
      const t6 = t4 === t5;
      return t6;
    };
    const t224 = errors;
    const t225 = [t224];
    const t226 = t222(t223, t225);
    isValid = t226;
    $[9] = useMemo;
    $[10] = errors;
    $[11] = isValid;
    $[12] = handleChange;
  } else {
  }
  const t229 = useCallback;
  const t230 = (field) => {
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
  const t231 = [];
  const t232 = t229(t230, t231);
  handleChange = t232;
  let handleSubmit;
  if ($[13] !== handleSubmit) {
    $[13] = handleSubmit;
  } else {
  }
  let t85;
  if ($[14] !== useCallback || $[15] !== isValid || $[16] !== form || $[17] !== handleSubmit || $[18] !== t85) {
    const t235 = useCallback;
    const t236 = () => {
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
    const t237 = isValid;
    const t238 = form;
    const t239 = [t237, t238];
    const t240 = t235(t236, t239);
    handleSubmit = t240;
    const t242 = "div";
    const t243 = "h2";
    const t244 = "Sign Up";
    const t245 = _jsx(t243, { children: t244 });
    const t246 = "div";
    const t247 = "input";
    const t248 = "email";
    const t249 = form;
    const t250 = t249.email;
    const t251 = handleChange;
    const t252 = "email";
    const t253 = t251(t252);
    const t254 = "Email";
    const t255 = _jsx(t247, { type: t248, value: t250, onChange: t253, placeholder: t254 });
    $[14] = useCallback;
    $[15] = isValid;
    $[16] = form;
    $[17] = handleSubmit;
    $[18] = t85;
  } else {
  }
  let t87;
  if ($[19] !== errors || $[20] !== t87 || $[21] !== submitted) {
    const t400 = submitted;
    t87 = t400;
    $[19] = errors;
    $[20] = t87;
    $[21] = submitted;
  } else {
  }
  const t412 = errors;
  const t413 = t412.email;
  t87 = t413;
  t85 = t87;
  if ($[22] !== errors || $[23] !== t85) {
    const t406 = "span";
    const t407 = "error";
    const t408 = errors;
    const t409 = t408.email;
    const t410 = _jsx(t406, { className: t407, children: t409 });
    t85 = t410;
    $[22] = errors;
    $[23] = t85;
  } else {
  }
  let t369;
  if ($[24] !== t117 || $[25] !== form || $[26] !== handleChange || $[27] !== t149 || $[28] !== handleSubmit || $[29] !== t172 || $[30] !== t190 || $[31] !== t85 || $[32] !== form || $[33] !== handleChange || $[34] !== form || $[35] !== handleChange) {
    const t276 = _jsxs(t246, { children: [t255, t85] });
    const t277 = "div";
    const t278 = "input";
    const t279 = "password";
    const t280 = form;
    const t281 = t280.password;
    const t282 = handleChange;
    const t283 = "password";
    const t284 = t282(t283);
    const t285 = "Password";
    const t286 = _jsx(t278, { type: t279, value: t281, onChange: t284, placeholder: t285 });
    $[36] = t369;
    $[24] = t117;
    $[25] = form;
    $[26] = handleChange;
    $[27] = t149;
    $[28] = handleSubmit;
    $[29] = t172;
    $[30] = t190;
    $[31] = t85;
    $[32] = form;
    $[33] = handleChange;
    $[34] = form;
    $[35] = handleChange;
  } else {
    t369 = $[36];
  }
  let t117;
  let t119;
  if ($[37] !== errors || $[38] !== t119 || $[39] !== submitted) {
    const t385 = submitted;
    t119 = t385;
    $[37] = errors;
    $[38] = t119;
    $[39] = submitted;
  } else {
  }
  const t387 = errors;
  const t388 = t387.password;
  t119 = t388;
  t117 = t119;
  if ($[40] !== errors || $[41] !== t117) {
    const t394 = "span";
    const t395 = "error";
    const t396 = errors;
    const t397 = t396.password;
    const t398 = _jsx(t394, { className: t395, children: t397 });
    t117 = t398;
    $[40] = errors;
    $[41] = t117;
  } else {
  }
  const t307 = _jsxs(t277, { children: [t286, t117] });
  const t308 = "div";
  const t309 = "input";
  const t310 = "password";
  const t311 = form;
  const t312 = t311.confirmPassword;
  const t313 = handleChange;
  const t314 = "confirmPassword";
  const t315 = t313(t314);
  const t316 = "Confirm Password";
  const t317 = _jsx(t309, { type: t310, value: t312, onChange: t315, placeholder: t316 });
  let t149;
  let t151;
  if ($[42] !== errors || $[43] !== t151 || $[44] !== submitted) {
    const t370 = submitted;
    t151 = t370;
    $[42] = errors;
    $[43] = t151;
    $[44] = submitted;
  } else {
  }
  const t372 = errors;
  const t373 = t372.confirmPassword;
  t151 = t373;
  t149 = t151;
  if ($[45] !== submitted || $[46] !== t172 || $[47] !== isValid || $[48] !== errors || $[49] !== t149) {
    const t379 = "span";
    const t380 = "error";
    const t381 = errors;
    const t382 = t381.confirmPassword;
    const t383 = _jsx(t379, { className: t380, children: t382 });
    t149 = t383;
    $[45] = submitted;
    $[46] = t172;
    $[47] = isValid;
    $[48] = errors;
    $[49] = t149;
  } else {
  }
  const t338 = _jsxs(t308, { children: [t317, t149] });
  const t339 = "button";
  const t340 = handleSubmit;
  let t172;
  const t342 = submitted;
  t172 = t342;
  const t344 = isValid;
  const t345 = !t344;
  t172 = t345;
  let t181;
  if ($[50] !== isValid || $[51] !== t181 || $[52] !== submitted) {
    const t351 = submitted;
    t181 = t351;
    $[50] = isValid;
    $[51] = t181;
    $[52] = submitted;
  } else {
  }
  const t353 = isValid;
  const t354 = !t353;
  t181 = t354;
  let t190;
  if (t181) {
    const t360 = "Fix errors";
    t190 = t360;
  } else {
    const t362 = "Submit";
    t190 = t362;
  }
  const t368 = _jsx(t339, { onClick: t340, disabled: t172, children: t190 });
  t369 = _jsxs(t242, { children: [t245, t276, t307, t338, t368] });
  return t369;
}

