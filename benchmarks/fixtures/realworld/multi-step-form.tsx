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

export function MultiStepForm({ steps, onSubmit, onCancel }: MultiStepFormProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [formState, dispatch] = useReducer(formReducer, {
    data: {},
    errors: {},
    touched: new Set(),
  });
  const [submitting, setSubmitting] = useState(false);

  const step = steps[currentStep];
  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === steps.length - 1;

  const progress = useMemo(
    () => Math.round(((currentStep + 1) / steps.length) * 100),
    [currentStep, steps.length]
  );

  const completedFields = useMemo(() => {
    let completed = 0;
    let total = 0;
    for (const s of steps) {
      for (const field of s.fields) {
        total++;
        if (formState.data[field.name]) completed++;
      }
    }
    return { completed, total };
  }, [steps, formState.data]);

  const validateStep = useCallback(
    (stepIndex: number): boolean => {
      const stepToValidate = steps[stepIndex];
      const errors: FormErrors = {};
      let valid = true;

      for (const field of stepToValidate.fields) {
        const value = formState.data[field.name] || '';

        if (field.required && !value.trim()) {
          errors[field.name] = `${field.label} is required`;
          valid = false;
        } else if (field.validate) {
          const error = field.validate(value);
          if (error) {
            errors[field.name] = error;
            valid = false;
          }
        }
      }

      dispatch({ type: 'SET_ERRORS', errors });
      return valid;
    },
    [steps, formState.data]
  );

  const handleNext = useCallback(() => {
    if (validateStep(currentStep)) {
      setCurrentStep((s) => Math.min(s + 1, steps.length - 1));
    }
  }, [currentStep, validateStep, steps.length]);

  const handlePrev = useCallback(() => {
    setCurrentStep((s) => Math.max(s - 1, 0));
  }, []);

  const handleSubmit = useCallback(async () => {
    if (!validateStep(currentStep)) return;
    setSubmitting(true);
    onSubmit(formState.data);
    setSubmitting(false);
  }, [currentStep, validateStep, formState.data, onSubmit]);

  const handleFieldChange = useCallback(
    (name: string, value: string) => {
      dispatch({ type: 'SET_FIELD', name, value });
      dispatch({ type: 'CLEAR_ERROR', name });
    },
    []
  );

  const stepErrors = useMemo(
    () => step.fields.filter((f) => formState.errors[f.name]).length,
    [step.fields, formState.errors]
  );

  return (
    <div className="max-w-2xl mx-auto">
      {/* Progress bar */}
      <div className="mb-6">
        <div className="flex justify-between text-sm text-gray-500 mb-1">
          <span>Step {currentStep + 1} of {steps.length}</span>
          <span>{completedFields.completed}/{completedFields.total} fields completed</span>
        </div>
        <div className="w-full bg-gray-200 rounded h-2">
          <div
            className="bg-blue-600 rounded h-2 transition-all"
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>

      {/* Step indicators */}
      <div className="flex mb-8">
        {steps.map((s, i) => (
          <div key={i} className="flex-1 flex items-center">
            <div
              className={`w-8 h-8 rounded-full flex items-center justify-center text-sm ${
                i < currentStep ? 'bg-green-500 text-white' :
                i === currentStep ? 'bg-blue-600 text-white' :
                'bg-gray-200 text-gray-500'
              }`}
            >
              {i < currentStep ? '✓' : i + 1}
            </div>
            <span className={`ml-2 text-sm ${i === currentStep ? 'font-semibold' : 'text-gray-400'}`}>
              {s.title}
            </span>
            {i < steps.length - 1 && <div className="flex-1 h-px bg-gray-200 mx-4" />}
          </div>
        ))}
      </div>

      {/* Step content */}
      <div className="bg-white border rounded-lg p-6">
        <h2 className="text-xl font-semibold mb-1">{step.title}</h2>
        <p className="text-sm text-gray-500 mb-6">{step.description}</p>

        {stepErrors > 0 && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-3 py-2 rounded text-sm mb-4">
            {stepErrors} field(s) have errors
          </div>
        )}

        <div className="space-y-4">
          {step.fields.map((field) => {
            const value = formState.data[field.name] || '';
            const error = formState.errors[field.name];
            const touched = formState.touched.has(field.name);

            return (
              <div key={field.name}>
                <label className="block text-sm font-medium mb-1">
                  {field.label}
                  {field.required && <span className="text-red-500 ml-1">*</span>}
                </label>

                {field.type === 'textarea' ? (
                  <textarea
                    value={value}
                    onChange={(e) => handleFieldChange(field.name, e.target.value)}
                    className={`w-full border rounded px-3 py-2 ${error && touched ? 'border-red-500' : ''}`}
                    rows={3}
                  />
                ) : field.type === 'select' ? (
                  <select
                    value={value}
                    onChange={(e) => handleFieldChange(field.name, e.target.value)}
                    className={`w-full border rounded px-3 py-2 ${error && touched ? 'border-red-500' : ''}`}
                  >
                    <option value="">Select...</option>
                    {field.options?.map((opt) => (
                      <option key={opt} value={opt}>{opt}</option>
                    ))}
                  </select>
                ) : field.type === 'checkbox' ? (
                  <input
                    type="checkbox"
                    checked={value === 'true'}
                    onChange={(e) => handleFieldChange(field.name, e.target.checked ? 'true' : 'false')}
                  />
                ) : (
                  <input
                    type={field.type}
                    value={value}
                    onChange={(e) => handleFieldChange(field.name, e.target.value)}
                    className={`w-full border rounded px-3 py-2 ${error && touched ? 'border-red-500' : ''}`}
                  />
                )}

                {error && touched && (
                  <p className="text-red-500 text-xs mt-1">{error}</p>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Navigation */}
      <div className="flex justify-between mt-6">
        <div>
          {onCancel && (
            <button onClick={onCancel} className="text-gray-500 hover:text-gray-700">
              Cancel
            </button>
          )}
        </div>
        <div className="flex gap-3">
          {!isFirstStep && (
            <button onClick={handlePrev} className="px-4 py-2 border rounded">
              Back
            </button>
          )}
          {isLastStep ? (
            <button
              onClick={handleSubmit}
              disabled={submitting}
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
            >
              {submitting ? 'Submitting...' : 'Submit'}
            </button>
          ) : (
            <button onClick={handleNext} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
              Next
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
