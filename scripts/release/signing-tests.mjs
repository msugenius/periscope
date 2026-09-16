import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import {
  checkCredentials,
  generateKeys,
  parsePublicKey,
  verifySignature,
} from "./signing.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const scratch = mkdtempSync(join(tmpdir(), "periscope-signing-tests-"));
const targetConfig = join(scratch, "tauri.conf.json");
const outputDir = join(scratch, "keys with spaces");
const initialConfig = {
  plugins: {
    updater: {
      pubkey: "old-key",
      endpoints: ["https://example.com/latest.json"],
    },
  },
};
let publicKey;
let signerEnv;
let data;
let signature;

before(() => {
  writeFileSync(targetConfig, JSON.stringify(initialConfig));
  const result = generateKeys({ outputDir, targetConfig });
  publicKey = readFileSync(
    join(outputDir, "TAURI_SIGNING_PUBLIC_KEY.txt"),
    "utf8",
  ).trim();
  signerEnv = {
    ...process.env,
    TAURI_SIGNING_PRIVATE_KEY: readFileSync(
      join(outputDir, "TAURI_SIGNING_PRIVATE_KEY.txt"),
      "utf8",
    ).trim(),
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: readFileSync(
      join(outputDir, "TAURI_SIGNING_PRIVATE_KEY_PASSWORD.txt"),
      "utf8",
    ),
  };
  delete signerEnv.TAURI_SIGNING_PRIVATE_KEY_PATH;
  assert.equal(result.id, parsePublicKey(publicKey).id);
  data = Buffer.from("Independent Tauri CLI signature fixture.\n");
  const probe = join(scratch, "fixture.txt");
  writeFileSync(probe, data);
  const signed = spawnSync(
    process.execPath,
    [
      join(root, "node_modules/@tauri-apps/cli/tauri.js"),
      "signer",
      "sign",
      probe,
    ],
    {
      cwd: root,
      env: signerEnv,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
      timeout: 120_000,
    },
  );
  assert.equal(
    signed.status,
    0,
    "Tauri CLI must successfully sign the fixture",
  );
  signature = readFileSync(`${probe}.sig`, "utf8");
});

after(() => {
  if (dirname(scratch) !== resolve(tmpdir()))
    throw new Error("Unexpected test scratch path.");
  rmSync(scratch, { recursive: true, force: true });
});

test("repository public key is correctly encoded", () => {
  const config = JSON.parse(
    readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"),
  );
  assert.match(
    parsePublicKey(config.plugins.updater.pubkey).id,
    /^[0-9A-F]{16}$/,
  );
});

test("generator saves exact secret names and commits only the public key to config", () => {
  const config = JSON.parse(readFileSync(targetConfig, "utf8"));
  assert.equal(config.plugins.updater.pubkey, publicKey);
  assert.deepEqual(
    config.plugins.updater.endpoints,
    initialConfig.plugins.updater.endpoints,
  );
  assert.equal(
    Buffer.from(signerEnv.TAURI_SIGNING_PRIVATE_KEY_PASSWORD, "base64url")
      .length,
    32,
  );
  assert.equal(
    readFileSync(targetConfig, "utf8").includes(
      signerEnv.TAURI_SIGNING_PRIVATE_KEY,
    ),
    false,
  );
  assert.equal(
    readFileSync(targetConfig, "utf8").includes(
      signerEnv.TAURI_SIGNING_PRIVATE_KEY_PASSWORD,
    ),
    false,
  );
});

test("real Tauri signature and encrypted credentials pass verification", () => {
  const expected = parsePublicKey(publicKey).id;
  assert.equal(verifySignature(publicKey, data, signature), expected);
  assert.equal(checkCredentials(publicKey, signerEnv), expected);
});

test("private-key file paths are supported like tauri build", () => {
  assert.equal(
    checkCredentials(publicKey, {
      ...signerEnv,
      TAURI_SIGNING_PRIVATE_KEY: join(
        outputDir,
        "TAURI_SIGNING_PRIVATE_KEY.txt",
      ),
    }),
    parsePublicKey(publicKey).id,
  );
});

