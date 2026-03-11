// Auto-generated NAPI binding loader
const { existsSync } = require('fs');
const { join } = require('path');

const localPath = join(__dirname, 'oxc-react-compiler.node');
if (!existsSync(localPath)) {
  throw new Error(`Native binding not found at ${localPath}. Run 'npx napi build --release' first.`);
}

module.exports = require(localPath);
