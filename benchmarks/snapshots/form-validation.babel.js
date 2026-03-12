import { c as _c } from "react/compiler-runtime";
import { useState, useMemo, useCallback } from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function FormValidation() {
  const $ = _c(48);
  let t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t0 = {
      email: "",
      password: "",
      confirmPassword: ""
    };
    $[0] = t0;
  } else {
    t0 = $[0];
  }
  const [form, setForm] = useState(t0);
  const [submitted, setSubmitted] = useState(false);
  let errs;
  if ($[1] !== form.confirmPassword || $[2] !== form.email || $[3] !== form.password) {
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
    $[1] = form.confirmPassword;
    $[2] = form.email;
    $[3] = form.password;
    $[4] = errs;
  } else {
    errs = $[4];
  }
  const errors = errs;
  let t1;
  if ($[5] !== errors) {
    t1 = Object.keys(errors);
    $[5] = errors;
    $[6] = t1;
  } else {
    t1 = $[6];
  }
  const isValid = t1.length === 0;
  let t2;
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    t2 = field => e => {
      setForm(prev => ({
        ...prev,
        [field]: e.target.value
      }));
    };
    $[7] = t2;
  } else {
    t2 = $[7];
  }
  const handleChange = t2;
  let t3;
  if ($[8] !== form || $[9] !== isValid) {
    t3 = () => {
      setSubmitted(true);
      if (isValid) {
        console.log("Form submitted:", form);
      }
    };
    $[8] = form;
    $[9] = isValid;
    $[10] = t3;
  } else {
    t3 = $[10];
  }
  const handleSubmit = t3;
  let t4;
  if ($[11] === Symbol.for("react.memo_cache_sentinel")) {
    t4 = /*#__PURE__*/_jsx("h2", {
      children: "Sign Up"
    });
    $[11] = t4;
  } else {
    t4 = $[11];
  }
  const t5 = form.email;
  let t6;
  if ($[12] === Symbol.for("react.memo_cache_sentinel")) {
    t6 = handleChange("email");
    $[12] = t6;
  } else {
    t6 = $[12];
  }
  let t7;
  if ($[13] !== form.email) {
    t7 = /*#__PURE__*/_jsx("input", {
      type: "email",
      value: t5,
      onChange: t6,
      placeholder: "Email"
    });
    $[13] = form.email;
    $[14] = t7;
  } else {
    t7 = $[14];
  }
  let t8;
  if ($[15] !== errors || $[16] !== submitted) {
    t8 = submitted && errors.email && /*#__PURE__*/_jsx("span", {
      className: "error",
      children: errors.email
    });
    $[15] = errors;
    $[16] = submitted;
    $[17] = t8;
  } else {
    t8 = $[17];
  }
  let t9;
  if ($[18] !== t7 || $[19] !== t8) {
    t9 = /*#__PURE__*/_jsxs("div", {
      children: [t7, t8]
    });
    $[18] = t7;
    $[19] = t8;
    $[20] = t9;
  } else {
    t9 = $[20];
  }
  const t10 = form.password;
  let t11;
  if ($[21] === Symbol.for("react.memo_cache_sentinel")) {
    t11 = handleChange("password");
    $[21] = t11;
  } else {
    t11 = $[21];
  }
  let t12;
  if ($[22] !== form.password) {
    t12 = /*#__PURE__*/_jsx("input", {
      type: "password",
      value: t10,
      onChange: t11,
      placeholder: "Password"
    });
    $[22] = form.password;
    $[23] = t12;
  } else {
    t12 = $[23];
  }
  let t13;
  if ($[24] !== errors || $[25] !== submitted) {
    t13 = submitted && errors.password && /*#__PURE__*/_jsx("span", {
      className: "error",
      children: errors.password
    });
    $[24] = errors;
    $[25] = submitted;
    $[26] = t13;
  } else {
    t13 = $[26];
  }
  let t14;
  if ($[27] !== t12 || $[28] !== t13) {
    t14 = /*#__PURE__*/_jsxs("div", {
      children: [t12, t13]
    });
    $[27] = t12;
    $[28] = t13;
    $[29] = t14;
  } else {
    t14 = $[29];
  }
  const t15 = form.confirmPassword;
  let t16;
  if ($[30] === Symbol.for("react.memo_cache_sentinel")) {
    t16 = handleChange("confirmPassword");
    $[30] = t16;
  } else {
    t16 = $[30];
  }
  let t17;
  if ($[31] !== form.confirmPassword) {
    t17 = /*#__PURE__*/_jsx("input", {
      type: "password",
      value: t15,
      onChange: t16,
      placeholder: "Confirm Password"
    });
    $[31] = form.confirmPassword;
    $[32] = t17;
  } else {
    t17 = $[32];
  }
  let t18;
  if ($[33] !== errors || $[34] !== submitted) {
    t18 = submitted && errors.confirmPassword && /*#__PURE__*/_jsx("span", {
      className: "error",
      children: errors.confirmPassword
    });
    $[33] = errors;
    $[34] = submitted;
    $[35] = t18;
  } else {
    t18 = $[35];
  }
  let t19;
  if ($[36] !== t17 || $[37] !== t18) {
    t19 = /*#__PURE__*/_jsxs("div", {
      children: [t17, t18]
    });
    $[36] = t17;
    $[37] = t18;
    $[38] = t19;
  } else {
    t19 = $[38];
  }
  const t20 = submitted && !isValid;
  const t21 = submitted && !isValid ? "Fix errors" : "Submit";
  let t22;
  if ($[39] !== handleSubmit || $[40] !== t20 || $[41] !== t21) {
    t22 = /*#__PURE__*/_jsx("button", {
      onClick: handleSubmit,
      disabled: t20,
      children: t21
    });
    $[39] = handleSubmit;
    $[40] = t20;
    $[41] = t21;
    $[42] = t22;
  } else {
    t22 = $[42];
  }
  let t23;
  if ($[43] !== t14 || $[44] !== t19 || $[45] !== t22 || $[46] !== t9) {
    t23 = /*#__PURE__*/_jsxs("div", {
      children: [t4, t9, t14, t19, t22]
    });
    $[43] = t14;
    $[44] = t19;
    $[45] = t22;
    $[46] = t9;
    $[47] = t23;
  } else {
    t23 = $[47];
  }
  return t23;
}