test("double Base64, raw public text and malformed packets are rejected", () => {
  assert.throws(
    () => parsePublicKey(Buffer.from(publicKey).toString("base64")),
    /do not Base64-encode/,
  );
  assert.throws(
    () => parsePublicKey(Buffer.from(publicKey, "base64").toString()),
    /Base64/,
  );
  assert.throws(() => parsePublicKey("!invalid!"), /Base64/);
  const truncated = Buffer.from("untrusted comment: key\nRWQ=\n").toString(
    "base64",
  );
  assert.throws(() => parsePublicKey(truncated), /packet/);
});

test("changed data, signature and trusted comment cannot pass verification", () => {
  assert.throws(
    () => verifySignature(publicKey, Buffer.from("tampered"), signature),
    /verification failed/,
  );
  const lines = Buffer.from(signature, "base64")
    .toString()
    .trimEnd()
    .split(/\r?\n/);
  const tamperedPacket = Buffer.from(lines[1], "base64");
  tamperedPacket[10] ^= 1;
  const changedSignature = [...lines];
  changedSignature[1] = tamperedPacket.toString("base64");
  assert.throws(
    () =>
      verifySignature(
        publicKey,
        data,
        Buffer.from(changedSignature.join("\n")).toString("base64"),
      ),
    /verification failed/,
  );
  lines[2] += " altered";
  assert.throws(
    () =>
      verifySignature(
        publicKey,
        data,
        Buffer.from(lines.join("\n")).toString("base64"),
      ),
    /verification failed/,
  );
});

test("wrong key IDs and different public keys with the same ID are rejected", () => {
  const lines = Buffer.from(publicKey, "base64")
    .toString()
    .trimEnd()
    .split(/\r?\n/);
  const packet = Buffer.from(lines[1], "base64");
  packet[2] ^= 1;
  lines[1] = packet.toString("base64");
  assert.throws(
    () =>
      verifySignature(
        Buffer.from(lines.join("\n")).toString("base64"),
        data,
        signature,
      ),
    /does not match/,
  );
  packet[2] ^= 1;
  packet[10] ^= 1;
  lines[1] = packet.toString("base64");
  assert.throws(
    () =>
      verifySignature(
        Buffer.from(lines.join("\n")).toString("base64"),
        data,
        signature,
      ),
    /verification failed/,
  );
});

test("missing key and wrong password fail without disclosing secrets", () => {
  assert.throws(
    () => checkCredentials(publicKey, {}),
    /TAURI_SIGNING_PRIVATE_KEY is missing/,
  );
  const wrongPassword = "incorrect-password-do-not-print";
  assert.throws(
    () =>
      checkCredentials(publicKey, {
        ...signerEnv,
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD: wrongPassword,
      }),
    (error) => {
      assert.match(error.message, /Tauri could not sign/);
      assert.equal(error.message.includes(wrongPassword), false);
      assert.equal(
        error.message.includes(signerEnv.TAURI_SIGNING_PRIVATE_KEY),
        false,
      );
      return true;
    },
  );
});

test("existing keys and config survive a repeated generation attempt", () => {
  const before = readFileSync(targetConfig, "utf8");
  assert.throws(
    () => generateKeys({ outputDir, targetConfig }),
    /never be overwritten/,
  );
  assert.equal(readFileSync(targetConfig, "utf8"), before);
  assert.equal(
    readFileSync(
      join(outputDir, "TAURI_SIGNING_PRIVATE_KEY.txt"),
      "utf8",
    ).trim() === signerEnv.TAURI_SIGNING_PRIVATE_KEY,
    true,
  );
});

test("generator refuses to save secrets inside the repository", () => {
  assert.throws(
    () =>
      generateKeys({
        outputDir: join(root, "signing-test-secrets"),
        targetConfig,
      }),
    /outside the repository/,
  );
});

test("preflight rejects a different real signing key", () => {
  const config = JSON.parse(
    readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"),
  );
  assert.throws(
    () => checkCredentials(config.plugins.updater.pubkey, signerEnv),
    /does not match/,
  );
});

test(
  "PowerShell entry point forwards errors without changing the config",
  { skip: process.platform !== "win32" },
  () => {
    const config = join(root, "src-tauri/tauri.conf.json");
    const original = readFileSync(config, "utf8");
    const result = spawnSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        join(root, "scripts/release/New-UpdaterSigningKey.ps1"),
        "-OutputDirectory",
        join(root, "signing-test-secrets"),
      ],
      {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        windowsHide: true,
        timeout: 30_000,
      },
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /outside the repository/);
    assert.equal(readFileSync(config, "utf8"), original);
  },
);
