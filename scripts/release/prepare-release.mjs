import { readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline/promises";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const powershell = process.platform === "win32" ? "powershell.exe" : "pwsh";
const versionFiles = [
  "package.json",
  "package-lock.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json",
];
const stableSemVer = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/;
const maxUint64 = 18_446_744_073_709_551_615n;

function run(command, args, capture = false) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const detail = capture ? (result.stderr || result.stdout).trim() : "";
    throw new Error(
      `${command} ${args.join(" ")} failed${detail ? `: ${detail}` : ""}`,
    );
  }
  return capture ? result.stdout.trim() : "";
}

function succeeds(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "ignore",
    shell: false,
  });
  return result.status === 0;
}

function runPackageManager(args) {
  if (process.env.npm_execpath) {
    run(process.execPath, [process.env.npm_execpath, ...args]);
  } else if (process.platform === "win32") {
    run("cmd.exe", ["/d", "/s", "/c", "npm", ...args]);
  } else {
    run("npm", args);
  }
}

function parseVersion(value) {
  const match = stableSemVer.exec(value);
  if (!match) {
    throw new Error(
      `Version '${value}' is not strict stable MAJOR.MINOR.PATCH SemVer.`,
    );
  }
  const parts = match.slice(1).map(BigInt);
  if (parts.some((part) => part > maxUint64)) {
    throw new Error(`Version '${value}' contains an identifier above uint64.`);
  }
  return parts;
}

function compareVersions(left, right) {
  const a = parseVersion(left);
  const b = parseVersion(right);
  for (let index = 0; index < 3; index += 1) {
    if (a[index] < b[index]) return -1;
    if (a[index] > b[index]) return 1;
  }
  return 0;
}

function incrementVersion(current, kind) {
  let [major, minor, patch] = parseVersion(current);
  if (kind === "major") [major, minor, patch] = [major + 1n, 0n, 0n];
  else if (kind === "minor") [minor, patch] = [minor + 1n, 0n];
  else if (kind === "patch") patch += 1n;
  else throw new Error(`Unsupported release increment '${kind}'.`);
  const next = `${major}.${minor}.${patch}`;
  parseVersion(next);
  return next;
}

function readJson(path) {
  return JSON.parse(readFileSync(join(root, path), "utf8"));
}

function writeJson(path, value) {
  writeFileSync(
    join(root, path),
    `${JSON.stringify(value, null, 2)}\n`,
    "utf8",
  );
}

function packageVersionFromToml(path, packageHeader) {
  const text = readFileSync(join(root, path), "utf8");
  const escapedHeader = packageHeader.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const block = text.match(
    new RegExp(`(?:^|\\n)${escapedHeader}\\r?\\n([\\s\\S]*?)(?=\\r?\\n\\[|$)`),
  );
  const version = block?.[1].match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) throw new Error(`Could not read the version from ${path}.`);
  return version;
}

function cargoLockVersion() {
  const text = readFileSync(join(root, "src-tauri/Cargo.lock"), "utf8");
  const match = text.match(
    /\[\[package\]\]\r?\nname = "periscope"\r?\nversion = "([^"]+)"/,
  );
  if (!match) throw new Error("Could not read periScope from Cargo.lock.");
  return match[1];
}

function declaredVersions() {
  return new Map([
    ["package.json", readJson("package.json").version],
    ["package-lock.json", readJson("package-lock.json").packages[""].version],
    [
      "src-tauri/Cargo.toml",
      packageVersionFromToml("src-tauri/Cargo.toml", "[package]"),
    ],
    ["src-tauri/Cargo.lock", cargoLockVersion()],
    [
      "src-tauri/tauri.conf.json",
      readJson("src-tauri/tauri.conf.json").version,
    ],
  ]);
}

function agreedVersion() {
  const declarations = declaredVersions();
  const values = new Set(declarations.values());
  if (values.size !== 1) {
    const detail = [...declarations]
      .map(([path, version]) => `  ${path}: ${version}`)
      .join("\n");
    throw new Error(`Version declarations do not agree:\n${detail}`);
  }
  const [version] = values;
  parseVersion(version);
  return version;
}

async function selectVersion(current) {
  const argument = process.argv[2]?.trim();
  if (argument) {
    return ["major", "minor", "patch"].includes(argument)
      ? incrementVersion(current, argument)
      : argument;
  }
  if (!process.stdin.isTTY) {
    throw new Error(
      "Pass patch, minor, major, or an explicit version in non-interactive mode.",
    );
  }

  const choices = {
    1: incrementVersion(current, "patch"),
    2: incrementVersion(current, "minor"),
    3: incrementVersion(current, "major"),
  };
  console.log(`Current version: ${current}`);
  console.log(`  1. Patch  ${choices[1]} (recommended)`);
  console.log(`  2. Minor  ${choices[2]}`);
  console.log(`  3. Major  ${choices[3]}`);
  console.log("  4. Custom stable version");
  const prompt = createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  try {
    const choice =
      (await prompt.question("Select release [1]: ")).trim() || "1";
    if (choice in choices) return choices[choice];
    if (choice === "4") return (await prompt.question("Version: ")).trim();
    throw new Error(`Unknown selection '${choice}'.`);
  } finally {
    prompt.close();
  }
}

