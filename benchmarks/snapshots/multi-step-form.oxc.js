import { c as _c } from "react/compiler-runtime";
// L tier - Inspired by cal.com event type creation wizard
import { useState, useMemo, useCallback, useReducer } from 'react';

interface FormField {
  name: string;
  label: string;
  type: 'text' | 'number' | 'email' | 'select' | 'textarea' | 'checkbox';
  required?: boolean;
  options?: string[];
  validate?: (value: string) => string | null;
}

interface Step {
  title: string;
  description: string;
  fields: FormField[];
}

type FormData = Record<string, string>;
type FormErrors = Record<string, string>;

type FormAction =
  | { type: 'SET_FIELD'; name: string; value: string }
  | { type: 'SET_ERRORS'; errors: FormErrors }
  | { type: 'CLEAR_ERROR'; name: string }
  | { type: 'RESET' };

interface FormState {
  data: FormData;
  errors: FormErrors;
  touched: Set<string>;
}

function formReducer(state: FormState, action: FormAction): FormState {
  switch (action.type) {
    case 'SET_FIELD':
      return {
        ...state,
        data: { ...state.data, [action.name]: action.value },
        touched: new Set([...state.touched, action.name]),
      };
    case 'SET_ERRORS':
      return { ...state, errors: action.errors };
    case 'CLEAR_ERROR': {
      const errors = { ...state.errors };
      delete errors[action.name];
      return { ...state, errors };
    }
    case 'RESET':
      return { data: {}, errors: {}, touched: new Set() };
    default:
      return state;
  }
}

interface MultiStepFormProps {
  steps: Step[];
  onSubmit: (data: FormData) => void;
  onCancel?: () => void;
}

