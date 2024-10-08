#!/usr/bin/env node

const { execFileSync } = require("child_process");
const { join } = require("path");

const executablePath = join(__dirname, "bin", "releaser");

try {
  const result = execFileSync(executablePath, process.argv.slice(2), {
    encoding: "utf-8",
  });
  console.log(result);
} catch (error) {
  console.error("Error executing releaser:", error.message);
  process.exit(1);
}
