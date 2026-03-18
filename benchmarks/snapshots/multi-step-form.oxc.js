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
  const $ = _c(72);
  let t7;
  let t20;
  let t26;
  let t224;
  let t225;
  let t60;
  let t64;
  let t226;
  let t227;
  let t71;
  let t75;
  let validateStep;
  let t228;
  let t81;
  let t86;
  let t229;
  let t230;
  let t92;
  let t93;
  let t231;
  let t232;
  let t99;
  let t105;
  let t233;
  let t111;
  let t112;
  let t234;
  let t235;
  let t118;
  let t123;
  let stepErrors;
  let t126;
  let t127;
  let t159;
  let t165;
  let t166;
  let t167;
  let t172;
  let t177;
  let t178;
  let t198;
  let t208;
  let t217;
  let t223;
  let { steps, onSubmit, onCancel } = t0;
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    t7 = 0;
    $[0] = t7;
  } else {
    t7 = $[0];
  }
  let currentStep;
  let setCurrentStep;
  ([currentStep, setCurrentStep] = useState(t7));
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    t20 = { data: {}, errors: {}, touched: new Set() };
    $[1] = t20;
  } else {
    t20 = $[1];
  }
  let t21 = useReducer(formReducer, t20);
  let formState;
  let dispatch;
  ([formState, dispatch] = t21);
  if ($[2] !== t21) {
    $[2] = t21;
    $[3] = formState;
    $[4] = dispatch;
  } else {
    formState = $[3];
    dispatch = $[4];
  }
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    t26 = false;
    $[5] = t26;
  } else {
    t26 = $[5];
  }
  let submitting;
  let setSubmitting;
  ([submitting, setSubmitting] = useState(t26));
  let step;
  step = steps[currentStep];
  let isFirstStep;
  isFirstStep = currentStep === 0;
  let isLastStep;
  isLastStep = currentStep === steps.length - 1;
  let progress;
  let t50 = () => {
    return Math.round(currentStep + 1 / steps.length * 100);
  };
  let t55 = useMemo(t50, [currentStep, steps.length]);
  if ($[6] !== t55 || $[7] !== formState.data || $[8] !== steps) {
    t224 = t55;
    t60 = () => {
      let completed;
      completed = 0;
      let total;
      total = 0;
      let t6 = steps[Symbol.iterator]();
      let t7 = t6.next();
      let s;
      s = t7;
      let t11 = s.fields[Symbol.iterator]();
      let t12 = t11.next();
      let field;
      field = t12;
      if (formState.data[field.name]) {
      }
    };
    t64 = [steps, formState.data];
    $[6] = t55;
    $[7] = formState.data;
    $[8] = steps;
    $[9] = t224;
    $[10] = t225;
    $[11] = t60;
    $[12] = t64;
  } else {
    t224 = $[9];
    t225 = $[10];
    t60 = $[11];
    t64 = $[12];
  }
  progress = t224;
  let completedFields = t225;
  let t65 = useMemo(t60, t64);
  if ($[13] !== t65 || $[14] !== formState.data || $[15] !== steps) {
    t226 = t65;
    t71 = (stepIndex) => {
      let stepToValidate;
      stepToValidate = steps[stepIndex];
      let errors;
      errors = {};
      let valid;
      valid = true;
      let t12 = stepToValidate.fields[Symbol.iterator]();
      let t13 = t12.next();
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
    $[13] = t65;
    $[14] = formState.data;
    $[15] = steps;
    $[16] = t226;
    $[17] = t227;
    $[18] = t71;
    $[19] = t75;
  } else {
    t226 = $[16];
    t227 = $[17];
    t71 = $[18];
    t75 = $[19];
  }
  completedFields = t226;
  validateStep = t227;
  let t76 = useCallback(t71, t75);
  if ($[20] !== t76 || $[21] !== steps.length) {
    validateStep = t76;
    t81 = () => {
      if (validateStep(currentStep)) {
        let t7 = (s) => {
          return Math.min(s + 1, steps.length - 1);
        };
        let t8 = setCurrentStep(t7);
      }
      return undefined;
    };
    t86 = [currentStep, validateStep, steps.length];
    $[20] = t76;
    $[21] = steps.length;
    $[22] = validateStep;
    $[23] = t228;
    $[24] = t81;
    $[25] = t86;
  } else {
    validateStep = $[22];
    t228 = $[23];
    t81 = $[24];
    t86 = $[25];
  }
  let handleNext = t228;
  let t87 = useCallback(t81, t86);
  if ($[26] !== t87) {
    t229 = t87;
    t92 = () => {
      let t2 = (s) => {
        return Math.max(s - 1, 0);
      };
      let t3 = setCurrentStep(t2);
      return undefined;
    };
    t93 = [];
    $[26] = t87;
    $[27] = t229;
    $[28] = t230;
    $[29] = t92;
    $[30] = t93;
  } else {
    t229 = $[27];
    t230 = $[28];
    t92 = $[29];
    t93 = $[30];
  }
  handleNext = t229;
  let handlePrev = t230;
  let t94 = useCallback(t92, t93);
  if ($[31] !== t94 || $[32] !== formState.data || $[33] !== onSubmit || $[34] !== steps) {
    t231 = t94;
    t99 = async () => {
      if (!validateStep(currentStep)) {
        return undefined;
      }
      let t10 = setSubmitting(true);
      let t16 = onSubmit(formState.data);
      let t19 = setSubmitting(false);
      return undefined;
    };
    t105 = [currentStep, validateStep, formState.data, onSubmit];
    $[31] = t94;
    $[32] = formState.data;
    $[33] = onSubmit;
    $[34] = steps;
    $[35] = t231;
    $[36] = t232;
    $[37] = t99;
    $[38] = t105;
  } else {
    t231 = $[35];
    t232 = $[36];
    t99 = $[37];
    t105 = $[38];
  }
  handlePrev = t231;
  let handleSubmit = t232;
  handleSubmit = useCallback(t99, t105);
  if ($[39] === Symbol.for("react.memo_cache_sentinel")) {
    t111 = (name, value) => {
      let t8 = dispatch({ type: "SET_FIELD", name, value });
      let t13 = dispatch({ type: "CLEAR_ERROR", name });
      return undefined;
    };
    t112 = [];
    $[39] = t233;
    $[40] = t111;
    $[41] = t112;
  } else {
    t233 = $[39];
    t111 = $[40];
    t112 = $[41];
  }
  let handleFieldChange = t233;
  let t113 = useCallback(t111, t112);
  if ($[42] !== t113) {
    t234 = t113;
    $[42] = t113;
    $[43] = t234;
  } else {
    t234 = $[43];
  }
  handleFieldChange = t234;
  if ($[44] !== formState.errors || $[45] !== step.fields) {
    t118 = () => {
      let t3 = (f) => {
        return formState.errors[f.name];
      };
      return step.fields.filter(t3).length;
    };
    t123 = [step.fields, formState.errors];
    $[44] = formState.errors;
    $[45] = step.fields;
    $[46] = t235;
    $[47] = t118;
    $[48] = t123;
  } else {
    t235 = $[46];
    t118 = $[47];
    t123 = $[48];
  }
  stepErrors = t235;
  let t124 = useMemo(t118, t123);
  if ($[49] !== t124 || $[50] !== completedFields.completed || $[51] !== completedFields.total || $[52] !== formState.data || $[53] !== step.title || $[54] !== step.description || $[55] !== steps) {
    stepErrors = t124;
    t126 = "div";
    t127 = "max-w-2xl mx-auto";
    t159 = (
      <div className="mb-6">
        <div className="flex justify-between text-sm text-gray-500 mb-1"><span>Step {currentStep + 1} of {steps.length}</span><span>{completedFields.completed}/{completedFields.total} fields completed</span></div>
        <div className="w-full bg-gray-200 rounded h-2"><div className="bg-blue-600 rounded h-2 transition-all" style={{ width: `${progress}%` }} /></div>
      </div>
    );
    let t163 = (s, i) => {
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
    t178 = stepErrors > 0;
    t178 = <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm mb-4">{stepErrors} field(s) have errors\n          </div>;
    let t191 = (field) => {
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
        let t44 = (e) => {
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
          let t61 = (e) => {
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
          let t75 = (opt) => {
            return <option key={opt} value={opt}>{opt}</option>;
          };
          t58 = <select value={value} onChange={t61} className={`w-full border rounded px-3 py-2 ${t65}`}><option value="">Select...</option>{field.options.map(t75)}</select>;
        } else {
          let t82;
          if (field.type === "checkbox") {
            let t88 = (e) => {
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
            let t94 = (e) => {
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
    if ($[56] === Symbol.for("react.memo_cache_sentinel")) {
      $[56] = t198;
    } else {
      t198 = $[56];
    }
    $[49] = t124;
    $[50] = completedFields.completed;
    $[51] = completedFields.total;
    $[52] = formState.data;
    $[53] = step.title;
    $[54] = step.description;
    $[55] = steps;
    $[56] = stepErrors;
    $[57] = t126;
    $[58] = t127;
    $[59] = t159;
    $[60] = t165;
    $[61] = t166;
    $[62] = t167;
    $[63] = t172;
    $[64] = t177;
    $[65] = t178;
    $[66] = t178;
    $[67] = t198;
    $[68] = t208;
    $[69] = t217;
    $[70] = t223;
  } else {
    stepErrors = $[56];
    t126 = $[57];
    t127 = $[58];
    t159 = $[59];
    t165 = $[60];
    t166 = $[61];
    t167 = $[62];
    t172 = $[63];
    t177 = $[64];
    t178 = $[65];
    t178 = $[66];
    t198 = $[67];
    t208 = $[68];
    t217 = $[69];
    t223 = $[70];
  }
}

