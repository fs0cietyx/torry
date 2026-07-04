import { createRequire } from 'node:module';
import os from 'node:os';

const require = createRequire(import.meta.url);
const platform = os.platform();
const arch = os.arch();

let binding;

try {
  if (platform === 'darwin' && arch === 'arm64') {
    binding = require('@torry/binding-darwin-arm64');
  } else if (platform === 'darwin' && arch === 'x64') {
    binding = require('@torry/binding-darwin-x64');
  } else if (platform === 'linux' && arch === 'x64') {
    binding = require('@torry/binding-linux-x64-gnu');
  } else if (platform === 'linux' && arch === 'arm64') {
    binding = require('@torry/binding-linux-arm64-gnu');
  } else if (platform === 'win32' && arch === 'x64') {
    binding = require('@torry/binding-win32-x64-msvc');
  } else {
    throw new Error(`Unsupported OS: ${platform}, architecture: ${arch}`);
  }
} catch (e) {
  // Local fallback for local development
  try {
    if (platform === 'darwin' && arch === 'arm64') {
      binding = require('./binding.darwin-arm64.node');
    } else if (platform === 'darwin' && arch === 'x64') {
      binding = require('./binding.darwin-x64.node');
    } else if (platform === 'linux' && arch === 'x64') {
      binding = require('./binding.linux-x64-gnu.node');
    } else if (platform === 'linux' && arch === 'arm64') {
      binding = require('./binding.linux-arm64-gnu.node');
    } else if (platform === 'win32' && arch === 'x64') {
      binding = require('./binding.win32-x64-msvc.node');
    } else {
      throw new Error(`Unsupported OS: ${platform}, architecture: ${arch}`);
    }
  } catch (err) {
    throw new Error(`Failed to load native binding: \n  - Npm package: ${e.message}\n  - Local fallback: ${err.message}`);
  }
}

export const RuntimeContext = binding.RuntimeContext;
