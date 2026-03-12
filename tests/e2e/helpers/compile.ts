import { createRequire } from 'module';
import path from 'path';
import { fileURLToPath } from 'url';

const require = createRequire(import.meta.url);

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const napiPath = path.resolve(__dirname, '../../../napi/react-compiler');

interface TransformResult {
  code: string;
  transformed: boolean;
  sourceMap?: string;
}

interface NapiBinding {
  transformReactFile(
    source: string,
    filename: string,
    options?: Record<string, unknown> | null
  ): TransformResult;
}

let _binding: NapiBinding | null = null;

function getBinding(): NapiBinding {
  if (!_binding) {
    _binding = require(napiPath) as NapiBinding;
  }
  return _binding;
}

/**
 * Compile a React source file using the OXC React Compiler.
 * Returns the compiled code, or null if the compiler chose not to transform it.
 */
export function compileSource(source: string, filename = 'test.tsx'): string | null {
  const binding = getBinding();
  const result = binding.transformReactFile(source, filename);
  if (!result.transformed) {
    return null;
  }
  return result.code;
}
