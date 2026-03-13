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
  const $ = _c(53);
  const { steps, onSubmit, onCancel } = t0;
  let formState;
  let dispatch;
  if ($[0] !== formReducer || $[1] !== useReducer) {
    $[0] = formReducer;
    $[1] = useReducer;
  }
  let step;
  let isFirstStep;
  let isLastStep;
  let progress;
  if ($[2] !== currentStep || $[3] !== currentStep || $[4] !== currentStep || $[5] !== steps || $[6] !== steps) {
    step = steps[currentStep];
    isFirstStep = currentStep === 0;
    isLastStep = currentStep === steps.length - 1;
    $[2] = currentStep;
    $[3] = currentStep;
    $[4] = currentStep;
    $[5] = steps;
    $[6] = steps;
  }
  let completedFields;
  if ($[7] !== currentStep || $[8] !== steps || $[9] !== useMemo) {
    const t376 = () => {
      return Math.round(currentStep + 1 / steps.length * 100);
    };
    progress = useMemo(t376, [currentStep, steps.length]);
    $[7] = currentStep;
    $[8] = steps;
    $[9] = useMemo;
  }
  let validateStep;
  if ($[10] !== formState || $[11] !== steps || $[12] !== useMemo) {
    const t385 = () => {
      let completed = 0;
      let total = 0;
      const t10 = steps[Symbol.iterator]();
      const t11 = t10.next();
      const s = t11;
      const t18 = s.fields[Symbol.iterator]();
      const t19 = t18.next();
      const field = t19;
      if (formState.data[field.name]) {
      }
    };
    completedFields = useMemo(t385, [steps, formState.data]);
    $[10] = formState;
    $[11] = steps;
    $[12] = useMemo;
  }
  let handleNext;
  if ($[13] !== formState || $[14] !== steps || $[15] !== useCallback) {
    const t394 = (stepIndex) => {
      const stepToValidate = steps[stepIndex];
      const errors = {};
      let valid = true;
      const t20 = stepToValidate.fields[Symbol.iterator]();
      const t21 = t20.next();
      const field = t21;
      t27 = formState.data[field.name];
      t27 = "";
      const value = t27;
      t40 = field.required;
      t40 = !value.trim();
      if (t40) {
        errors[field.name] = `${field.label} is required`;
        valid = false;
      } else {
        if (field.validate) {
          const error = field.validate(value);
          if (error) {
            errors[field.name] = error;
            valid = false;
          }
        }
      }
    };
    validateStep = useCallback(t394, [steps, formState.data]);
    $[13] = formState;
    $[14] = steps;
    $[15] = useCallback;
  }
  let handlePrev;
  if ($[16] !== currentStep || $[17] !== steps || $[18] !== useCallback || $[19] !== validateStep) {
    const t403 = () => {
      if (validateStep(currentStep)) {
        const t7 = (s) => {
          return Math.min(s + 1, steps.length - 1);
        };
        const t8 = setCurrentStep(t7);
      }
      return undefined;
    };
    handleNext = useCallback(t403, [currentStep, validateStep, steps.length]);
    $[16] = currentStep;
    $[17] = steps;
    $[18] = useCallback;
    $[19] = validateStep;
  }
  let handleSubmit;
  if ($[20] !== useCallback) {
    const t413 = () => {
      const t2 = (s) => {
        return Math.max(s - 1, 0);
      };
      const t3 = setCurrentStep(t2);
      return undefined;
    };
    handlePrev = useCallback(t413, []);
    $[20] = useCallback;
  }
  let handleFieldChange;
  if ($[21] !== currentStep || $[22] !== formState || $[23] !== onSubmit || $[24] !== useCallback || $[25] !== validateStep) {
    const t419 = async () => {
      if (!validateStep(currentStep)) {
        return undefined;
      }
      const t10 = setSubmitting(true);
      const t16 = onSubmit(formState.data);
      const t20 = setSubmitting(false);
      return undefined;
    };
    handleSubmit = useCallback(t419, [currentStep, validateStep, formState.data, onSubmit]);
    $[21] = currentStep;
    $[22] = formState;
    $[23] = onSubmit;
    $[24] = useCallback;
    $[25] = validateStep;
  }
  let stepErrors;
  if ($[26] !== useCallback) {
    const t430 = (name, value) => {
      const t10 = dispatch({ type: "SET_FIELD", name, value });
      const t17 = dispatch({ type: "CLEAR_ERROR", name });
      return undefined;
    };
    handleFieldChange = useCallback(t430, []);
    $[26] = useCallback;
  }
  const t436 = () => {
    const t3 = (f) => {
      return formState.errors[f.name];
    };
    return step.fields.filter(t3).length;
  };
  stepErrors = useMemo(t436, [step.fields, formState.errors]);
  const t481 = (s, i) => {
    if (i < currentStep) {
      t12 = "bg-green-500 text-white";
    } else {
      if (i === currentStep) {
        t21 = "bg-blue-600 text-white";
      } else {
        t21 = "bg-gray-200 text-gray-500";
      }
      t12 = t21;
    }
    if (i < currentStep) {
      t34 = "✓";
    } else {
      t34 = i + 1;
    }
    if (i === currentStep) {
      t50 = "font-semibold";
    } else {
      t50 = "text-gray-400";
    }
    t61 = i < steps.length - 1;
    t61 = <div className="flex-1 h-px bg-gray-200 mx-4" />;
    return <div key={i} className="flex-1 flex items-center"><div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm ${t12}`}>{t34}</div><span className={`ml-2 text-sm ${t50}`}>{s.title}</span>{t61}</div>;
  };
  if ($[27] !== formState || $[28] !== handlePrev || $[29] !== isFirstStep || $[30] !== isLastStep || $[31] !== onCancel || $[32] !== onCancel || $[33] !== step || $[34] !== useMemo) {
    t244 = stepErrors > 0;
    $[27] = formState;
    $[28] = handlePrev;
    $[29] = isFirstStep;
    $[30] = isLastStep;
    $[31] = onCancel;
    $[32] = onCancel;
    $[33] = step;
    $[34] = useMemo;
  }
  t244 = <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm mb-4">{stepErrors} field(s) have errors\n          </div>;
  let t568;
  if ($[35] !== t270 || $[36] !== t285 || $[37] !== t310 || $[38] !== t244 || $[39] !== completedFields || $[40] !== completedFields || $[41] !== currentStep || $[42] !== handleNext || $[43] !== handleSubmit || $[44] !== progress || $[45] !== step || $[46] !== step || $[47] !== step || $[48] !== steps || $[49] !== steps || $[50] !== submitting || $[51] !== submitting) {
    const t511 = (field) => {
      t3 = formState.data[field.name];
      t3 = "";
      const value = t3;
      const error = formState.errors[field.name];
      const touched = formState.touched.has(field.name);
      t45 = field.required;
      t45 = <span className="text-red-500 ml-1">*</span>;
      if (field.type === "textarea") {
        const t67 = (e) => {
          return handleFieldChange(field.name, e.target.value);
        };
        t68 = error;
        t68 = touched;
        if (t68) {
          t76 = "border-red-500";
        } else {
          t76 = "";
        }
        t62 = <textarea value={value} onChange={t67} className={`w-full border rounded px-3 py-2 ${t76}`} rows={3} />;
      } else {
        if (field.type === "select") {
          const t96 = (e) => {
            return handleFieldChange(field.name, e.target.value);
          };
          t97 = error;
          t97 = touched;
          if (t97) {
            t105 = "border-red-500";
          } else {
            t105 = "";
          }
          const t119 = (opt) => {
            return <option key={opt} value={opt}>{opt}</option>;
          };
          t91 = <select value={value} onChange={t96} className={`w-full border rounded px-3 py-2 ${t105}`}><option value="">Select...</option>{field.options.map(t119)}</select>;
        } else {
          if (field.type === "checkbox") {
            const t136 = (e) => {
              if (e.target.checked) {
                t10 = "true";
              } else {
                t10 = "false";
              }
              return handleFieldChange(field.name, t10);
            };
            t128 = <input type="checkbox" checked={value === "true"} onChange={t136} />;
          } else {
            const t145 = (e) => {
              return handleFieldChange(field.name, e.target.value);
            };
            t146 = error;
            t146 = touched;
            if (t146) {
              t154 = "border-red-500";
            } else {
              t154 = "";
            }
            t128 = <input type={field.type} value={value} onChange={t145} className={`w-full border rounded px-3 py-2 ${t154}`} />;
          }
          t91 = t128;
        }
        t62 = t91;
      }
      t167 = error;
      t167 = touched;
      t165 = t167;
      t165 = <p className="text-red-500 text-xs mt-1">{error}</p>;
      return <div key={field.name}><label className="block text-sm font-medium mb-1">{field.label}{t45}</label>{t62}{t165}</div>;
    };
    $[35] = t270;
    $[36] = t285;
    $[37] = t310;
    $[38] = t244;
    $[39] = completedFields;
    $[40] = completedFields;
    $[41] = currentStep;
    $[42] = handleNext;
    $[43] = handleSubmit;
    $[44] = progress;
    $[45] = step;
    $[46] = step;
    $[47] = step;
    $[48] = steps;
    $[49] = steps;
    $[50] = submitting;
    $[51] = submitting;
    $[52] = t568;
  } else {
    t568 = $[52];
  }
  t270 = onCancel;
  t270 = <button onClick={onCancel} className="text-gray-500 hover:text-gray-700">\n              Cancel\n            </button>;
  t285 = !isFirstStep;
  t285 = <button onClick={handlePrev} className="px-4 py-2 border rounded">\n              Back\n            </button>;
  if (isLastStep) {
    if (submitting) {
      t310 = "Submitting...";
    } else {
      t310 = "Submit";
    }
    t300 = <button onClick={handleSubmit} disabled={submitting} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50">{t310}</button>;
  } else {
    t300 = <button onClick={handleNext} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">\n              Next\n            </button>;
  }
  return <div className="max-w-2xl mx-auto"><div className="mb-6"><div className="flex justify-between text-sm text-gray-500 mb-1"><span>Step {currentStep + 1} of {steps.length}</span><span>{completedFields.completed}/{completedFields.total} fields completed</span></div><div className="w-full bg-gray-200 rounded h-2"><div className="bg-blue-600 rounded h-2 transition-all" style={{ width: `${progress}%` }} /></div></div><div className="flex mb-8">{steps.map(t481)}</div>{t514}<t515 className={t516}><t517>{t270}</t517><div className="flex gap-3">{t285}{t300}</div></t515></div>;
}

