import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import ts from "typescript";

// Compile the actual pure dashboard modules; no browser or test dependency needed.
const output = mkdtempSync(join(tmpdir(), "netmon-dashboard-tests-"));
after(() => rmSync(output, { recursive: true, force: true }));
for (const name of ["history", "chart", "connection"]) {
  const source = readFileSync(new URL(`../src/lib/${name}.ts`, import.meta.url), "utf8");
  const result = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  });
  writeFileSync(join(output, `${name}.mjs`), result.outputText.replace('from "./history"', 'from "./history.mjs"'));
}
const history = await import(pathToFileURL(join(output, "history.mjs")));
const chart = await import(pathToFileURL(join(output, "chart.mjs")));
const { createListenerConnection } = await import(pathToFileURL(join(output, "connection.mjs")));
const now = 1_000_000;
function sample(sequence, timestamp = now, cpu = 10) {
  return { sequence, timestamp_ms: timestamp, network: { down: 1, up: 0 }, network_interface: "en0", network_interfaces: ["en0"], cpu_percent: cpu, memory: { used_bytes: 2, total_bytes: 4 }, memory_percent: 50, gpu_percent: null };
}
function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

test("reopening replaces old 10% history with the current 80% backend window", () => {
  const old = Array.from({ length: 60 }, (_, i) => sample(i + 1, now - 180_000 + i * 1000, 10));
  const remote = Array.from({ length: 60 }, (_, i) => sample(i + 181, now - 59_000 + i * 1000, 80));
  const result = history.reconcileHistory(old, remote, now);
  assert.equal(result.length, 60);
  assert.equal(result.reduce((total, s) => total + s.cpu_percent, 0) / result.length, 80);
  assert.equal(result.at(-1).sequence, 240);
});

test("events arriving while a history request is pending survive its response", async () => {
  const pending = deferred();
  let state;
  const controller = history.createHistoryController(() => pending.promise, (s) => { state = s; }, () => now);
  controller.receive(sample(60, now - 1000));
  const request = controller.synchronize();
  controller.receive(sample(61));
  pending.resolve({ samples: [sample(59, now - 2000), sample(60, now - 1000)], network_selection: "en0" });
  await request;
  assert.deepEqual(state.samples.map((s) => s.sequence), [59, 60, 61]);
  assert.equal(state.networkSelection, "en0");
});

test("out-of-order fetch completions and duplicate events cannot roll the view back", async () => {
  const first = deferred(), second = deferred();
  const requests = [first, second];
  let state;
  const controller = history.createHistoryController(() => requests.shift().promise, (s) => { state = s; }, () => now);
  const one = controller.synchronize(), two = controller.synchronize();
  second.resolve({ samples: [sample(20)], network_selection: "en0" });
  await two;
  first.resolve({ samples: [sample(10, now - 10000)], network_selection: null });
  await one;
  controller.receive(sample(20));
  controller.receive(sample(19, now - 1000));
  assert.equal(state.samples.at(-1).sequence, 20);
  assert.equal(state.samples.filter((s) => s.sequence === 20).length, 1);
  assert.equal(state.networkSelection, "en0");
});

test("resume removes samples older than 60 seconds before the new request completes", async () => {
  let clock = now;
  let state;
  const request = deferred();
  const controller = history.createHistoryController(() => request.promise, (s) => { state = s; }, () => clock);
  controller.receive(sample(1));
  clock += 180_000;
  const refresh = controller.synchronize();
  assert.deepEqual(state.samples, []);
  request.resolve({ samples: [], network_selection: null });
  await refresh;
  assert.deepEqual(state.samples, []);
});

test("history is bounded by time, count and safe sequence ordering", () => {
  const many = Array.from({ length: 120 }, (_, i) => sample(i, now - 119_000 + i * 1000));
  const result = history.mergeSamples([], many, now);
  assert.equal(result.length, 60);
  assert.equal(result[0].timestamp_ms, now - 59_000);
  const backwardsClock = history.mergeSamples([sample(1, now + 500)], [sample(2, now)], now);
  assert.deepEqual(backwardsClock.map((s) => s.sequence), [2]);
});

test("disposing prevents late asynchronous updates", async () => {
  const request = deferred();
  let publishes = 0;
  const controller = history.createHistoryController(() => request.promise, () => { publishes++; }, () => now);
  const fetch = controller.synchronize();
  controller.dispose();
  const before = publishes;
  request.resolve({ samples: [sample(1)], network_selection: null });
  await fetch;
  controller.receive(sample(2));
  assert.equal(publishes, before);
});

test("chart uses actual time positions, not a stretched sample count", () => {
  const points = [{ timestamp: now - 1000, value: 50 }, { timestamp: now, value: 50 }];
  assert.equal(chart.chartPath(points, now, 100), "M 295.0 49.0 L 300.0 49.0");
});

