import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const frontendRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const provenancePath = path.join(frontendRoot, '.appfw-package-provenance.json');
const packageJsonPath = path.join(frontendRoot, 'package.json');
const lockfilePath = path.join(frontendRoot, 'package-lock.json');
const aboutPath = path.join(frontendRoot, 'src/features/nexus-a0/AboutPage.tsx');
const mainPath = path.join(frontendRoot, 'src/main.tsx');
const frontendReadmePath = path.join(frontendRoot, 'README.md');
const productReadmePath = path.join(frontendRoot, '../README.md');

const errors = [];

function fail(message) {
  errors.push(message);
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function sha256File(filePath) {
  return createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function mustInclude(filePath, label, values) {
  if (!fs.existsSync(filePath)) {
    fail(`${label} is missing: ${path.relative(frontendRoot, filePath)}`);
    return;
  }
  const text = fs.readFileSync(filePath, 'utf8');
  for (const value of values) {
    if (!text.includes(value)) {
      fail(`${label} does not visibly include ${value}`);
    }
  }
}

const provenance = readJson(provenancePath);
const packageJson = readJson(packageJsonPath);
const lockfile = readJson(lockfilePath);
const serializedManifests = `${JSON.stringify(packageJson)}\n${JSON.stringify(lockfile)}`;

if (provenance.schema !== 'appfw.package_provenance@1') {
  fail(`unexpected provenance schema: ${provenance.schema}`);
}
if (provenance.product !== 'pds-nexus') {
  fail(`provenance.product must be pds-nexus`);
}
if (provenance.channel !== 'web') {
  fail(`provenance.channel must be web`);
}
if (provenance.consume_family !== 'B_IX') {
  fail(`provenance.consume_family must be B_IX`);
}
if (provenance.packed_from !== 'dd63f6127b2c6ae19bedcc41fdf70d0e370c996d') {
  fail(`provenance.packed_from must be dd63f6127b2c6ae19bedcc41fdf70d0e370c996d`);
}
const expectedForbidden = {
  pds_health_components_0_9_0: true,
  pds_health_native_0_2_0: true,
  appfw_ui_source_alias: true,
  nexus_prefixed_design_system_fork: true
};
for (const [key, value] of Object.entries(expectedForbidden)) {
  if (provenance.forbidden?.[key] !== value) {
    fail(`provenance.forbidden.${key} must be ${value}`);
  }
}

const expected = [
  {
    name: '@appfw/pds-health-components',
    version: '0.12.0',
    sha256: 'b5c655533e9b935df92dc348b05b1966226bb6532a323c37ccf77ea12272b9e0',
    vendor: 'vendor/appfw-pds-health-components-0.12.0.tgz',
    pin: 'file:vendor/appfw-pds-health-components-0.12.0.tgz'
  },
  {
    name: '@appfw/pds-ix-presentation-contract',
    version: '0.2.0',
    sha256: '3a0a3404e2186e87302af019c1eca5dee5a80de9d10e98383f3fdcfcae884364',
    vendor: 'vendor/appfw-pds-ix-presentation-contract-0.2.0.tgz',
    pin: 'file:vendor/appfw-pds-ix-presentation-contract-0.2.0.tgz'
  }
];

if (!Array.isArray(provenance.packages) || provenance.packages.length !== expected.length) {
  fail('provenance packages must be exactly the two B_IX consume identities');
}

for (const item of expected) {
  const recorded = (provenance.packages ?? []).find((entry) => entry.name === item.name);
  if (!recorded) {
    fail(`provenance missing ${item.name}`);
    continue;
  }
  for (const key of ['version', 'sha256', 'vendor', 'pin']) {
    if (recorded[key] !== item[key]) {
      fail(`provenance ${item.name}.${key} must be ${item[key]}`);
    }
  }

  const vendorPath = path.join(frontendRoot, item.vendor);
  if (!fs.existsSync(vendorPath)) {
    fail(`vendored archive missing: ${item.vendor}`);
    continue;
  }
  const digest = sha256File(vendorPath);
  if (digest !== item.sha256) {
    fail(`vendored ${item.vendor} sha256 ${digest} != ${item.sha256}`);
  }

  const pin = packageJson.dependencies?.[item.name];
  if (pin !== item.pin) {
    fail(`package.json pin for ${item.name} must be ${item.pin}`);
  }

  const lockEntry = lockfile.packages?.[`node_modules/${item.name}`];
  if (!lockEntry) {
    fail(`package-lock.json missing node_modules/${item.name}`);
  } else {
    if (lockEntry.version !== item.version) {
      fail(`package-lock.json ${item.name} version must be ${item.version}`);
    }
    if (lockEntry.resolved !== item.pin) {
      fail(`package-lock.json ${item.name} resolved must be ${item.pin}`);
    }
  }

  const installedManifest = path.join(frontendRoot, 'node_modules', item.name, 'package.json');
  if (fs.existsSync(installedManifest)) {
    const installed = readJson(installedManifest);
    if (installed.name !== item.name || installed.version !== item.version) {
      fail(`installed ${item.name} is ${installed.name}@${installed.version}`);
    }
  }
}

if (serializedManifests.includes('0.9.0')) {
  fail('0.9.0 must not appear in package.json or package-lock.json');
}
if (serializedManifests.includes('appfw_ui') || serializedManifests.includes('appfw-ui')) {
  fail('appfw_ui source alias must not appear in package.json or package-lock.json');
}
if (serializedManifests.includes('pds-health-native')) {
  fail('native package must not be pinned on Nexus Web');
}

mustInclude(aboutPath, 'AboutPage', [
  'About PDS Nexus Web',
  '@appfw/pds-health-components',
  '0.12.0',
  'b5c655533e9b935df92dc348b05b1966226bb6532a323c37ccf77ea12272b9e0',
  '@appfw/pds-ix-presentation-contract',
  '0.2.0',
  '3a0a3404e2186e87302af019c1eca5dee5a80de9d10e98383f3fdcfcae884364'
]);
mustInclude(mainPath, 'main.tsx About composition', [
  "from './features/nexus-a0/AboutPage'",
  'AboutPage',
  '#about'
]);
mustInclude(frontendReadmePath, 'frontend README', ['0.12.0', '0.2.0', '/#about']);
mustInclude(productReadmePath, 'product README', ['0.12.0', '0.2.0', '/#about']);

if (errors.length > 0) {
  for (const error of errors) {
    console.error(error);
  }
  process.exit(1);
}

console.log('PDS Nexus Web package provenance OK');
console.log('Consumed @appfw/pds-health-components@0.12.0 sha256 b5c655533e9b935df92dc348b05b1966226bb6532a323c37ccf77ea12272b9e0');
console.log('Consumed @appfw/pds-ix-presentation-contract@0.2.0 sha256 3a0a3404e2186e87302af019c1eca5dee5a80de9d10e98383f3fdcfcae884364');
