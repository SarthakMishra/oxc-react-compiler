import { c as _c } from "react/compiler-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(30);
  let t5;
  let t11;
  let errors;
  let t132;
  let t28;
  let t30;
  let t133;
  let t134;
  let t37;
  let t38;
  let t135;
  let t136;
  let t44;
  let t47;
  let t137;
  let t50;
  let t53;
  let t54;
  let t63;
  let t64;
  let t65;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t5 = { email: "", password: "", confirmPassword: "" };
    $[0] = t5;
  } else {
    t5 = $[0];
  }
  let t6 = useState(t5);
  let form;
  let setForm;
  ([form, setForm] = t6);
  if ($[1] !== t6) {
    $[1] = t6;
    $[2] = form;
    $[3] = setForm;
  } else {
    form = $[2];
    setForm = $[3];
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    t11 = false;
    $[4] = t11;
  } else {
    t11 = $[4];
  }
  let submitted;
  let setSubmitted;
  ([submitted, setSubmitted] = useState(t11));
  let t20 = () => {
    let errs;
    errs = {};
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
  let t23 = useMemo(t20, [form]);
  if ($[5] !== t23) {
    errors = t23;
    t28 = () => {
      return Object.keys(errors).length === 0;
    };
    t30 = [errors];
    $[5] = t23;
    $[6] = errors;
    $[7] = t132;
    $[8] = t28;
    $[9] = t30;
  } else {
    errors = $[6];
    t132 = $[7];
    t28 = $[8];
    t30 = $[9];
  }
  let isValid = t132;
  let t31 = useMemo(t28, t30);
  if ($[10] !== t31) {
    t133 = t31;
    t37 = (field) => {
      let t1 = (e) => {
        let t3 = (prev) => {
          return { ...prev, [field]: e.target.value };
        };
        let t4 = setForm(t3);
        return undefined;
      };
      return t1;
    };
    t38 = [];
    $[10] = t31;
    $[11] = t133;
    $[12] = t134;
    $[13] = t37;
    $[14] = t38;
  } else {
    t133 = $[11];
    t134 = $[12];
    t37 = $[13];
    t38 = $[14];
  }
  isValid = t133;
  let handleChange = t134;
  let t39 = useCallback(t37, t38);
  if ($[15] !== t39 || $[16] !== form) {
    t135 = t39;
    t44 = () => {
      let t3 = setSubmitted(true);
      if (isValid) {
        let t10 = console.log("Form submitted:", form);
      }
      return undefined;
    };
    t47 = [isValid, form];
    $[15] = t39;
    $[16] = form;
    $[17] = t135;
    $[18] = t136;
    $[19] = t44;
    $[20] = t47;
  } else {
    t135 = $[17];
    t136 = $[18];
    t44 = $[19];
    t47 = $[20];
  }
  handleChange = t135;
  let handleSubmit = t136;
  let t48 = useCallback(t44, t47);
  if ($[21] !== t48 || $[22] !== form) {
    t137 = t48;
    t50 = "div";
    t53 = (
      <h2>
        Sign Up
      </h2>
    );
    t54 = "div";
    t63 = <input type="email" value={form.email} onChange={handleChange("email")} placeholder="Email" />;
    $[21] = t48;
    $[22] = form;
    $[23] = t137;
    $[24] = t50;
    $[25] = t53;
    $[26] = t54;
    $[27] = t63;
    $[28] = t64;
  } else {
    t137 = $[23];
    t50 = $[24];
    t53 = $[25];
    t54 = $[26];
    t63 = $[27];
    t64 = $[28];
  }
  handleSubmit = t137;
  if ($[29] === Symbol.for("react.memo_cache_sentinel")) {
    $[29] = t65;
  } else {
    t65 = $[29];
  }
  t65 = submitted;
  t65 = errors.email;
  t64 = t65;
  t64 = <span className="error">{errors.email}</span>;
  let t85;
  let t86;
  t86 = submitted;
  t86 = errors.password;
  t85 = t86;
  t85 = <span className="error">{errors.password}</span>;
  let t106;
  let t107;
  t107 = submitted;
  t107 = errors.confirmPassword;
  t106 = t107;
  t106 = <span className="error">{errors.confirmPassword}</span>;
  let t119;
  t119 = submitted;
  t119 = !isValid;
  let t123;
  t123 = submitted;
  t123 = !isValid;
  let t127;
  if (t123) {
    t127 = "Fix errors";
  } else {
    t127 = "Submit";
  }
  return <div>{t53}<div>{t63}{t64}</div><div><input type="password" value={form.password} onChange={handleChange("password")} placeholder="Password" />{t85}</div><div><input type="password" value={form.confirmPassword} onChange={handleChange("confirmPassword")} placeholder="Confirm Password" />{t106}</div><button onClick={handleSubmit} disabled={t119}>{t127}</button></div>;
}

