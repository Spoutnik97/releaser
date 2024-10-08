import { existsSync, renameSync, chmodSync } from "fs";
import { join } from "path";
import { platform as _platform, arch as _arch } from "os";
import { execSync } from "child_process";

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

  // For macOS, we need to remove quarantine attribute
  if (platform === "darwin") {
    try {
      execSync(`xattr -d com.apple.quarantine "${destPath}"`);
    } catch (error) {
      console.warn(
        "Failed to remove quarantine attribute. You may need to allow the app in System Preferences > Security & Privacy.",
      );
    }
  }
} else {
  console.error(`Executable not found: ${executableName}`);
  process.exit(1);
}