function replaceSingle(text, pattern, replacement, path) {
  const matches = [
    ...text.matchAll(new RegExp(pattern.source, pattern.flags + "g")),
  ];
  if (matches.length !== 1) {
    throw new Error(
      `Expected one version declaration in ${path}; found ${matches.length}.`,
    );
  }
  return text.replace(pattern, replacement);
}

function updateVersions(version) {
  const packageJson = readJson("package.json");
  packageJson.version = version;
  writeJson("package.json", packageJson);

  const packageLock = readJson("package-lock.json");
  packageLock.version = version;
  packageLock.packages[""].version = version;
  writeJson("package-lock.json", packageLock);

  const tauriConfig = readJson("src-tauri/tauri.conf.json");
  tauriConfig.version = version;
  writeJson("src-tauri/tauri.conf.json", tauriConfig);

  const cargoTomlPath = "src-tauri/Cargo.toml";
  const cargoToml = readFileSync(join(root, cargoTomlPath), "utf8");
  writeFileSync(
    join(root, cargoTomlPath),
    replaceSingle(
      cargoToml,
      /(^\[package\]\r?\n(?:.*\r?\n)*?version\s*=\s*")[^"]+(".*$)/m,
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
      cargoTomlPath,
    ),
    "utf8",
  );

  const cargoLockPath = "src-tauri/Cargo.lock";
  const cargoLock = readFileSync(join(root, cargoLockPath), "utf8");
  writeFileSync(
    join(root, cargoLockPath),
    replaceSingle(
      cargoLock,
      /(\[\[package\]\]\r?\nname = "periscope"\r?\nversion = ")[^"]+(")/,
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
      cargoLockPath,
    ),
    "utf8",
  );
}

async function main() {
  const repositoryRoot = realpathSync(
    run("git", ["rev-parse", "--show-toplevel"], true),
  );
  if (repositoryRoot.toLowerCase() !== realpathSync(root).toLowerCase()) {
    throw new Error(`Run this script inside ${root}.`);
  }
  if (run("git", ["status", "--porcelain", "--untracked-files=all"], true)) {
    throw new Error("The worktree must be clean before preparing a release.");
  }
  const branch = run("git", ["branch", "--show-current"], true);
  if (branch !== "dev") {
    throw new Error(
      `Expected branch 'dev', but currently on '${branch || "detached HEAD"}'.`,
    );
  }

  const current = agreedVersion();
  const version = await selectVersion(current);
  parseVersion(version);
  if (compareVersions(version, current) <= 0) {
    throw new Error(
      `Release version ${version} must be greater than ${current}.`,
    );
  }
  const releaseBranch = `release/v${version}`;
  if (
    succeeds("git", [
      "show-ref",
      "--verify",
      "--quiet",
      `refs/heads/${releaseBranch}`,
    ]) ||
    succeeds("git", [
      "show-ref",
      "--verify",
      "--quiet",
      `refs/remotes/origin/${releaseBranch}`,
    ])
  ) {
    throw new Error(`Branch '${releaseBranch}' already exists.`);
  }

  console.log(`\nPreparing periScope ${version} on ${releaseBranch}…`);
  run("git", ["switch", "-c", releaseBranch]);
  updateVersions(version);
  if (agreedVersion() !== version) {
    throw new Error(
      "Updated version declarations failed agreement validation.",
    );
  }

  const changed = run("git", ["diff", "--name-only"], true)
    .split(/\r?\n/)
    .filter(Boolean)
    .sort();
  const expected = [...versionFiles].sort();
  if (JSON.stringify(changed) !== JSON.stringify(expected)) {
    throw new Error(
      `Unexpected changed files after version update:\n${changed.map((path) => `  ${path}`).join("\n")}`,
    );
  }

  run(powershell, [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    "scripts/release/Test-ReleasePipeline.ps1",
  ]);
  runPackageManager(["run", "format:check"]);
  runPackageManager(["run", "lint"]);
  runPackageManager(["run", "test:coverage"]);
  run("git", ["diff", "--check"]);

  run("git", ["add", "--", ...versionFiles]);
  run("git", ["commit", "-m", `chore(release): v${version}`]);

  console.log("\nRelease commit prepared successfully.");
  console.log(`Review: git show --stat --oneline HEAD`);
  console.log(`Push:   git push -u origin ${releaseBranch}`);
}

main().catch((error) => {
  console.error(`\nRelease preparation failed: ${error.message}`);
  process.exitCode = 1;
});
