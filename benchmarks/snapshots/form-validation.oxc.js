import { c as _c } from "react/compiler-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(35);
  let form;
  let setForm;
  if ($[0] !== useState) {
    $[0] = useState;
  }
  let isValid;
  if ($[1] !== form || $[2] !== useMemo) {
    const t216 = () => {
      const errs = {};
      if (!form.email.includes("@")) {
        errs.email = "Invalid email address";
      }
      if (form.password.length < 8) {
        errs.password = "Password must be at least 8 characters";
      }
      if (form.password !== form.confirmPassword) {
        errs.confirmPassword = "Passwords do not match";
      }
      return errs;
    };
    const errors = useMemo(t216, [form]);
    $[1] = form;
    $[2] = useMemo;
  }
  let handleChange;
  if ($[3] !== errors || $[4] !== useMemo) {
    const t223 = () => {
      return Object.keys(errors).length === 0;
    };
    isValid = useMemo(t223, [errors]);
    $[3] = errors;
    $[4] = useMemo;
  }
  let handleSubmit;
  if ($[5] !== useCallback) {
    const t230 = (field) => {
      const t1 = (e) => {
        const t3 = (prev) => {
          return { ...prev, [field]: e.target.value };
        };
        const t4 = setForm(t3);
        return undefined;
      };
      return t1;
    };
    handleChange = useCallback(t230, []);
    $[5] = useCallback;
  }
  let t85;
  if ($[6] !== form || $[7] !== isValid || $[8] !== useCallback) {
    const t236 = () => {
      const t3 = setSubmitted(true);
      if (isValid) {
        const t10 = console.log("Form submitted:", form);
      }
      return undefined;
    };
    handleSubmit = useCallback(t236, [isValid, form]);
    $[6] = form;
    $[7] = isValid;
    $[8] = useCallback;
  }
  if ($[9] !== errors || $[10] !== submitted) {
    t87 = submitted;
    $[9] = errors;
    $[10] = submitted;
  }
  t87 = errors.email;
  t85 = t87;
  if ($[11] !== errors) {
    t85 = <span className="error">{errors.email}</span>;
    $[11] = errors;
  }
  let t369;
  if ($[12] !== t117 || $[13] !== t149 || $[14] !== t172 || $[15] !== t190 || $[16] !== t85 || $[17] !== form || $[18] !== form || $[19] !== form || $[20] !== handleChange || $[21] !== handleChange || $[22] !== handleChange || $[23] !== handleSubmit) {
    $[12] = t117;
    $[13] = t149;
    $[14] = t172;
    $[15] = t190;
    $[16] = t85;
    $[17] = form;
    $[18] = form;
    $[19] = form;
    $[20] = handleChange;
    $[21] = handleChange;
    $[22] = handleChange;
    $[23] = handleSubmit;
    $[24] = t369;
  } else {
    t369 = $[24];
  }
  if ($[25] !== errors || $[26] !== submitted) {
    t119 = submitted;
    $[25] = errors;
    $[26] = submitted;
  }
  t119 = errors.password;
  t117 = t119;
  if ($[27] !== errors) {
    t117 = <span className="error">{errors.password}</span>;
    $[27] = errors;
  }
  if ($[28] !== errors || $[29] !== submitted) {
    t151 = submitted;
    $[28] = errors;
    $[29] = submitted;
  }
  t151 = errors.confirmPassword;
  t149 = t151;
  if ($[30] !== errors || $[31] !== isValid || $[32] !== submitted) {
    t149 = <span className="error">{errors.confirmPassword}</span>;
    $[30] = errors;
    $[31] = isValid;
    $[32] = submitted;
  }
  t172 = submitted;
  t172 = !isValid;
  if ($[33] !== isValid || $[34] !== submitted) {
    t181 = submitted;
    $[33] = isValid;
    $[34] = submitted;
  }
  t181 = !isValid;
  if (t181) {
    t190 = "Fix errors";
  } else {
    t190 = "Submit";
  }
  return <t242>{t245}{t276}<t277>{t286}{t117}</t277><div><input type="password" value={form.confirmPassword} onChange={handleChange("confirmPassword")} placeholder="Confirm Password" />{t149}</div><button onClick={handleSubmit} disabled={t172}>{t190}</button></t242>;
}

