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
  const $ = _c(81);
  const { steps, onSubmit, onCancel } = t0;
  if ($[0] !== steps || $[1] !== onSubmit || $[2] !== onCancel) {
    $[0] = steps;
    $[1] = onSubmit;
    $[2] = onCancel;
  } else {
  }
  const t333 = useState;
  const t334 = 0;
  const t335 = t333(t334);
  let currentStep;
  let setCurrentStep;
  if ($[3] !== currentStep || $[4] !== setCurrentStep) {
    $[3] = currentStep;
    $[4] = setCurrentStep;
  } else {
  }
  ([currentStep, setCurrentStep] = t335);
  const t339 = useReducer;
  const t340 = formReducer;
  const t341 = {};
  const t342 = {};
  const t343 = Set;
  const t344 = new t343();
  const t345 = { data: t341, errors: t342, touched: t344 };
  const t346 = t339(t340, t345);
  let formState;
  let dispatch;
  if ($[5] !== formState || $[6] !== dispatch) {
    $[5] = formState;
    $[6] = dispatch;
  } else {
  }
  ([formState, dispatch] = t346);
  const t350 = useState;
  const t351 = false;
  const t352 = t350(t351);
  let submitting;
  let setSubmitting;
  if ($[7] !== submitting || $[8] !== setSubmitting) {
    $[7] = submitting;
    $[8] = setSubmitting;
  } else {
  }
  ([submitting, setSubmitting] = t352);
  let step;
  let isFirstStep;
  let isLastStep;
  let progress;
  if ($[9] !== step || $[10] !== steps || $[11] !== currentStep || $[12] !== isFirstStep || $[13] !== currentStep || $[14] !== isLastStep || $[15] !== currentStep || $[16] !== steps || $[17] !== progress) {
    const t357 = steps;
    const t358 = currentStep;
    const t359 = t357[t358];
    step = t359;
    const t362 = currentStep;
    const t363 = 0;
    const t364 = t362 === t363;
    isFirstStep = t364;
    const t367 = currentStep;
    const t368 = steps;
    const t369 = t368.length;
    const t370 = 1;
    const t371 = t369 - t370;
    const t372 = t367 === t371;
    isLastStep = t372;
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
    const t375 = useMemo;
    const t376 = () => {
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
    const t377 = currentStep;
    const t378 = steps;
    const t379 = t378.length;
    const t380 = [t377, t379];
    const t381 = t375(t376, t380);
    progress = t381;
    $[18] = useMemo;
    $[19] = currentStep;
    $[20] = steps;
    $[21] = progress;
    $[22] = completedFields;
  } else {
  }
  let validateStep;
  if ($[23] !== useMemo || $[24] !== steps || $[25] !== formState || $[26] !== completedFields || $[27] !== validateStep) {
    const t384 = useMemo;
    const t385 = () => {
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
      const t19 = t18.next();
      let field;
      field = t19;
      const t24 = total++;
      const t26 = formState;
      const t27 = t26.data;
      const t29 = field;
      const t30 = t29.name;
      const t31 = t27[t30];
      if (t31) {
        const t33 = completed++;
      } else {
      }
    };
    const t386 = steps;
    const t387 = formState;
    const t388 = t387.data;
    const t389 = [t386, t388];
    const t390 = t384(t385, t389);
    completedFields = t390;
    $[23] = useMemo;
    $[24] = steps;
    $[25] = formState;
    $[26] = completedFields;
    $[27] = validateStep;
  } else {
  }
  let handleNext;
  if ($[28] !== useCallback || $[29] !== steps || $[30] !== formState || $[31] !== validateStep || $[32] !== handleNext) {
    const t393 = useCallback;
    const t394 = (stepIndex) => {
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
      let t27;
      const t30 = formState;
      const t31 = t30.data;
      const t33 = field;
      const t34 = t33.name;
      const t35 = t31[t34];
      t27 = t35;
      const t37 = "";
      t27 = t37;
      value = t27;
      let t40;
      const t43 = field;
      const t44 = t43.required;
      t40 = t44;
      const t47 = value;
      const t48 = t47.trim();
      const t49 = !t48;
      t40 = t49;
      if (t40) {
        const t52 = field;
        const t53 = t52.label;
        const t54 = `${t53} is required`;
        const t56 = errors;
        const t58 = field;
        const t59 = t58.name;
        t56[t59] = t54;
        const t61 = false;
        valid = t61;
      } else {
        const t65 = field;
        const t66 = t65.validate;
        if (t66) {
          let error;
          const t70 = field;
          const t72 = value;
          const t73 = t70.validate(t72);
          error = t73;
          const t76 = error;
          if (t76) {
            const t78 = error;
            const t80 = errors;
            const t82 = field;
            const t83 = t82.name;
            t80[t83] = t78;
            const t85 = false;
            valid = t85;
          } else {
          }
        } else {
        }
      }
    };
    const t395 = steps;
    const t396 = formState;
    const t397 = t396.data;
    const t398 = [t395, t397];
    const t399 = t393(t394, t398);
    validateStep = t399;
    $[28] = useCallback;
    $[29] = steps;
    $[30] = formState;
    $[31] = validateStep;
    $[32] = handleNext;
  } else {
  }
  let handlePrev;
  if ($[33] !== useCallback || $[34] !== currentStep || $[35] !== validateStep || $[36] !== steps || $[37] !== handleNext || $[38] !== handlePrev) {
    const t402 = useCallback;
    const t403 = () => {
      const t1 = validateStep;
      const t3 = currentStep;
      const t4 = t1(t3);
      if (t4) {
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
      } else {
      }
      const t9 = undefined;
      return t9;
    };
    const t404 = currentStep;
    const t405 = validateStep;
    const t406 = steps;
    const t407 = t406.length;
    const t408 = [t404, t405, t407];
    const t409 = t402(t403, t408);
    handleNext = t409;
    $[33] = useCallback;
    $[34] = currentStep;
    $[35] = validateStep;
    $[36] = steps;
    $[37] = handleNext;
    $[38] = handlePrev;
  } else {
  }
  const t412 = useCallback;
  const t413 = () => {
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
  const t414 = [];
  const t415 = t412(t413, t414);
  handlePrev = t415;
  let handleSubmit;
  if ($[39] !== handleSubmit) {
    $[39] = handleSubmit;
  } else {
  }
  let handleFieldChange;
  if ($[40] !== useCallback || $[41] !== currentStep || $[42] !== validateStep || $[43] !== formState || $[44] !== onSubmit || $[45] !== handleSubmit || $[46] !== handleFieldChange) {
    const t418 = useCallback;
    const t419 = async () => {
      const t1 = validateStep;
      const t3 = currentStep;
      const t4 = t1(t3);
      const t5 = !t4;
      if (t5) {
        const t6 = undefined;
        return t6;
      } else {
      }
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
    const t420 = currentStep;
    const t421 = validateStep;
    const t422 = formState;
    const t423 = t422.data;
    const t424 = onSubmit;
    const t425 = [t420, t421, t423, t424];
    const t426 = t418(t419, t425);
    handleSubmit = t426;
    $[40] = useCallback;
    $[41] = currentStep;
    $[42] = validateStep;
    $[43] = formState;
    $[44] = onSubmit;
    $[45] = handleSubmit;
    $[46] = handleFieldChange;
  } else {
  }
  const t429 = useCallback;
  const t430 = (name, value) => {
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
  const t431 = [];
  const t432 = t429(t430, t431);
  handleFieldChange = t432;
  let stepErrors;
  if ($[47] !== stepErrors) {
    $[47] = stepErrors;
  } else {
  }
  const t435 = useMemo;
  const t436 = () => {
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
  const t437 = step;
  const t438 = t437.fields;
  const t439 = formState;
  const t440 = t439.errors;
  const t441 = [t438, t440];
  const t442 = t435(t436, t441);
  stepErrors = t442;
  const t444 = "div";
  const t445 = "max-w-2xl mx-auto";
  const t446 = "div";
  const t447 = "mb-6";
  const t448 = "div";
  const t449 = "flex justify-between text-sm text-gray-500 mb-1";
  const t450 = "span";
  const t451 = "Step ";
  const t452 = currentStep;
  const t453 = 1;
  const t454 = t452 + t453;
  const t455 = " of ";
  const t456 = steps;
  const t457 = t456.length;
  const t458 = _jsxs(t450, { children: [t451, t454, t455, t457] });
  const t459 = "span";
  const t460 = completedFields;
  const t461 = t460.completed;
  const t462 = "/";
  const t463 = completedFields;
  const t464 = t463.total;
  const t465 = " fields completed";
  const t466 = _jsxs(t459, { children: [t461, t462, t464, t465] });
  const t467 = _jsxs(t448, { className: t449, children: [t458, t466] });
  const t468 = "div";
  const t469 = "w-full bg-gray-200 rounded h-2";
  const t470 = "div";
  const t471 = "bg-blue-600 rounded h-2 transition-all";
  const t472 = progress;
  const t473 = `${t472}%`;
  const t474 = { width: t473 };
  const t475 = _jsx(t470, { className: t471, style: t474 });
  const t476 = _jsx(t468, { className: t469, children: t475 });
  const t477 = _jsxs(t446, { className: t447, children: [t467, t476] });
  const t478 = "div";
  const t479 = "flex mb-8";
  const t480 = steps;
  const t481 = (s, i) => {
    const t2 = "div";
    const t4 = i;
    const t5 = "flex-1 flex items-center";
    const t6 = "div";
    const t8 = i;
    const t10 = currentStep;
    const t11 = t8 < t10;
    let t12;
    if (t11) {
      const t14 = "bg-green-500 text-white";
      t12 = t14;
    } else {
      const t17 = i;
      const t19 = currentStep;
      const t20 = t17 === t19;
      let t21;
      if (t20) {
        const t23 = "bg-blue-600 text-white";
        t21 = t23;
      } else {
        const t25 = "bg-gray-200 text-gray-500";
        t21 = t25;
      }
      t12 = t21;
    }
    const t28 = `w-8 h-8 rounded-full flex items-center justify-center text-sm ${t12}`;
    const t30 = i;
    const t32 = currentStep;
    const t33 = t30 < t32;
    let t34;
    if (t33) {
      const t36 = "✓";
      t34 = t36;
    } else {
      const t39 = i;
      const t40 = 1;
      const t41 = t39 + t40;
      t34 = t41;
    }
    const t43 = _jsx(t6, { className: t28, children: t34 });
    const t44 = "span";
    const t46 = i;
    const t48 = currentStep;
    const t49 = t46 === t48;
    let t50;
    if (t49) {
      const t52 = "font-semibold";
      t50 = t52;
    } else {
      const t54 = "text-gray-400";
      t50 = t54;
    }
    const t56 = `ml-2 text-sm ${t50}`;
    const t58 = s;
    const t59 = t58.title;
    const t60 = _jsx(t44, { className: t56, children: t59 });
    let t61;
    const t64 = i;
    const t66 = steps;
    const t67 = t66.length;
    const t68 = 1;
    const t69 = t67 - t68;
    const t70 = t64 < t69;
    t61 = t70;
    const t72 = "div";
    const t73 = "flex-1 h-px bg-gray-200 mx-4";
    const t74 = _jsx(t72, { className: t73 });
    t61 = t74;
    const t76 = _jsxs(t2, { key: t4, className: t5, children: [t43, t60, t61] });
    return t76;
  };
  const t482 = t480.map(t481);
  const t483 = _jsx(t478, { className: t479, children: t482 });
  const t484 = "div";
  const t485 = "bg-white border rounded-lg p-6";
  const t486 = "h2";
  const t487 = "text-xl font-semibold mb-1";
  const t488 = step;
  const t489 = t488.title;
  const t490 = _jsx(t486, { className: t487, children: t489 });
  const t491 = "p";
  const t492 = "text-sm text-gray-500 mb-6";
  const t493 = step;
  const t494 = t493.description;
  const t495 = _jsx(t491, { className: t492, children: t494 });
  let t244;
  if ($[48] !== stepErrors || $[49] !== t244 || $[50] !== onCancel || $[51] !== t270 || $[52] !== onCancel || $[53] !== isFirstStep || $[54] !== t285 || $[55] !== handlePrev || $[56] !== isLastStep || $[57] !== useMemo || $[58] !== step || $[59] !== formState || $[60] !== stepErrors || $[61] !== stepErrors) {
    const t497 = stepErrors;
    const t498 = 0;
    const t499 = t497 > t498;
    t244 = t499;
    $[48] = stepErrors;
    $[49] = t244;
    $[50] = onCancel;
    $[51] = t270;
    $[52] = onCancel;
    $[53] = isFirstStep;
    $[54] = t285;
    $[55] = handlePrev;
    $[56] = isLastStep;
    $[57] = useMemo;
    $[58] = step;
    $[59] = formState;
    $[60] = stepErrors;
    $[61] = stepErrors;
  } else {
  }
  const t597 = "div";
  const t598 = "bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm mb-4";
  const t599 = stepErrors;
  const t600 = " field(s) have errors\n          ";
  const t601 = _jsxs(t597, { className: t598, children: [t599, t600] });
  t244 = t601;
  let t568;
  if ($[62] !== t270 || $[63] !== handleSubmit || $[64] !== submitting || $[65] !== submitting || $[66] !== handleNext || $[67] !== t300 || $[68] !== t285 || $[69] !== t310 || $[70] !== currentStep || $[71] !== steps || $[72] !== completedFields || $[73] !== completedFields || $[74] !== progress || $[75] !== steps || $[76] !== step || $[77] !== step || $[78] !== step || $[79] !== t244) {
    const t507 = "div";
    const t508 = "space-y-4";
    const t509 = step;
    const t510 = t509.fields;
    const t511 = (field) => {
      let value;
      let t3;
      const t6 = formState;
      const t7 = t6.data;
      const t9 = field;
      const t10 = t9.name;
      const t11 = t7[t10];
      t3 = t11;
      const t13 = "";
      t3 = t13;
      value = t3;
      let error;
      const t19 = formState;
      const t20 = t19.errors;
      const t22 = field;
      const t23 = t22.name;
      const t24 = t20[t23];
      error = t24;
      let touched;
      const t29 = formState;
      const t30 = t29.touched;
      const t32 = field;
      const t33 = t32.name;
      const t34 = t30.has(t33);
      touched = t34;
      const t36 = "div";
      const t38 = field;
      const t39 = t38.name;
      const t40 = "label";
      const t41 = "block text-sm font-medium mb-1";
      const t43 = field;
      const t44 = t43.label;
      let t45;
      const t48 = field;
      const t49 = t48.required;
      t45 = t49;
      const t51 = "span";
      const t52 = "text-red-500 ml-1";
      const t53 = "*";
      const t54 = _jsx(t51, { className: t52, children: t53 });
      t45 = t54;
      const t56 = _jsxs(t40, { className: t41, children: [t44, t45] });
      const t58 = field;
      const t59 = t58.type;
      const t60 = "textarea";
      const t61 = t59 === t60;
      let t62;
      if (t61) {
        const t64 = "textarea";
        const t66 = value;
        const t67 = (e) => {
          const t2 = handleFieldChange;
          const t4 = field;
          const t5 = t4.name;
          const t7 = e;
          const t8 = t7.target;
          const t9 = t8.value;
          const t10 = t2(t5, t9);
          return t10;
        };
        let t68;
        const t71 = error;
        t68 = t71;
        const t74 = touched;
        t68 = t74;
        let t76;
        if (t68) {
          const t78 = "border-red-500";
          t76 = t78;
        } else {
          const t80 = "";
          t76 = t80;
        }
        const t82 = `w-full border rounded px-3 py-2 ${t76}`;
        const t83 = 3;
        const t84 = _jsx(t64, { value: t66, onChange: t67, className: t82, rows: t83 });
        t62 = t84;
      } else {
        const t87 = field;
        const t88 = t87.type;
        const t89 = "select";
        const t90 = t88 === t89;
        let t91;
        if (t90) {
          const t93 = "select";
          const t95 = value;
          const t96 = (e) => {
            const t2 = handleFieldChange;
            const t4 = field;
            const t5 = t4.name;
            const t7 = e;
            const t8 = t7.target;
            const t9 = t8.value;
            const t10 = t2(t5, t9);
            return t10;
          };
          let t97;
          const t100 = error;
          t97 = t100;
          const t103 = touched;
          t97 = t103;
          let t105;
          if (t97) {
            const t107 = "border-red-500";
            t105 = t107;
          } else {
            const t109 = "";
            t105 = t109;
          }
          const t111 = `w-full border rounded px-3 py-2 ${t105}`;
          const t112 = "option";
          const t113 = "";
          const t114 = "Select...";
          const t115 = _jsx(t112, { value: t113, children: t114 });
          const t117 = field;
          const t118 = t117.options;
          const t119 = (opt) => {
            const t1 = "option";
            const t3 = opt;
            const t5 = opt;
            const t7 = opt;
            const t8 = _jsx(t1, { key: t3, value: t5, children: t7 });
            return t8;
          };
          const t120 = t118.map(t119);
          const t121 = _jsxs(t93, { value: t95, onChange: t96, className: t111, children: [t115, t120] });
          t91 = t121;
        } else {
          const t124 = field;
          const t125 = t124.type;
          const t126 = "checkbox";
          const t127 = t125 === t126;
          let t128;
          if (t127) {
            const t130 = "input";
            const t131 = "checkbox";
            const t133 = value;
            const t134 = "true";
            const t135 = t133 === t134;
            const t136 = (e) => {
              const t2 = handleFieldChange;
              const t4 = field;
              const t5 = t4.name;
              const t7 = e;
              const t8 = t7.target;
              const t9 = t8.checked;
              let t10;
              if (t9) {
                const t12 = "true";
                t10 = t12;
              } else {
                const t14 = "false";
                t10 = t14;
              }
              const t16 = t2(t5, t10);
              return t16;
            };
            const t137 = _jsx(t130, { type: t131, checked: t135, onChange: t136 });
            t128 = t137;
          } else {
            const t139 = "input";
            const t141 = field;
            const t142 = t141.type;
            const t144 = value;
            const t145 = (e) => {
              const t2 = handleFieldChange;
              const t4 = field;
              const t5 = t4.name;
              const t7 = e;
              const t8 = t7.target;
              const t9 = t8.value;
              const t10 = t2(t5, t9);
              return t10;
            };
            let t146;
            const t149 = error;
            t146 = t149;
            const t152 = touched;
            t146 = t152;
            let t154;
            if (t146) {
              const t156 = "border-red-500";
              t154 = t156;
            } else {
              const t158 = "";
              t154 = t158;
            }
            const t160 = `w-full border rounded px-3 py-2 ${t154}`;
            const t161 = _jsx(t139, { type: t142, value: t144, onChange: t145, className: t160 });
            t128 = t161;
          }
          t91 = t128;
        }
        t62 = t91;
      }
      let t165;
      let t167;
      const t170 = error;
      t167 = t170;
      const t173 = touched;
      t167 = t173;
      t165 = t167;
      const t176 = "p";
      const t177 = "text-red-500 text-xs mt-1";
      const t179 = error;
      const t180 = _jsx(t176, { className: t177, children: t179 });
      t165 = t180;
      const t182 = _jsxs(t36, { key: t39, children: [t56, t62, t165] });
      return t182;
    };
    const t512 = t510.map(t511);
    const t513 = _jsx(t507, { className: t508, children: t512 });
    const t514 = _jsxs(t484, { className: t485, children: [t490, t495, t244, t513] });
    const t515 = "div";
    const t516 = "flex justify-between mt-6";
    const t517 = "div";
    $[80] = t568;
    $[62] = t270;
    $[63] = handleSubmit;
    $[64] = submitting;
    $[65] = submitting;
    $[66] = handleNext;
    $[67] = t300;
    $[68] = t285;
    $[69] = t310;
    $[70] = currentStep;
    $[71] = steps;
    $[72] = completedFields;
    $[73] = completedFields;
    $[74] = progress;
    $[75] = steps;
    $[76] = step;
    $[77] = step;
    $[78] = step;
    $[79] = t244;
  } else {
    t568 = $[80];
  }
  let t270;
  const t519 = onCancel;
  t270 = t519;
  const t591 = "button";
  const t592 = onCancel;
  const t593 = "text-gray-500 hover:text-gray-700";
  const t594 = "\n              Cancel\n            ";
  const t595 = _jsx(t591, { onClick: t592, className: t593, children: t594 });
  t270 = t595;
  const t527 = _jsx(t517, { children: t270 });
  const t528 = "div";
  const t529 = "flex gap-3";
  let t285;
  const t531 = isFirstStep;
  const t532 = !t531;
  t285 = t532;
  const t585 = "button";
  const t586 = handlePrev;
  const t587 = "px-4 py-2 border rounded";
  const t588 = "\n              Back\n            ";
  const t589 = _jsx(t585, { onClick: t586, className: t587, children: t588 });
  t285 = t589;
  const t540 = isLastStep;
  let t300;
  if (t540) {
    const t569 = "button";
    const t570 = handleSubmit;
    const t571 = submitting;
    const t572 = "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50";
    const t573 = submitting;
    let t310;
    if (t573) {
      const t583 = "Submitting...";
      t310 = t583;
    } else {
      const t575 = "Submit";
      t310 = t575;
    }
    const t581 = _jsx(t569, { onClick: t570, disabled: t571, className: t572, children: t310 });
    t300 = t581;
  } else {
    const t542 = "button";
    const t543 = handleNext;
    const t544 = "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700";
    const t545 = "\n              Next\n            ";
    const t546 = _jsx(t542, { onClick: t543, className: t544, children: t545 });
    t300 = t546;
  }
  const t566 = _jsxs(t528, { className: t529, children: [t285, t300] });
  const t567 = _jsxs(t515, { className: t516, children: [t527, t566] });
  t568 = _jsxs(t444, { className: t445, children: [t477, t483, t514, t567] });
  return t568;
}

