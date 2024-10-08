#!/usr/bin/env node

import { execFileSync } from "child_process";
import { join } from "path";

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