export function MultiStepForm(t0) {
  const $ = _c(76);
  const { steps, onSubmit, onCancel } = t0;
  let t7;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    $[0] = t7;
  } else {
    t7 = $[0];
  }
  let t20;
  if ($[1] !== t21) {
    $[1] = t21;
    $[2] = t20;
  } else {
    t20 = $[2];
  }
  let t26;
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    $[3] = t26;
  } else {
    t26 = $[3];
  }
  let step;
  step = steps[currentStep];
  let isFirstStep;
  isFirstStep = currentStep === 0;
  let isLastStep;
  isLastStep = currentStep === steps.length - 1;
  let progress;
  const t50 = () => {
    return Math.round(currentStep + 1 / steps.length * 100);
  };
  const t55 = useMemo(t50, [currentStep, steps.length]);
  let t236;
  let t235;
  let t60;
  let t64;
  if ($[4] !== t55 || $[5] !== formState.data || $[6] !== steps) {
    t235 = t55;
    t60 = () => {
      let completed;
      completed = 0;
      let total;
      total = 0;
      const t6 = steps[Symbol.iterator]();
      const t7 = t6.next();
      let s;
      s = t7;
      const t11 = s.fields[Symbol.iterator]();
      const t12 = t11.next();
      let field;
      field = t12;
      if (formState.data[field.name]) {
      }
    };
    t64 = [steps, formState.data];
    $[4] = t55;
    $[5] = formState.data;
    $[6] = steps;
    $[7] = t235;
    $[8] = t236;
    $[9] = t60;
    $[10] = t64;
  } else {
    t235 = $[7];
    t236 = $[8];
    t60 = $[9];
    t64 = $[10];
  }
  progress = t235;
  const completedFields = t236;
  const t65 = useMemo(t60, t64);
  let t238;
  let t237;
  let t71;
  let t75;
  if ($[11] !== t65 || $[12] !== formState.data || $[13] !== steps) {
    t237 = t65;
    t71 = (stepIndex) => {
      let stepToValidate;
      stepToValidate = steps[stepIndex];
      let errors;
      errors = {};
      let valid;
      valid = true;
      const t12 = stepToValidate.fields[Symbol.iterator]();
      const t13 = t12.next();
      let field;
      field = t13;
      let value;
      let t16;
      t16 = formState.data[field.name];
      t16 = "";
      value = t16;
      let t24;
      t24 = field.required;
      t24 = !value.trim();
      if (t24) {
        errors[field.name] = `${field.label} is required`;
        valid = false;
      } else {
        if (field.validate) {
          let error;
          error = field.validate(value);
          if (error) {
            errors[field.name] = error;
            valid = false;
          }
        }
      }
    };
    t75 = [steps, formState.data];
    $[11] = t65;
    $[12] = formState.data;
    $[13] = steps;
    $[14] = t237;
    $[15] = t238;
    $[16] = t71;
    $[17] = t75;
  } else {
    t237 = $[14];
    t238 = $[15];
    t71 = $[16];
    t75 = $[17];
  }
  const completedFields = t237;
  const validateStep = t238;
  const t76 = useCallback(t71, t75);
  let t239;
  let validateStep;
  let t81;
  let t86;
  if ($[18] !== t76 || $[19] !== steps.length) {
    validateStep = t76;
    t81 = () => {
      if (validateStep(currentStep)) {
        const t7 = (s) => {
          return Math.min(s + 1, steps.length - 1);
        };
        const t8 = setCurrentStep(t7);
      }
      return undefined;
    };
    t86 = [currentStep, validateStep, steps.length];
    $[18] = t76;
    $[19] = steps.length;
    $[20] = validateStep;
    $[21] = t239;
    $[22] = t81;
    $[23] = t86;
  } else {
    validateStep = $[20];
    t239 = $[21];
    t81 = $[22];
    t86 = $[23];
  }
  const handleNext = t239;
  const t87 = useCallback(t81, t86);
  let t241;
  let t240;
  let t92;
  let t93;
  if ($[24] !== t87) {
    t240 = t87;
    t92 = () => {
      const t2 = (s) => {
        return Math.max(s - 1, 0);
      };
      const t3 = setCurrentStep(t2);
      return undefined;
    };
    t93 = [];
    $[24] = t87;
    $[25] = t240;
    $[26] = t241;
    $[27] = t92;
    $[28] = t93;
  } else {
    t240 = $[25];
    t241 = $[26];
    t92 = $[27];
    t93 = $[28];
  }
  const handleNext = t240;
  const handlePrev = t241;
  const t94 = useCallback(t92, t93);
  let t243;
  let t242;
  let t99;
  let t105;
  if ($[29] !== t94 || $[30] !== formState.data || $[31] !== onSubmit || $[32] !== steps) {
    t242 = t94;
    t99 = async () => {
      if (!validateStep(currentStep)) {
        return undefined;
      }
      const t10 = setSubmitting(true);
      const t16 = onSubmit(formState.data);
      const t19 = setSubmitting(false);
      return undefined;
    };
    t105 = [currentStep, validateStep, formState.data, onSubmit];
    $[29] = t94;
    $[30] = formState.data;
    $[31] = onSubmit;
    $[32] = steps;
    $[33] = t242;
    $[34] = t243;
    $[35] = t99;
    $[36] = t105;
  } else {
    t242 = $[33];
    t243 = $[34];
    t99 = $[35];
    t105 = $[36];
  }
  const handlePrev = t242;
  const handleSubmit = t243;
  const t106 = useCallback(t99, t105);
  let t244;
  if ($[37] !== t106) {
    t244 = t106;
    $[37] = t106;
    $[38] = t244;
  } else {
    t244 = $[38];
  }
  const handleSubmit = t244;
  let t245;
  let t111;
  let t112;
  if ($[39] === Symbol.for("react.memo_cache_sentinel")) {
    t111 = (name, value) => {
      const t8 = dispatch({ type: "SET_FIELD", name, value });
      const t13 = dispatch({ type: "CLEAR_ERROR", name });
      return undefined;
    };
    t112 = [];
    $[39] = t245;
    $[40] = t111;
    $[41] = t112;
  } else {
    t245 = $[39];
    t111 = $[40];
    t112 = $[41];
  }
  const handleFieldChange = t245;
  const t113 = useCallback(t111, t112);
  let t246;
  if ($[42] !== t113) {
    t246 = t113;
    $[42] = t113;
    $[43] = t246;
  } else {
    t246 = $[43];
  }
  const handleFieldChange = t246;
  let stepErrors;
  const t118 = () => {
    const t3 = (f) => {
      return formState.errors[f.name];
    };
    return step.fields.filter(t3).length;
  };
  const t124 = useMemo(t118, [step.fields, formState.errors]);
  let t178;
  let t247;
  let t126;
  let t127;
  let t159;
  let t165;
  let t166;
  let t167;
  let t172;
  let t177;
  if ($[44] !== t124 || $[45] !== completedFields.completed || $[46] !== completedFields.total || $[47] !== formState.data || $[48] !== step.title || $[49] !== step.description || $[50] !== steps) {
    t247 = t124;
    t126 = "div";
    t127 = "max-w-2xl mx-auto";
    t159 = (
      <div className="mb-6">
        <div className="flex justify-between text-sm text-gray-500 mb-1"><span>Step {currentStep + 1} of {steps.length}</span><span>{completedFields.completed}/{completedFields.total} fields completed</span></div>
        <div className="w-full bg-gray-200 rounded h-2"><div className="bg-blue-600 rounded h-2 transition-all" style={{ width: `${progress}%` }} /></div>
      </div>
    );
    const t163 = (s, i) => {
      let t10;
      if (i < currentStep) {
        t10 = "bg-green-500 text-white";
      } else {
        let t15;
        if (i === currentStep) {
          t15 = "bg-blue-600 text-white";
        } else {
          t15 = "bg-gray-200 text-gray-500";
        }
        t10 = t15;
      }
      let t22;
      if (i < currentStep) {
        t22 = "✓";
      } else {
        t22 = i + 1;
      }
      let t32;
      if (i === currentStep) {
        t32 = "font-semibold";
      } else {
        t32 = "text-gray-400";
      }
      let t39;
      t39 = i < steps.length - 1;
      t39 = <div className="flex-1 h-px bg-gray-200 mx-4" />;
      return <div key={i} className="flex-1 flex items-center"><div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm ${t10}`}>{t22}</div><span className={`ml-2 text-sm ${t32}`}>{s.title}</span>{t39}</div>;
    };
    t165 = <div className="flex mb-8">{steps.map(t163)}</div>;
    t166 = "div";
    t167 = "bg-white border rounded-lg p-6";
    t172 = <h2 className="text-xl font-semibold mb-1">{step.title}</h2>;
    t177 = <p className="text-sm text-gray-500 mb-6">{step.description}</p>;
    $[44] = t124;
    $[45] = completedFields.completed;
    $[46] = completedFields.total;
    $[47] = formState.data;
    $[48] = step.title;
    $[49] = step.description;
    $[50] = steps;
    $[51] = t247;
    $[52] = t126;
    $[53] = t127;
    $[54] = t159;
    $[55] = t165;
    $[56] = t166;
    $[57] = t167;
    $[58] = t172;
    $[59] = t177;
    $[60] = t178;
  } else {
    t247 = $[51];
    t126 = $[52];
    t127 = $[53];
    t159 = $[54];
    t165 = $[55];
    t166 = $[56];
    t167 = $[57];
    t172 = $[58];
    t177 = $[59];
    t178 = $[60];
  }
  stepErrors = t247;
  let t234;
  let t118;
  let t123;
  let t248;
  let t249;
  let t50;
  let t54;
  let t250;
  let t251;
  if ($[61] !== t172 || $[62] !== steps) {
    t178 = stepErrors > 0;
    $[61] = t172;
    $[62] = steps;
    $[63] = t178;
    $[64] = t234;
    $[65] = stepErrors;
    $[66] = t118;
    $[67] = t123;
    $[68] = t248;
    $[69] = t249;
    $[70] = t50;
    $[71] = t54;
    $[72] = t250;
    $[73] = t251;
  } else {
    t178 = $[63];
    t234 = $[64];
    stepErrors = $[65];
    t118 = $[66];
    t123 = $[67];
    t248 = $[68];
    t249 = $[69];
    t50 = $[70];
    t54 = $[71];
    t250 = $[72];
    t251 = $[73];
  }
  step = t248;
  progress = t249;
  const currentStep = t250;
  const formState = t251;
  t178 = <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm mb-4">{stepErrors} field(s) have errors\n          </div>;
  const t191 = (field) => {
    let value;
    let t2;
    t2 = formState.data[field.name];
    t2 = "";
    value = t2;
    let error;
    error = formState.errors[field.name];
    let touched;
    touched = formState.touched.has(field.name);
    let t29;
    t29 = field.required;
    t29 = <span className="text-red-500 ml-1">*</span>;
    let t41;
    if (field.type === "textarea") {
      const t44 = (e) => {
        return handleFieldChange(field.name, e.target.value);
      };
      let t45;
      t45 = error;
      t45 = touched;
      let t48;
      if (t45) {
        t48 = "border-red-500";
      } else {
        t48 = "";
      }
      t41 = <textarea value={value} onChange={t44} className={`w-full border rounded px-3 py-2 ${t48}`} rows={3} />;
    } else {
      let t58;
      if (field.type === "select") {
        const t61 = (e) => {
          return handleFieldChange(field.name, e.target.value);
        };
        let t62;
        t62 = error;
        t62 = touched;
        let t65;
        if (t62) {
          t65 = "border-red-500";
        } else {
          t65 = "";
        }
        const t75 = (opt) => {
          return <option key={opt} value={opt}>{opt}</option>;
        };
        t58 = <select value={value} onChange={t61} className={`w-full border rounded px-3 py-2 ${t65}`}><option value="">Select...</option>{field.options.map(t75)}</select>;
      } else {
        let t82;
        if (field.type === "checkbox") {
          const t88 = (e) => {
            let t9;
            if (e.target.checked) {
              t9 = "true";
            } else {
              t9 = "false";
            }
            return handleFieldChange(field.name, t9);
          };
          t82 = <input type="checkbox" checked={value === "true"} onChange={t88} />;
        } else {
          const t94 = (e) => {
            return handleFieldChange(field.name, e.target.value);
          };
          let t95;
          t95 = error;
          t95 = touched;
          let t98;
          if (t95) {
            t98 = "border-red-500";
          } else {
            t98 = "";
          }
          t82 = <input type={field.type} value={value} onChange={t94} className={`w-full border rounded px-3 py-2 ${t98}`} />;
        }
        t58 = t82;
      }
      t41 = t58;
    }
    let t103;
    let t104;
    t104 = error;
    t104 = touched;
    t103 = t104;
    t103 = <p className="text-red-500 text-xs mt-1">{error}</p>;
    return <div key={field.name}><label className="block text-sm font-medium mb-1">{field.label}{t29}</label>{t41}{t103}</div>;
  };
  let t198;
  if ($[74] !== onCancel) {
    $[74] = onCancel;
    $[75] = t198;
  } else {
    t198 = $[75];
  }
  t198 = onCancel;
  t198 = <button onClick={onCancel} className="text-gray-500 hover:text-gray-700">\n              Cancel\n            </button>;
  let t208;
  t208 = !isFirstStep;
  t208 = <button onClick={handlePrev} className="px-4 py-2 border rounded">\n              Back\n            </button>;
  let t217;
  if (isLastStep) {
    let t223;
    if (submitting) {
      t223 = "Submitting...";
    } else {
      t223 = "Submit";
    }
    t217 = <button onClick={handleSubmit} disabled={submitting} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50">{t223}</button>;
  } else {
    t217 = <button onClick={handleNext} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">\n              Next\n            </button>;
  }
  return <t126 className={t127}>{t159}{t165}<t166 className={t167}>{t172}{t177}{t178}<div className="space-y-4">{step.fields.map(t191)}</div></t166><div className="flex justify-between mt-6"><div>{t198}</div><div className="flex gap-3">{t208}{t217}</div></div></t126>;
}

