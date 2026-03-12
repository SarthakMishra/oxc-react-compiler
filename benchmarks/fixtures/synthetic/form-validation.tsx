import { useState, useMemo, useCallback } from 'react';

interface FormState {
  email: string;
  password: string;
  confirmPassword: string;
}

export function FormValidation() {
  const [form, setForm] = useState<FormState>({
    email: '',
    password: '',
    confirmPassword: '',
  });
  const [submitted, setSubmitted] = useState(false);

  const errors = useMemo(() => {
    const errs: Record<string, string> = {};
    if (!form.email.includes('@')) {
      errs.email = 'Invalid email address';
    }
    if (form.password.length < 8) {
      errs.password = 'Password must be at least 8 characters';
    }
    if (form.password !== form.confirmPassword) {
      errs.confirmPassword = 'Passwords do not match';
    }
    return errs;
  }, [form]);

  const isValid = useMemo(() => Object.keys(errors).length === 0, [errors]);

  const handleChange = useCallback((field: keyof FormState) => (e: React.ChangeEvent<HTMLInputElement>) => {
    setForm((prev) => ({ ...prev, [field]: e.target.value }));
  }, []);

  const handleSubmit = useCallback(() => {
    setSubmitted(true);
    if (isValid) {
      console.log('Form submitted:', form);
    }
  }, [isValid, form]);

  return (
    <div>
      <h2>Sign Up</h2>
      <div>
        <input type="email" value={form.email} onChange={handleChange('email')} placeholder="Email" />
        {submitted && errors.email && <span className="error">{errors.email}</span>}
      </div>
      <div>
        <input type="password" value={form.password} onChange={handleChange('password')} placeholder="Password" />
        {submitted && errors.password && <span className="error">{errors.password}</span>}
      </div>
      <div>
        <input type="password" value={form.confirmPassword} onChange={handleChange('confirmPassword')} placeholder="Confirm Password" />
        {submitted && errors.confirmPassword && <span className="error">{errors.confirmPassword}</span>}
      </div>
      <button onClick={handleSubmit} disabled={submitted && !isValid}>
        {submitted && !isValid ? 'Fix errors' : 'Submit'}
      </button>
    </div>
  );
}
