import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createHash, createPublicKey, randomBytes, verify } from "node:crypto";
import { spawnSync } from "node:child_process";
import { homedir, tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const root = resolve(dirname(scriptPath), "../..");
const configPath = join(root, "src-tauri/tauri.conf.json");
const cliPath = join(root, "node_modules/@tauri-apps/cli/tauri.js");
const privateName = "TAURI_SIGNING_PRIVATE_KEY";
const passwordName = "TAURI_SIGNING_PRIVATE_KEY_PASSWORD";

function decodeBase64(value, label) {
  const text = typeof value === "string" ? value.trim() : "";
  if (
    !text ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(
      text,
    )
  ) {
    throw new Error(`${label} must contain the original Tauri Base64 value.`);
  }
  const bytes = Buffer.from(text, "base64");
  if (bytes.toString("base64") !== text) {
    throw new Error(`${label} is not canonical Base64.`);
  }
  return bytes;
}

export function parsePublicKey(value) {
  const lines = decodeBase64(value, "plugins.updater.pubkey")
    .toString("utf8")
    .trimEnd()
    .split(/\r?\n/);
  if (lines.length !== 2 || !lines[0].startsWith("untrusted comment: ")) {
    throw new Error(
      "Invalid plugins.updater.pubkey: copy the .pub file contents directly; do not Base64-encode them again.",
    );
  }
  const packet = decodeBase64(lines[1], "Public key packet");
  if (packet.length !== 42 || packet.subarray(0, 2).toString() !== "Ed") {
    throw new Error("Invalid Tauri Ed25519 public key packet.");
  }
  return {
    id: packet.readBigUInt64LE(2).toString(16).toUpperCase().padStart(16, "0"),
    key: createPublicKey({
      // RFC 8410 SubjectPublicKeyInfo prefix for a 32-byte Ed25519 public key.
      key: Buffer.concat([
        Buffer.from("302a300506032b6570032100", "hex"),
        packet.subarray(10),
      ]),
      format: "der",
      type: "spki",
    }),
  };
}

export function verifySignature(publicKey, data, encodedSignature) {
  const { id, key } = parsePublicKey(publicKey);
  const lines = decodeBase64(encodedSignature, "Updater signature")
    .toString("utf8")
    .trimEnd()
    .split(/\r?\n/);
  if (
    lines.length !== 4 ||
    !lines[0].startsWith("untrusted comment: ") ||
    !lines[2].startsWith("trusted comment: ")
  ) {
    throw new Error("Invalid Tauri signature structure.");
  }
  const packet = decodeBase64(lines[1], "Signature packet");
  const globalSignature = decodeBase64(lines[3], "Comment signature");
  if (
    packet.length !== 74 ||
    globalSignature.length !== 64 ||
    packet.subarray(0, 2).toString() !== "ED"
  ) {
    throw new Error("Expected a prehashed Tauri Ed25519 signature.");
  }
  const signingId = packet
    .readBigUInt64LE(2)
    .toString(16)
    .toUpperCase()
    .padStart(16, "0");
  if (signingId !== id) {
    throw new Error(
      `Signing key ${signingId} does not match updater public key ${id}. Check secrets in the GitHub release environment.`,
    );
  }
  // Match minisign-verify (used by Tauri): verify the BLAKE2b-512 digest,
  // then the signature bytes concatenated with the trusted comment.
  const signature = packet.subarray(10);
  const digest = createHash("blake2b512").update(data).digest();
  const comment = Buffer.from(lines[2].slice("trusted comment: ".length));
  if (
    !verify(null, digest, key, signature) ||
    !verify(null, Buffer.concat([signature, comment]), key, globalSignature)
  ) {
    throw new Error("Updater signature verification failed.");
  }
  return id;
}

function runSigner(args, env, failure) {
  if (!existsSync(cliPath))
    throw new Error("Tauri CLI is missing. Run npm ci first.");
  const result = spawnSync(process.execPath, [cliPath, "signer", ...args], {
    cwd: root,
    env,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
    timeout: 120_000,
  });
  // The CLI can print key material. Never forward its output or arguments,
  // even on failure (generate receives its random password as an argument).
  if (result.error || result.status !== 0) throw new Error(failure);
}

export function checkCredentials(publicKey, env = process.env) {
  parsePublicKey(publicKey);
  let privateKey = env[privateName]?.trim();
  if (!privateKey)
    throw new Error(
      `${privateName} is missing. Set it in the GitHub release environment.`,
    );
  if (existsSync(privateKey))
    privateKey = readFileSync(privateKey, "utf8").trim();
  const signerEnv = {
    ...env,
    [privateName]: privateKey,
    [passwordName]: env[passwordName] ?? "",
  };
  delete signerEnv.TAURI_SIGNING_PRIVATE_KEY_PATH;
  const scratch = mkdtempSync(join(tmpdir(), "periscope-signing-check-"));
  try {
    const probe = join(scratch, "probe.txt");
    const data = randomBytes(64);
    writeFileSync(probe, data, { mode: 0o600 });
    runSigner(
      ["sign", probe],
      signerEnv,
      `Tauri could not sign the probe. Check ${privateName} and ${passwordName} in the GitHub release environment.`,
    );
    return verifySignature(
      publicKey,
      data,
      readFileSync(`${probe}.sig`, "utf8"),
    );
  } finally {
    // Only remove the unique directory created by this invocation in tmpdir.
    if (dirname(scratch) !== resolve(tmpdir()))
      throw new Error("Unexpected signing scratch path.");
    rmSync(scratch, { recursive: true, force: true });
  }
}

function isInside(parent, child) {
  const path = relative(parent, child);
  return (
    path === "" ||
    (!isAbsolute(path) &&
      path !== ".." &&
      !path.startsWith(`..${process.platform === "win32" ? "\\" : "/"}`))
  );
}

export function generateKeys({ outputDir, targetConfig = configPath }) {
  const original = readFileSync(targetConfig, "utf8");
  const config = JSON.parse(original);
  if (typeof config.plugins?.updater?.pubkey !== "string")
    throw new Error("Updater configuration is missing.");
  const output = resolve(outputDir);
  // Account for existing symlinks/junctions before creating any secret files.
  let ancestor = output;
  while (!existsSync(ancestor)) {
    const parent = dirname(ancestor);
    if (parent === ancestor) throw new Error("Output drive does not exist.");
    ancestor = parent;
  }
  const physicalOutput = resolve(
    realpathSync(ancestor),
    relative(ancestor, output),
  );
  if (isInside(realpathSync(root), physicalOutput))
    throw new Error(
      "Choose an output directory outside the repository for signing secrets.",
    );
  if (existsSync(output))
    throw new Error(
      "Output directory already exists. Existing keys will never be overwritten.",
    );
  mkdirSync(dirname(output), { recursive: true, mode: 0o700 });
  mkdirSync(output, { mode: 0o700 });
  const privatePath = join(output, `${privateName}.txt`);
  const passwordPath = join(output, `${passwordName}.txt`);
  const publicPath = join(output, "TAURI_SIGNING_PUBLIC_KEY.txt");
  const password = randomBytes(32).toString("base64url");
  writeFileSync(passwordPath, password, { flag: "wx", mode: 0o600 });
  runSigner(
    ["generate", "--ci", "--password", password, "--write-keys", privatePath],
    process.env,
    "Tauri key generation failed. The application config was not changed; retry with a new output directory.",
  );
  chmodSync(privatePath, 0o600);
  const publicKey = readFileSync(`${privatePath}.pub`, "utf8").trim();
  const id = checkCredentials(publicKey, {
    ...process.env,
    [privateName]: readFileSync(privatePath, "utf8").trim(),
    [passwordName]: password,
  });
  renameSync(`${privatePath}.pub`, publicPath);
  // Use the CLI's public key verbatim: it already contains the required Base64.
  config.plugins.updater.pubkey = publicKey;
  if (readFileSync(targetConfig, "utf8") !== original)
    throw new Error(
      "Configuration changed during generation; generated keys were saved but the config was not overwritten.",
    );
  writeFileSync(targetConfig, `${JSON.stringify(config, null, 2)}\n`, "utf8");
  return { output, id };
}

function main() {
  const [command, ...args] = process.argv.slice(2);
  if (
    command === "check" &&
    (args.length === 0 || (args.length === 1 && args[0] === "--public-only"))
  ) {
    const publicKey = JSON.parse(readFileSync(configPath, "utf8")).plugins
      ?.updater?.pubkey;
    const id = args.length
      ? parsePublicKey(publicKey).id
      : checkCredentials(publicKey);
    console.log(
      args.length
        ? `Updater public key format OK (${id}).`
        : `Signing probe verified against updater public key ${id}.`,
    );
  } else if (
    command === "generate" &&
    (args.length === 0 ||
      (args.length === 2 && args[0] === "--output-dir" && args[1]))
  ) {
    const outputDir =
      args[1] ??
      join(
        homedir(),
        ".tauri",
        "periscope",
        `signing-${new Date().toISOString().replaceAll(":", "-")}`,
      );
    const { output, id } = generateKeys({ outputDir });
    console.log(`Generated and verified updater key ${id}.`);
    console.log(
      "Updated src-tauri/tauri.conf.json with the matching public key.",
    );
    console.log(`Saved private files in: ${output}`);
    console.log(
      "GitHub > Settings > Environments > release > Environment secrets:",
    );
    console.log(`  ${privateName} = entire contents of ${privateName}.txt`);
    console.log(`  ${passwordName} = entire contents of ${passwordName}.txt`);
    console.log(
      "Paste file contents directly, without quotes or additional Base64 encoding.",
    );
    console.log(
      "Keep these files as a secure backup. Do not generate keys again for each release.",
    );
    console.log(
      "Key rotation: existing installations trusting the previous key must install the new release manually.",
    );
  } else {
    throw new Error(
      "Usage: npm run signing:generate [-- --output-dir <new-directory-outside-repo>] | npm run signing:check [-- --public-only]",
    );
  }
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  try {
    main();
  } catch (error) {
    console.error(`Signing setup failed: ${error.message}`);
    process.exitCode = 1;
  }
}
