const { existsSync, renameSync, chmodSync } = require("fs");
const { join } = require("path");
const { platform: _platform, arch: _arch } = require("os");
const { execSync } = require("child_process");

const binPath = join(__dirname, "..", "bin");
const platform = _platform();
const arch = _arch();

let executableName;

switch (platform) {
  case "win32":
    executableName = "releaser-win.exe";
    break;
  case "darwin":
    if (arch === "arm64") {
      executableName = "releaser-macos-arm64";
    } else {
      executableName = "releaser-macos-x64";
    }
    break;
  case "linux":
    executableName = "releaser-linux";
    break;
  default:
    console.error(`Unsupported platform: ${platform}`);
    process.exit(1);
}

const sourcePath = join(binPath, executableName);
const destPath = join(
  binPath,
  "releaser" + (platform === "win32" ? ".exe" : ""),
);

if (existsSync(sourcePath)) {
  renameSync(sourcePath, destPath);
  chmodSync(destPath, 0o755); // Make the file executable
  console.log(`Installed releaser for ${platform} (${arch})`);
} else {
  console.error(`Executable not found: ${executableName}`);
  process.exit(1);
}
