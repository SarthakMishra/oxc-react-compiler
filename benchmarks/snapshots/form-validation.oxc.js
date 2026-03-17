import { c as _c } from "react/compiler-runtime";
import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const $ = _c(35);
  let t5;
  if ($[0] !== t6) {
    $[0] = t6;
    $[1] = t5;
  } else {
    t5 = $[1];
  }
  let t11;
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    $[2] = t11;
  } else {
    t11 = $[2];
  }
  let errors;
  const t20 = () => {
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
  const t23 = useMemo(t20, [form]);
  let t132;
  let t28;
  let t30;
  if ($[3] !== t23) {
    errors = t23;
    t28 = () => {
      return Object.keys(errors).length === 0;
    };
    t30 = [errors];
    $[3] = t23;
    $[4] = errors;
    $[5] = t132;
    $[6] = t28;
    $[7] = t30;
  } else {
    errors = $[4];
    t132 = $[5];
    t28 = $[6];
    t30 = $[7];
  }
  const isValid = t132;
  const t31 = useMemo(t28, t30);
  let t134;
  let t133;
  let t37;
  let t38;
  if ($[8] !== t31) {
    t133 = t31;
    t37 = (field) => {
      const t1 = (e) => {
        const t3 = (prev) => {
          return { ...prev, [field]: e.target.value };
        };
        const t4 = setForm(t3);
        return undefined;
      };
      return t1;
    };
    t38 = [];
    $[8] = t31;
    $[9] = t133;
    $[10] = t134;
    $[11] = t37;
    $[12] = t38;
  } else {
    t133 = $[9];
    t134 = $[10];
    t37 = $[11];
    t38 = $[12];
  }
  const isValid = t133;
  const handleChange = t134;
  const t39 = useCallback(t37, t38);
  let t136;
  let t135;
  let t44;
  let t47;
  if ($[13] !== t39 || $[14] !== form.email) {
    t135 = t39;
    t44 = () => {
      const t3 = setSubmitted(true);
      if (isValid) {
        const t10 = console.log("Form submitted:", form);
      }
      return undefined;
    };
    t47 = [isValid, form];
    $[13] = t39;
    $[14] = form.email;
    $[15] = t135;
    $[16] = t136;
    $[17] = t44;
    $[18] = t47;
  } else {
    t135 = $[15];
    t136 = $[16];
    t44 = $[17];
    t47 = $[18];
  }
  const handleChange = t135;
  const handleSubmit = t136;
  const t48 = useCallback(t44, t47);
  let t64;
  let t137;
  let t50;
  let t53;
  let t54;
  let t63;
  if ($[19] !== t48 || $[20] !== form.email) {
    t137 = t48;
    t50 = "div";
    t53 = (
      <h2>
        Sign Up
      </h2>
    );
    t54 = "div";
    t63 = <input type="email" value={form.email} onChange={handleChange("email")} placeholder="Email" />;
    $[19] = t48;
    $[20] = form.email;
    $[21] = t137;
    $[22] = t50;
    $[23] = t53;
    $[24] = t54;
    $[25] = t63;
    $[26] = t64;
  } else {
    t137 = $[21];
    t50 = $[22];
    t53 = $[23];
    t54 = $[24];
    t63 = $[25];
    t64 = $[26];
  }
  const handleSubmit = t137;
  let t131;
  let t138;
  let t20;
  let t22;
  let t139;
  if ($[27] !== t63 || $[28] !== form.email) {
    $[27] = t63;
    $[28] = form.email;
    $[29] = t64;
    $[30] = t131;
    $[31] = t138;
    $[32] = t20;
    $[33] = t22;
    $[34] = t139;
  } else {
    t64 = $[29];
    t131 = $[30];
    t138 = $[31];
    t20 = $[32];
    t22 = $[33];
    t139 = $[34];
  }
  errors = t138;
  const form = t139;
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
  return <t50>{t53}<t54>{t63}{t64}</t54><div><input type="password" value={form.password} onChange={handleChange("password")} placeholder="Password" />{t85}</div><div><input type="password" value={form.confirmPassword} onChange={handleChange("confirmPassword")} placeholder="Confirm Password" />{t106}</div><button onClick={handleSubmit} disabled={t119}>{t127}</button></t50>;
}

