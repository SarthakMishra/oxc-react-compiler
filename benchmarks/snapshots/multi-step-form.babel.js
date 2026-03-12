import { c as _c } from "react/compiler-runtime";
// L tier - Inspired by cal.com event type creation wizard
import { useState, useMemo, useCallback, useReducer } from 'react';
import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
function formReducer(state, action) {
  switch (action.type) {
    case 'SET_FIELD':
      return {
        ...state,
        data: {
          ...state.data,
          [action.name]: action.value
        },
        touched: new Set([...state.touched, action.name])
      };
    case 'SET_ERRORS':
      return {
        ...state,
        errors: action.errors
      };
    case 'CLEAR_ERROR':
      {
        const errors = {
          ...state.errors
        };
        delete errors[action.name];
        return {
          ...state,
          errors
        };
      }
    case 'RESET':
      return {
        data: {},
        errors: {},
        touched: new Set()
      };
    default:
      return state;
  }
}
export function MultiStepForm(t0) {
  const $ = _c(92);
  const {
    steps,
    onSubmit,
    onCancel
  } = t0;
  const [currentStep, setCurrentStep] = useState(0);
  let t1;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t1 = {
      data: {},
      errors: {},
      touched: new Set()
    };
    $[0] = t1;
  } else {
    t1 = $[0];
  }
  const [formState, dispatch] = useReducer(formReducer, t1);
  const [submitting, setSubmitting] = useState(false);
  const step = steps[currentStep];
  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === steps.length - 1;
  let t2;
  if ($[1] !== currentStep || $[2] !== steps.length) {
    t2 = Math.round((currentStep + 1) / steps.length * 100);
    $[1] = currentStep;
    $[2] = steps.length;
    $[3] = t2;
  } else {
    t2 = $[3];
  }
  const progress = t2;
  let completed = 0;
  let total = 0;
  for (const s of steps) {
    for (const field of s.fields) {
      total++;
      if (formState.data[field.name]) {
        completed++;
      }
    }
  }
  let t3;
  if ($[4] !== completed || $[5] !== total) {
    t3 = {
      completed,
      total
    };
    $[4] = completed;
    $[5] = total;
    $[6] = t3;
  } else {
    t3 = $[6];
  }
  const completedFields = t3;
  let t4;
  if ($[7] !== formState.data || $[8] !== steps) {
    t4 = stepIndex => {
      const stepToValidate = steps[stepIndex];
      const errors = {};
      let valid = true;
      for (const field_0 of stepToValidate.fields) {
        const value = formState.data[field_0.name] || "";
        if (field_0.required && !value.trim()) {
          errors[field_0.name] = `${field_0.label} is required`;
          valid = false;
        } else {
          if (field_0.validate) {
            const error = field_0.validate(value);
            if (error) {
              errors[field_0.name] = error;
              valid = false;
            }
          }
        }
      }
      dispatch({
        type: "SET_ERRORS",
        errors
      });
      return valid;
    };
    $[7] = formState.data;
    $[8] = steps;
    $[9] = t4;
  } else {
    t4 = $[9];
  }
  const validateStep = t4;
  let t5;
  if ($[10] !== currentStep || $[11] !== steps.length || $[12] !== validateStep) {
    t5 = () => {
      if (validateStep(currentStep)) {
        setCurrentStep(s_0 => Math.min(s_0 + 1, steps.length - 1));
      }
    };
    $[10] = currentStep;
    $[11] = steps.length;
    $[12] = validateStep;
    $[13] = t5;
  } else {
    t5 = $[13];
  }
  const handleNext = t5;
  let t6;
  if ($[14] === Symbol.for("react.memo_cache_sentinel")) {
    t6 = () => {
      setCurrentStep(_temp);
    };
    $[14] = t6;
  } else {
    t6 = $[14];
  }
  const handlePrev = t6;
  let t7;
  if ($[15] !== currentStep || $[16] !== formState.data || $[17] !== onSubmit || $[18] !== validateStep) {
    t7 = async () => {
      if (!validateStep(currentStep)) {
        return;
      }
      setSubmitting(true);
      onSubmit(formState.data);
      setSubmitting(false);
    };
    $[15] = currentStep;
    $[16] = formState.data;
    $[17] = onSubmit;
    $[18] = validateStep;
    $[19] = t7;
  } else {
    t7 = $[19];
  }
  const handleSubmit = t7;
  let t8;
  if ($[20] === Symbol.for("react.memo_cache_sentinel")) {
    t8 = (name, value_0) => {
      dispatch({
        type: "SET_FIELD",
        name,
        value: value_0
      });
      dispatch({
        type: "CLEAR_ERROR",
        name
      });
    };
    $[20] = t8;
  } else {
    t8 = $[20];
  }
  const handleFieldChange = t8;
  let t9;
  if ($[21] !== formState.errors || $[22] !== step.fields) {
    let t10;
    if ($[24] !== formState.errors) {
      t10 = f => formState.errors[f.name];
      $[24] = formState.errors;
      $[25] = t10;
    } else {
      t10 = $[25];
    }
    t9 = step.fields.filter(t10);
    $[21] = formState.errors;
    $[22] = step.fields;
    $[23] = t9;
  } else {
    t9 = $[23];
  }
  const stepErrors = t9.length;
  const t10 = currentStep + 1;
  let t11;
  if ($[26] !== steps.length || $[27] !== t10) {
    t11 = /*#__PURE__*/_jsxs("span", {
      children: ["Step ", t10, " of ", steps.length]
    });
    $[26] = steps.length;
    $[27] = t10;
    $[28] = t11;
  } else {
    t11 = $[28];
  }
  let t12;
  if ($[29] !== completedFields.completed || $[30] !== completedFields.total) {
    t12 = /*#__PURE__*/_jsxs("span", {
      children: [completedFields.completed, "/", completedFields.total, " fields completed"]
    });
    $[29] = completedFields.completed;
    $[30] = completedFields.total;
    $[31] = t12;
  } else {
    t12 = $[31];
  }
  let t13;
  if ($[32] !== t11 || $[33] !== t12) {
    t13 = /*#__PURE__*/_jsxs("div", {
      className: "flex justify-between text-sm text-gray-500 mb-1",
      children: [t11, t12]
    });
    $[32] = t11;
    $[33] = t12;
    $[34] = t13;
  } else {
    t13 = $[34];
  }
  const t14 = `${progress}%`;
  let t15;
  if ($[35] !== t14) {
    t15 = /*#__PURE__*/_jsx("div", {
      className: "w-full bg-gray-200 rounded h-2",
      children: /*#__PURE__*/_jsx("div", {
        className: "bg-blue-600 rounded h-2 transition-all",
        style: {
          width: t14
        }
      })
    });
    $[35] = t14;
    $[36] = t15;
  } else {
    t15 = $[36];
  }
  let t16;
  if ($[37] !== t13 || $[38] !== t15) {
    t16 = /*#__PURE__*/_jsxs("div", {
      className: "mb-6",
      children: [t13, t15]
    });
    $[37] = t13;
    $[38] = t15;
    $[39] = t16;
  } else {
    t16 = $[39];
  }
  let t17;
  if ($[40] !== currentStep || $[41] !== steps) {
    let t18;
    if ($[43] !== currentStep || $[44] !== steps.length) {
      t18 = (s_2, i) => /*#__PURE__*/_jsxs("div", {
        className: "flex-1 flex items-center",
        children: [/*#__PURE__*/_jsx("div", {
          className: `w-8 h-8 rounded-full flex items-center justify-center text-sm ${i < currentStep ? "bg-green-500 text-white" : i === currentStep ? "bg-blue-600 text-white" : "bg-gray-200 text-gray-500"}`,
          children: i < currentStep ? "\u2713" : i + 1
        }), /*#__PURE__*/_jsx("span", {
          className: `ml-2 text-sm ${i === currentStep ? "font-semibold" : "text-gray-400"}`,
          children: s_2.title
        }), i < steps.length - 1 && /*#__PURE__*/_jsx("div", {
          className: "flex-1 h-px bg-gray-200 mx-4"
        })]
      }, i);
      $[43] = currentStep;
      $[44] = steps.length;
      $[45] = t18;
    } else {
      t18 = $[45];
    }
    t17 = steps.map(t18);
    $[40] = currentStep;
    $[41] = steps;
    $[42] = t17;
  } else {
    t17 = $[42];
  }
  let t18;
  if ($[46] !== t17) {
    t18 = /*#__PURE__*/_jsx("div", {
      className: "flex mb-8",
      children: t17
    });
    $[46] = t17;
    $[47] = t18;
  } else {
    t18 = $[47];
  }
  let t19;
  if ($[48] !== step.title) {
    t19 = /*#__PURE__*/_jsx("h2", {
      className: "text-xl font-semibold mb-1",
      children: step.title
    });
    $[48] = step.title;
    $[49] = t19;
  } else {
    t19 = $[49];
  }
  let t20;
  if ($[50] !== step.description) {
    t20 = /*#__PURE__*/_jsx("p", {
      className: "text-sm text-gray-500 mb-6",
      children: step.description
    });
    $[50] = step.description;
    $[51] = t20;
  } else {
    t20 = $[51];
  }
  let t21;
  if ($[52] !== stepErrors) {
    t21 = stepErrors > 0 && /*#__PURE__*/_jsxs("div", {
      className: "bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm mb-4",
      children: [stepErrors, " field(s) have errors"]
    });
    $[52] = stepErrors;
    $[53] = t21;
  } else {
    t21 = $[53];
  }
  let t22;
  if ($[54] !== formState.data || $[55] !== formState.errors || $[56] !== formState.touched || $[57] !== step.fields) {
    let t23;
    if ($[59] !== formState.data || $[60] !== formState.errors || $[61] !== formState.touched) {
      t23 = field_1 => {
        const value_1 = formState.data[field_1.name] || "";
        const error_0 = formState.errors[field_1.name];
        const touched = formState.touched.has(field_1.name);
        return /*#__PURE__*/_jsxs("div", {
          children: [/*#__PURE__*/_jsxs("label", {
            className: "block text-sm font-medium mb-1",
            children: [field_1.label, field_1.required && /*#__PURE__*/_jsx("span", {
              className: "text-red-500 ml-1",
              children: "*"
            })]
          }), field_1.type === "textarea" ? /*#__PURE__*/_jsx("textarea", {
            value: value_1,
            onChange: e => handleFieldChange(field_1.name, e.target.value),
            className: `w-full border rounded px-3 py-2 ${error_0 && touched ? "border-red-500" : ""}`,
            rows: 3
          }) : field_1.type === "select" ? /*#__PURE__*/_jsxs("select", {
            value: value_1,
            onChange: e_0 => handleFieldChange(field_1.name, e_0.target.value),
            className: `w-full border rounded px-3 py-2 ${error_0 && touched ? "border-red-500" : ""}`,
            children: [/*#__PURE__*/_jsx("option", {
              value: "",
              children: "Select..."
            }), field_1.options?.map(_temp2)]
          }) : field_1.type === "checkbox" ? /*#__PURE__*/_jsx("input", {
            type: "checkbox",
            checked: value_1 === "true",
            onChange: e_1 => handleFieldChange(field_1.name, e_1.target.checked ? "true" : "false")
          }) : /*#__PURE__*/_jsx("input", {
            type: field_1.type,
            value: value_1,
            onChange: e_2 => handleFieldChange(field_1.name, e_2.target.value),
            className: `w-full border rounded px-3 py-2 ${error_0 && touched ? "border-red-500" : ""}`
          }), error_0 && touched && /*#__PURE__*/_jsx("p", {
            className: "text-red-500 text-xs mt-1",
            children: error_0
          })]
        }, field_1.name);
      };
      $[59] = formState.data;
      $[60] = formState.errors;
      $[61] = formState.touched;
      $[62] = t23;
    } else {
      t23 = $[62];
    }
    t22 = step.fields.map(t23);
    $[54] = formState.data;
    $[55] = formState.errors;
    $[56] = formState.touched;
    $[57] = step.fields;
    $[58] = t22;
  } else {
    t22 = $[58];
  }
  let t23;
  if ($[63] !== t22) {
    t23 = /*#__PURE__*/_jsx("div", {
      className: "space-y-4",
      children: t22
    });
    $[63] = t22;
    $[64] = t23;
  } else {
    t23 = $[64];
  }
  let t24;
  if ($[65] !== t19 || $[66] !== t20 || $[67] !== t21 || $[68] !== t23) {
    t24 = /*#__PURE__*/_jsxs("div", {
      className: "bg-white border rounded-lg p-6",
      children: [t19, t20, t21, t23]
    });
    $[65] = t19;
    $[66] = t20;
    $[67] = t21;
    $[68] = t23;
    $[69] = t24;
  } else {
    t24 = $[69];
  }
  let t25;
  if ($[70] !== onCancel) {
    t25 = onCancel && /*#__PURE__*/_jsx("button", {
      onClick: onCancel,
      className: "text-gray-500 hover:text-gray-700",
      children: "Cancel"
    });
    $[70] = onCancel;
    $[71] = t25;
  } else {
    t25 = $[71];
  }
  let t26;
  if ($[72] !== t25) {
    t26 = /*#__PURE__*/_jsx("div", {
      children: t25
    });
    $[72] = t25;
    $[73] = t26;
  } else {
    t26 = $[73];
  }
  let t27;
  if ($[74] !== isFirstStep) {
    t27 = !isFirstStep && /*#__PURE__*/_jsx("button", {
      onClick: handlePrev,
      className: "px-4 py-2 border rounded",
      children: "Back"
    });
    $[74] = isFirstStep;
    $[75] = t27;
  } else {
    t27 = $[75];
  }
  let t28;
  if ($[76] !== handleNext || $[77] !== handleSubmit || $[78] !== isLastStep || $[79] !== submitting) {
    t28 = isLastStep ? /*#__PURE__*/_jsx("button", {
      onClick: handleSubmit,
      disabled: submitting,
      className: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50",
      children: submitting ? "Submitting..." : "Submit"
    }) : /*#__PURE__*/_jsx("button", {
      onClick: handleNext,
      className: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
      children: "Next"
    });
    $[76] = handleNext;
    $[77] = handleSubmit;
    $[78] = isLastStep;
    $[79] = submitting;
    $[80] = t28;
  } else {
    t28 = $[80];
  }
  let t29;
  if ($[81] !== t27 || $[82] !== t28) {
    t29 = /*#__PURE__*/_jsxs("div", {
      className: "flex gap-3",
      children: [t27, t28]
    });
    $[81] = t27;
    $[82] = t28;
    $[83] = t29;
  } else {
    t29 = $[83];
  }
  let t30;
  if ($[84] !== t26 || $[85] !== t29) {
    t30 = /*#__PURE__*/_jsxs("div", {
      className: "flex justify-between mt-6",
      children: [t26, t29]
    });
    $[84] = t26;
    $[85] = t29;
    $[86] = t30;
  } else {
    t30 = $[86];
  }
  let t31;
  if ($[87] !== t16 || $[88] !== t18 || $[89] !== t24 || $[90] !== t30) {
    t31 = /*#__PURE__*/_jsxs("div", {
      className: "max-w-2xl mx-auto",
      children: [t16, t18, t24, t30]
    });
    $[87] = t16;
    $[88] = t18;
    $[89] = t24;
    $[90] = t30;
    $[91] = t31;
  } else {
    t31 = $[91];
  }
  return t31;
}
function _temp2(opt) {
  return /*#__PURE__*/_jsx("option", {
    value: opt,
    children: opt
  }, opt);
}
function _temp(s_1) {
  return Math.max(s_1 - 1, 0);
}