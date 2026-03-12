import { c as _c } from "react/compiler-runtime";
import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
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
  let steps;
  let onSubmit;
  let onCancel;
  if ($[0] !== steps || $[1] !== onSubmit || $[2] !== onCancel) {
    $[0] = steps;
    $[1] = onSubmit;
    $[2] = onCancel;
  } else {
  }
  ({ steps, onSubmit, onCancel } = t0);
  const t318 = useState;
  const t319 = 0;
  const t320 = t318(t319);
  let currentStep;
  let setCurrentStep;
  if ($[3] !== currentStep || $[4] !== setCurrentStep) {
    $[3] = currentStep;
    $[4] = setCurrentStep;
  } else {
  }
  ([currentStep, setCurrentStep] = t320);
  const t324 = useReducer;
  const t325 = formReducer;
  const t326 = {};
  const t327 = {};
  const t328 = Set;
  const t329 = new t328();
  const t330 = { data: t326, errors: t327, touched: t329 };
  const t331 = t324(t325, t330);
  let formState;
  let dispatch;
  if ($[5] !== formState || $[6] !== dispatch) {
    $[5] = formState;
    $[6] = dispatch;
  } else {
  }
  ([formState, dispatch] = t331);
  const t335 = useState;
  const t336 = false;
  const t337 = t335(t336);
  let submitting;
  let setSubmitting;
  if ($[7] !== submitting || $[8] !== setSubmitting) {
    $[7] = submitting;
    $[8] = setSubmitting;
  } else {
  }
  ([submitting, setSubmitting] = t337);
  let step;
  let isFirstStep;
  let isLastStep;
  let progress;
  if ($[9] !== step || $[10] !== steps || $[11] !== currentStep || $[12] !== isFirstStep || $[13] !== currentStep || $[14] !== isLastStep || $[15] !== currentStep || $[16] !== steps || $[17] !== progress) {
    const t342 = steps;
    const t343 = currentStep;
    const t344 = t342[t343];
    step = t344;
    const t347 = currentStep;
    const t348 = 0;
    const t349 = t347 === t348;
    isFirstStep = t349;
    const t352 = currentStep;
    const t353 = steps;
    const t354 = t353.length;
    const t355 = 1;
    const t356 = t354 - t355;
    const t357 = t352 === t356;
    isLastStep = t357;
    $[9] = step;
    $[10] = steps;
    $[11] = currentStep;
    $[12] = isFirstStep;
    $[13] = currentStep;
    $[14] = isLastStep;
    $[15] = currentStep;
    $[16] = steps;
    $[17] = progress;
  } else {
  }
  let completedFields;
  if ($[18] !== useMemo || $[19] !== currentStep || $[20] !== steps || $[21] !== progress || $[22] !== completedFields) {
    const t360 = useMemo;
    const t361 = () => {
      const t0 = Math;
      const t2 = currentStep;
      const t3 = 1;
      const t4 = t2 + t3;
      const t6 = steps;
      const t7 = t6.length;
      const t8 = t4 / t7;
      const t9 = 100;
      const t10 = t8 * t9;
      const t11 = t0.round(t10);
      return t11;
    };
    const t362 = currentStep;
    const t363 = steps;
    const t364 = t363.length;
    const t365 = [t362, t364];
    const t366 = t360(t361, t365);
    progress = t366;
    $[18] = useMemo;
    $[19] = currentStep;
    $[20] = steps;
    $[21] = progress;
    $[22] = completedFields;
  } else {
  }
  let validateStep;
  if ($[23] !== useMemo || $[24] !== steps || $[25] !== formState || $[26] !== completedFields || $[27] !== validateStep) {
    const t369 = useMemo;
    const t370 = () => {
      let completed;
      const t2 = 0;
      completed = t2;
      let total;
      const t6 = 0;
      total = t6;
      const t9 = steps;
      const t10 = t9[Symbol.iterator]();
      const t11 = t10.next();
      let s;
      s = t11;
      const t16 = s;
      const t17 = t16.fields;
      const t18 = t17[Symbol.iterator]();
      const t35 = completed;
      const t37 = total;
      const t38 = { completed, total };
      return t38;
      const t19 = t18.next();
      let field;
      field = t19;
      const t24 = total++;
      const t26 = formState;
      const t27 = t26.data;
      const t29 = field;
      const t30 = t29.name;
      const t31 = t27[t30];
      const t33 = completed++;
      const t39 = undefined;
      return t39;
    };
    const t371 = steps;
    const t372 = formState;
    const t373 = t372.data;
    const t374 = [t371, t373];
    const t375 = t369(t370, t374);
    completedFields = t375;
    $[23] = useMemo;
    $[24] = steps;
    $[25] = formState;
    $[26] = completedFields;
    $[27] = validateStep;
  } else {
  }
  let handleNext;
  if ($[28] !== useCallback || $[29] !== steps || $[30] !== formState || $[31] !== validateStep || $[32] !== handleNext) {
    const t378 = useCallback;
    const t379 = (stepIndex) => {
      let stepToValidate;
      const t4 = steps;
      const t6 = stepIndex;
      const t7 = t4[t6];
      stepToValidate = t7;
      let errors;
      const t11 = {};
      errors = t11;
      let valid;
      const t15 = true;
      valid = t15;
      const t18 = stepToValidate;
      const t19 = t18.fields;
      const t20 = t19[Symbol.iterator]();
      const t21 = t20.next();
      let field;
      field = t21;
      let value;
      const t83 = dispatch;
      const t84 = "SET_ERRORS";
      const t86 = errors;
      const t87 = { type: t84, errors };
      const t88 = t83(t87);
      const t90 = valid;
      return t90;
      const t28 = formState;
      const t29 = t28.data;
      const t31 = field;
      const t32 = t31.name;
      const t33 = t29[t32];
      const t34 = "";
      value = t35;
      const t38 = field;
      const t39 = t38.required;
      const t41 = value;
      const t42 = t41.trim();
      const t43 = !t42;
      const t46 = field;
      const t47 = t46.label;
      const t48 = `${t47} is required`;
      const t50 = errors;
      const t52 = field;
      const t53 = t52.name;
      t50[t53] = t48;
      const t55 = false;
      valid = t55;
      const t59 = field;
      const t60 = t59.validate;
      let error;
      const t64 = field;
      const t66 = value;
      const t67 = t64.validate(t66);
      error = t67;
      const t70 = error;
      const t72 = error;
      const t74 = errors;
      const t76 = field;
      const t77 = t76.name;
      t74[t77] = t72;
      const t79 = false;
      valid = t79;
      const t91 = undefined;
      return t91;
    };
    const t380 = steps;
    const t381 = formState;
    const t382 = t381.data;
    const t383 = [t380, t382];
    const t384 = t378(t379, t383);
    validateStep = t384;
    $[28] = useCallback;
    $[29] = steps;
    $[30] = formState;
    $[31] = validateStep;
    $[32] = handleNext;
  } else {
  }
  let handlePrev;
  if ($[33] !== useCallback || $[34] !== currentStep || $[35] !== validateStep || $[36] !== steps || $[37] !== handleNext || $[38] !== handlePrev) {
    const t387 = useCallback;
    const t388 = () => {
      const t1 = validateStep;
      const t3 = currentStep;
      const t4 = t1(t3);
      const t6 = setCurrentStep;
      const t7 = (s) => {
        const t1 = Math;
        const t3 = s;
        const t4 = 1;
        const t5 = t3 + t4;
        const t7 = steps;
        const t8 = t7.length;
        const t9 = 1;
        const t10 = t8 - t9;
        const t11 = t1.min(t5, t10);
        return t11;
      };
      const t8 = t6(t7);
      const t9 = undefined;
      return t9;
    };
    const t389 = currentStep;
    const t390 = validateStep;
    const t391 = steps;
    const t392 = t391.length;
    const t393 = [t389, t390, t392];
    const t394 = t387(t388, t393);
    handleNext = t394;
    $[33] = useCallback;
    $[34] = currentStep;
    $[35] = validateStep;
    $[36] = steps;
    $[37] = handleNext;
    $[38] = handlePrev;
  } else {
  }
  const t397 = useCallback;
  const t398 = () => {
    const t1 = setCurrentStep;
    const t2 = (s) => {
      const t1 = Math;
      const t3 = s;
      const t4 = 1;
      const t5 = t3 - t4;
      const t6 = 0;
      const t7 = t1.max(t5, t6);
      return t7;
    };
    const t3 = t1(t2);
    const t4 = undefined;
    return t4;
  };
  const t399 = [];
  const t400 = t397(t398, t399);
  handlePrev = t400;
  let handleSubmit;
  if ($[39] !== handleSubmit) {
    $[39] = handleSubmit;
  } else {
  }
  let handleFieldChange;
  if ($[40] !== useCallback || $[41] !== currentStep || $[42] !== validateStep || $[43] !== formState || $[44] !== onSubmit || $[45] !== handleSubmit || $[46] !== handleFieldChange) {
    const t403 = useCallback;
    const t404 = async () => {
      const t1 = validateStep;
      const t3 = currentStep;
      const t4 = t1(t3);
      const t5 = !t4;
      const t6 = undefined;
      return t6;
      const t8 = setSubmitting;
      const t9 = true;
      const t10 = t8(t9);
      const t12 = onSubmit;
      const t14 = formState;
      const t15 = t14.data;
      const t16 = t12(t15);
      const t18 = setSubmitting;
      const t19 = false;
      const t20 = t18(t19);
      const t21 = undefined;
      return t21;
    };
    const t405 = currentStep;
    const t406 = validateStep;
    const t407 = formState;
    const t408 = t407.data;
    const t409 = onSubmit;
    const t410 = [t405, t406, t408, t409];
    const t411 = t403(t404, t410);
    handleSubmit = t411;
    $[40] = useCallback;
    $[41] = currentStep;
    $[42] = validateStep;
    $[43] = formState;
    $[44] = onSubmit;
    $[45] = handleSubmit;
    $[46] = handleFieldChange;
  } else {
  }
  const t414 = useCallback;
  const t415 = (name, value) => {
    const t3 = dispatch;
    const t4 = "SET_FIELD";
    const t6 = name;
    const t8 = value;
    const t9 = { type: t4, name, value };
    const t10 = t3(t9);
    const t12 = dispatch;
    const t13 = "CLEAR_ERROR";
    const t15 = name;
    const t16 = { type: t13, name };
    const t17 = t12(t16);
    const t18 = undefined;
    return t18;
  };
  const t416 = [];
  const t417 = t414(t415, t416);
  handleFieldChange = t417;
  let stepErrors;
  if ($[47] !== stepErrors) {
    $[47] = stepErrors;
  } else {
  }
  const t420 = useMemo;
  const t421 = () => {
    const t1 = step;
    const t2 = t1.fields;
    const t3 = (f) => {
      const t2 = formState;
      const t3 = t2.errors;
      const t5 = f;
      const t6 = t5.name;
      const t7 = t3[t6];
      return t7;
    };
    const t4 = t2.filter(t3);
    const t5 = t4.length;
    return t5;
  };
  const t422 = step;
  const t423 = t422.fields;
  const t424 = formState;
  const t425 = t424.errors;
  const t426 = [t423, t425];
  const t427 = t420(t421, t426);
  stepErrors = t427;
  const t429 = "div";
  const t430 = "max-w-2xl mx-auto";
  const t431 = "div";
  const t432 = "mb-6";
  const t433 = "div";
  const t434 = "flex justify-between text-sm text-gray-500 mb-1";
  const t435 = "span";
  const t436 = "Step ";
  const t437 = currentStep;
  const t438 = 1;
  const t439 = t437 + t438;
  const t440 = " of ";
  const t441 = steps;
  const t442 = t441.length;
  const t443 = _jsxs(t435, { children: [t436, t439, t440, t442] });
  const t444 = "span";
  const t445 = completedFields;
  const t446 = t445.completed;
  const t447 = "/";
  const t448 = completedFields;
  const t449 = t448.total;
  const t450 = " fields completed";
  const t451 = _jsxs(t444, { children: [t446, t447, t449, t450] });
  const t452 = _jsxs(t433, { className: t434, children: [t443, t451] });
  const t453 = "div";
  const t454 = "w-full bg-gray-200 rounded h-2";
  const t455 = "div";
  const t456 = "bg-blue-600 rounded h-2 transition-all";
  const t457 = progress;
  const t458 = `${t457}%`;
  const t459 = { width: t458 };
  const t460 = _jsx(t455, { className: t456, style: t459 });
  const t461 = _jsx(t453, { className: t454, children: t460 });
  const t462 = _jsxs(t431, { className: t432, children: [t452, t461] });
  const t463 = "div";
  const t464 = "flex mb-8";
  const t465 = steps;
  const t466 = (s, i) => {
    const t2 = "div";
    const t4 = i;
    const t5 = "flex-1 flex items-center";
    const t6 = "div";
    const t8 = i;
    const t10 = currentStep;
    const t11 = t8 < t10;
    const t12 = "bg-green-500 text-white";
    const t14 = i;
    const t16 = currentStep;
    const t17 = t14 === t16;
    const t22 = `w-8 h-8 rounded-full flex items-center justify-center text-sm ${t21}`;
    const t24 = i;
    const t26 = currentStep;
    const t27 = t24 < t26;
    const t18 = "bg-blue-600 text-white";
    const t19 = "bg-gray-200 text-gray-500";
    const t28 = "✓";
    const t30 = i;
    const t31 = 1;
    const t32 = t30 + t31;
    const t34 = _jsx(t6, { className: t22, children: t33 });
    const t35 = "span";
    const t37 = i;
    const t39 = currentStep;
    const t40 = t37 === t39;
    const t41 = "font-semibold";
    const t42 = "text-gray-400";
    const t44 = `ml-2 text-sm ${t43}`;
    const t46 = s;
    const t47 = t46.title;
    const t48 = _jsx(t35, { className: t44, children: t47 });
    const t50 = i;
    const t52 = steps;
    const t53 = t52.length;
    const t54 = 1;
    const t55 = t53 - t54;
    const t56 = t50 < t55;
    const t57 = "div";
    const t58 = "flex-1 h-px bg-gray-200 mx-4";
    const t59 = _jsx(t57, { className: t58 });
    const t61 = _jsxs(t2, { key: t4, className: t5, children: [t34, t48, t60] });
    return t61;
  };
  const t467 = t465.map(t466);
  const t468 = _jsx(t463, { className: t464, children: t467 });
  const t469 = "div";
  const t470 = "bg-white border rounded-lg p-6";
  const t471 = "h2";
  const t472 = "text-xl font-semibold mb-1";
  const t473 = step;
  const t474 = t473.title;
  const t475 = _jsx(t471, { className: t472, children: t474 });
  const t476 = "p";
  const t477 = "text-sm text-gray-500 mb-6";
  const t478 = step;
  const t479 = t478.description;
  const t480 = _jsx(t476, { className: t477, children: t479 });
  if ($[48] !== useMemo || $[49] !== step || $[50] !== formState || $[51] !== stepErrors || $[52] !== stepErrors) {
    const t481 = stepErrors;
    $[48] = useMemo;
    $[49] = step;
    $[50] = formState;
    $[51] = stepErrors;
    $[52] = stepErrors;
  } else {
  }
  const t482 = 0;
}

