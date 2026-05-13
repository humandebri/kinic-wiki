// Where: workers/wiki-generator/tests/identity-pem.test.ts
// What: Identity PEM compatibility tests for Worker canister calls.
// Why: Worker secrets use icp-cli PKCS#8 exports, while the SDK supports SEC1 PEM directly.
import assert from "node:assert/strict";
import test from "node:test";
import { identityFromPem, secretKeyFromPkcs8Pem } from "../src/identity-pem.js";

const PKCS8_SECP256K1_PEM = `-----BEGIN PRIVATE KEY-----
MIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQgpMWIEVJaFjnP3I297Dqr
6YBqyUEme3F0CyAePSCzBr6hRANCAATIjRJhFTWsZNfBNYOgw08ONMo+adRfTBH+
CupS2EihSyL1uidh07FtUAzCgWX4Fa6vJWeups6jw8mvyb1K3JOk
-----END PRIVATE KEY-----`;

const SEC1_SECP256K1_PEM = `-----BEGIN EC PRIVATE KEY-----
MHQCAQEEIFoXT7c45m9hozzAHfVMbxgu8DeBl5k1EDzAQptCLn8HoAcGBSuBBAAK
oUQDQgAEgBJ1nXIk9SHIili5bPX5Z0BNSsjNhhtz1m50icx18jeAdsZbuYpcDUAr
krR8YbOlBtZAsomwu0to+Pzw+SCcWw==
-----END EC PRIVATE KEY-----`;

const PKCS8_PRIME256V1_PEM = `-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgKvWy3mAMmfpS6Zl8
O6Wjw1Oh0mg2PQ8fhx3+DMtBVv2hRANCAAT1zEXpasptlTKlFyw/d+WyCR/SBO3P
qxudSih85sKHHAMG/VMIQOt+kpChNRKugxvS9m04ZPzvlwehEPPl3dRM
-----END PRIVATE KEY-----`;

test("PKCS#8 secp256k1 PEM exported by icp-cli parses", () => {
  const secretKey = secretKeyFromPkcs8Pem(PKCS8_SECP256K1_PEM);
  assert.equal(secretKey.length, 32);
  assert.doesNotThrow(() => identityFromPem(PKCS8_SECP256K1_PEM));
});

test("SEC1 secp256k1 PEM remains supported", () => {
  assert.doesNotThrow(() => identityFromPem(SEC1_SECP256K1_PEM));
});

test("invalid or non-secp256k1 PEM is rejected", () => {
  assert.throws(() => identityFromPem("not pem"), /secp256k1 private key/);
  assert.throws(() => secretKeyFromPkcs8Pem(PKCS8_PRIME256V1_PEM), /secp256k1/);
  assert.throws(() => secretKeyFromPkcs8Pem(pkcs8WithoutSecretOctet(PKCS8_SECP256K1_PEM)), /32-byte/);
});

function pkcs8WithoutSecretOctet(pem: string): string {
  const lines = pem.trim().split(/\r?\n/);
  const bytes = Buffer.from(lines.slice(1, -1).join(""), "base64");
  const innerSecret = bytes.indexOf(Buffer.from([0x04, 0x20]));
  assert.notEqual(innerSecret, -1);
  bytes[innerSecret] = 0x05;
  return [lines[0], bytes.toString("base64"), lines.at(-1)].join("\n");
}
