#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

// Check if cargo is installed
const checkCargo = spawn('cargo', ['--version']);

checkCargo.on('error', (err) => {
  console.error('Error: "cargo" is not found in your PATH. Please install Rust and Cargo to run tbook.');
  process.exit(1);
});

checkCargo.on('exit', (code) => {
  if (code === 0) {
    const args = process.argv.slice(2);
    const child = spawn('cargo', ['run', '--release', '--', ...args], {
      cwd: path.join(__dirname, '..'),
      stdio: 'inherit'
    });

    child.on('exit', (exitCode) => {
      process.exit(exitCode);
    });
  }
});
