// Run with: node reference/designs/verify.cjs
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { generate, validate } = require('./capital-layout.js');

for (const family of ['A', 'B', 'C', 'mixed']) {
  const signatures = new Set();
  for (let seed = 0; seed < 1000; seed++) {
    const plan = generate(String(seed), family);
    assert.deepEqual(plan, generate(String(seed), family));
    assert.deepEqual(validate(plan), [], `${family}/${seed}`);
    assert.ok(plan.attempt < 64);
    assert.equal(plan.buildings.filter(b => b.role.includes('tower')).length,
      { A: 0, B: 2, C: 4 }[plan.family]);
    assert.ok(plan.path.every(p => p.w >= 2 && p.d >= 0));
    const { seed: ignoredSeed, attempt, ...composition } = plan;
    signatures.add(JSON.stringify(composition));
  }
  assert.ok(signatures.size > 500, `${family} composition diversity`);
}
for (const name of fs.readdirSync(__dirname)) {
  const text = fs.readFileSync(path.join(__dirname, name), 'utf8');
  if (name.endsWith('.js')) new vm.Script(text, { filename: name });
  if (name.endsWith('.html')) {
    for (const [, local] of text.matchAll(/(?:src|href)="([^":]+)"/g)) {
      assert.ok(fs.existsSync(path.join(__dirname, local)), `${name}: ${local}`);
    }
  }
}
console.log('4,000 valid deterministic castle layouts; script syntax and local links passed.');
