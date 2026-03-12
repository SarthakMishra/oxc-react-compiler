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
  const $ = _c(12);
  const t307 = useState;
  const t308 = 0;
  const t309 = t307(t308);
  if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
    /* t310 = Discriminant(4) */
    /* t311 = Discriminant(4) */
  } else {
  }
  /* t312 = Discriminant(6) */
  const t313 = useReducer;
  const t314 = formReducer;
  const t315 = {};
  const t316 = {};
  /* t317 = Discriminant(30) */
  const t318 = new t317();
  const t319 = { data: t315, errors: t316, touched: t318 };
  const t320 = t313(t314, t319);
  if ($[1] === Symbol.for("react.memo_cache_sentinel")) {
    /* t321 = Discriminant(4) */
    /* t322 = Discriminant(4) */
  } else {
  }
  /* t323 = Discriminant(6) */
  const t324 = useState;
  const t325 = false;
  const t326 = t324(t325);
  if ($[2] === Symbol.for("react.memo_cache_sentinel")) {
    /* t327 = Discriminant(4) */
    /* t328 = Discriminant(4) */
  } else {
  }
  /* t329 = Discriminant(6) */
  if ($[3] === Symbol.for("react.memo_cache_sentinel")) {
    /* t330 = Discriminant(4) */
    const t331 = steps;
    const t332 = currentStep;
    /* t333 = Discriminant(20) */
    const step = t333;
    /* t335 = Discriminant(4) */
    const t336 = currentStep;
    const t337 = 0;
    const t338 = t336 === t337;
    const isFirstStep = t338;
    /* t340 = Discriminant(4) */
    const t341 = currentStep;
    const t342 = steps;
    const t343 = t342.length;
    const t344 = 1;
    const t345 = t343 - t344;
    const t346 = t341 === t345;
    const isLastStep = t346;
    /* t348 = Discriminant(4) */
  } else {
  }
  if ($[4] === Symbol.for("react.memo_cache_sentinel")) {
    const t349 = useMemo;
    /* t350 = Discriminant(28) */
    const t351 = currentStep;
    const t352 = steps;
    const t353 = t352.length;
    const t354 = [t351, t353];
    const t355 = t349(t350, t354);
    const progress = t355;
    /* t357 = Discriminant(4) */
  } else {
  }
  if ($[5] === Symbol.for("react.memo_cache_sentinel")) {
    const t358 = useMemo;
    /* t359 = Discriminant(28) */
    const t360 = steps;
    const t361 = formState;
    const t362 = t361.data;
    const t363 = [t360, t362];
    const t364 = t358(t359, t363);
    const completedFields = t364;
    /* t366 = Discriminant(4) */
  } else {
  }
  if ($[6] === Symbol.for("react.memo_cache_sentinel")) {
    const t367 = useCallback;
    /* t368 = Discriminant(28) */
    const t369 = steps;
    const t370 = formState;
    const t371 = t370.data;
    const t372 = [t369, t371];
    const t373 = t367(t368, t372);
    const validateStep = t373;
    /* t375 = Discriminant(4) */
  } else {
  }
  if ($[7] === Symbol.for("react.memo_cache_sentinel")) {
    const t376 = useCallback;
    /* t377 = Discriminant(28) */
    const t378 = currentStep;
    const t379 = validateStep;
    const t380 = steps;
    const t381 = t380.length;
    const t382 = [t378, t379, t381];
    const t383 = t376(t377, t382);
    const handleNext = t383;
    /* t385 = Discriminant(4) */
  } else {
  }
  const t386 = useCallback;
  /* t387 = Discriminant(28) */
  const t388 = [];
  const t389 = t386(t387, t388);
  const handlePrev = t389;
  if ($[8] === Symbol.for("react.memo_cache_sentinel")) {
    /* t391 = Discriminant(4) */
  } else {
  }
  if ($[9] === Symbol.for("react.memo_cache_sentinel")) {
    const t392 = useCallback;
    /* t393 = Discriminant(28) */
    const t394 = currentStep;
    const t395 = validateStep;
    const t396 = formState;
    const t397 = t396.data;
    const t398 = onSubmit;
    const t399 = [t394, t395, t397, t398];
    const t400 = t392(t393, t399);
    const handleSubmit = t400;
    /* t402 = Discriminant(4) */
  } else {
  }
  const t403 = useCallback;
  /* t404 = Discriminant(28) */
  const t405 = [];
  const t406 = t403(t404, t405);
  const handleFieldChange = t406;
  if ($[10] === Symbol.for("react.memo_cache_sentinel")) {
    /* t408 = Discriminant(4) */
  } else {
  }
  const t409 = useMemo;
  /* t410 = Discriminant(28) */
  const t411 = step;
  const t412 = t411.fields;
  const t413 = formState;
  const t414 = t413.errors;
  const t415 = [t412, t414];
  const t416 = t409(t410, t415);
  const stepErrors = t416;
  const t418 = "div";
  const t419 = "max-w-2xl mx-auto";
  const t420 = "div";
  const t421 = "mb-6";
  const t422 = "div";
  const t423 = "flex justify-between text-sm text-gray-500 mb-1";
  const t424 = "span";
  /* t425 = Discriminant(8) */
  const t426 = currentStep;
  const t427 = 1;
  const t428 = t426 + t427;
  /* t429 = Discriminant(8) */
  const t430 = steps;
  const t431 = t430.length;
  const t432 = <t424>{t425}{t428}{t429}{t431}</t424>;
  const t433 = "span";
  const t434 = completedFields;
  const t435 = t434.completed;
  /* t436 = Discriminant(8) */
  const t437 = completedFields;
  const t438 = t437.total;
  /* t439 = Discriminant(8) */
  const t440 = <t433>{t435}{t436}{t438}{t439}</t433>;
  const t441 = <t422 className={t423}>{t432}{t440}</t422>;
  const t442 = "div";
  const t443 = "w-full bg-gray-200 rounded h-2";
  const t444 = "div";
  const t445 = "bg-blue-600 rounded h-2 transition-all";
  const t446 = progress;
  const t447 = `${t446}%`;
  const t448 = { width: t447 };
  const t449 = <t444 className={t445} style={t448} />;
  const t450 = <t442 className={t443}>{t449}</t442>;
  const t451 = <t420 className={t421}>{t441}{t450}</t420>;
  const t452 = "div";
  const t453 = "flex mb-8";
  const t454 = steps;
  /* t455 = Discriminant(28) */
  const t456 = t454.map(t455);
  const t457 = <t452 className={t453}>{t456}</t452>;
  const t458 = "div";
  const t459 = "bg-white border rounded-lg p-6";
  const t460 = "h2";
  const t461 = "text-xl font-semibold mb-1";
  const t462 = step;
  const t463 = t462.title;
  const t464 = <t460 className={t461}>{t463}</t460>;
  const t465 = "p";
  const t466 = "text-sm text-gray-500 mb-6";
  const t467 = step;
  const t468 = t467.description;
  const t469 = <t465 className={t466}>{t468}</t465>;
  if ($[11] === Symbol.for("react.memo_cache_sentinel")) {
    const t470 = stepErrors;
  } else {
  }
  const t471 = 0;
}