test("chart does not join missing data, pauses, or changed network interfaces", () => {
  const points = [
    { timestamp: now - 10_000, value: 10, source: "en0" },
    { timestamp: now - 9_000, value: null, source: "en0" },
    { timestamp: now - 8_000, value: 10, source: "en0" },
    { timestamp: now - 4_000, value: 10, source: "en0" },
    { timestamp: now - 3_000, value: 10, source: "utun0" },
    { timestamp: now - 2_000, value: 10, source: "utun0" },
  ];
  const path = chart.chartPath(points, now, 100);
  assert.equal((path.match(/M /g) ?? []).length, 4);
  assert.equal((path.match(/L /g) ?? []).length, 1);
});

test("unavailable GPU values are omitted from summaries, not treated as zero", () => {
  const points = [{ timestamp: now - 2000, value: null }, { timestamp: now - 1000, value: 80 }, { timestamp: now, value: NaN }];
  assert.equal(chart.average(points), 80);
  assert.equal(chart.peak(points), 80);
  assert.equal(chart.average([{ timestamp: now, value: null }]), null);
});

test("binary network and RAM units match their labels", () => {
  assert.equal(chart.formatSpeed(0), "0 B/s");
  assert.equal(chart.formatSpeed(1 / 1024), "1 KiB/s");
  assert.equal(chart.formatSpeed(1), "1.0 MiB/s");
  assert.equal(chart.formatSpeed(1024), "1.0 GiB/s");
  assert.equal(chart.formatGiB(8 * 1024 ** 3), "8.0 GiB");
  assert.equal(chart.formatSpeed(null), "—");
});


test("visible clock expires current values and history when backend events stop, without IPC", () => {
  let clock = now;
  let state;
  let calls = 0;
  let tick;
  let starts = 0, stops = 0;
  const controller = history.createHistoryController(async () => { calls++; return { samples: [], network_selection: null }; }, (s) => { state = s; }, () => clock);
  const visibleClock = history.createVisibilityClock(() => controller.advanceClock(), (run) => {
    starts++; tick = run;
    return () => { stops++; tick = undefined; };
  });
  controller.receive(sample(1));
  visibleClock.setVisible(true);
  visibleClock.setVisible(true);
  assert.equal(starts, 1);
  assert.equal(history.latestFresh(state.samples, state.now).sequence, 1);
  clock += history.STALE_MS + 1;
  tick();
  assert.equal(history.latestFresh(state.samples, state.now), null);
  clock += 60_000;
  tick();
  assert.deepEqual(state.samples, []);
  assert.equal(calls, 0);
  visibleClock.setVisible(false);
  assert.equal(stops, 1);
  assert.equal(tick, undefined);
  visibleClock.setVisible(true);
  assert.equal(starts, 2);
  visibleClock.dispose();
  assert.equal(stops, 2);
  visibleClock.setVisible(true);
  assert.equal(starts, 2);
});

test("failed listener setup remains visible across ticks and retries without duplicate subscriptions", async () => {
  let error = null;
  let fail = true;
  let firstCount = 0, secondCount = 0, cleanups = 0;
  const connection = createListenerConnection([
    async () => { firstCount++; return () => { cleanups++; }; },
    async () => { secondCount++; if (fail) throw new Error("unavailable"); return () => { cleanups++; }; },
  ], (message) => { error = message; });
  assert.equal(await connection.connect(), false);
  assert.match(error, /unavailable/);
  assert.equal(cleanups, 1);
  const controller = history.createHistoryController(async () => ({ samples: [], network_selection: null }), () => {}, () => now);
  controller.advanceClock();
  assert.match(error, /unavailable/);
  fail = false;
  assert.deepEqual(await Promise.all([connection.connect(), connection.connect()]), [true, true]);
  assert.equal(error, null);
  assert.equal(firstCount, 2);
  assert.equal(secondCount, 2);
  await connection.connect();
  assert.equal(firstCount, 2);
  connection.dispose();
  assert.equal(cleanups, 3);
});

test("disposal during listener setup removes the late subscription and prevents further setup", async () => {
  const registration = deferred();
  let cleanups = 0, secondCount = 0;
  const connection = createListenerConnection([
    () => registration.promise,
    async () => { secondCount++; return () => {}; },
  ], () => { assert.fail("disposed setup must not publish"); });
  const pending = connection.connect();
  connection.dispose();
  registration.resolve(() => { cleanups++; });
  assert.equal(await pending, false);
  assert.equal(cleanups, 1);
  assert.equal(secondCount, 0);
  assert.equal(await connection.connect(), false);
});
