const fs = require("fs");
const path = require("path");

function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) {
      continue;
    }
    const key = arg.slice(2);
    const value = argv[i + 1];
    args[key] = value;
    i += 1;
  }
  return args;
}

function clearRequireCache(pkgDir) {
  const prefix = path.resolve(pkgDir) + path.sep;
  for (const key of Object.keys(require.cache)) {
    if (key.startsWith(prefix)) {
      delete require.cache[key];
    }
  }
}

function loadBudgets(pathToBudgets) {
  const raw = fs.readFileSync(pathToBudgets, "utf8");
  const payload = JSON.parse(raw);
  if (payload && payload.cases) {
    return payload.cases;
  }
  return payload;
}

function runCase(pkgDir, caseName) {
  clearRequireCache(pkgDir);
  const wasm = require(pkgDir);
  if (typeof wasm.run_memory_benchmark !== "function") {
    throw new Error("run_memory_benchmark export missing from wasm package");
  }
  if (typeof wasm.wasm_memory_bytes !== "function") {
    throw new Error("wasm_memory_bytes export missing from wasm package");
  }

  wasm.run_memory_benchmark(caseName);
  return wasm.wasm_memory_bytes();
}

function main() {
  const args = parseArgs(process.argv);
  const repoRoot = path.resolve(__dirname, "..");
  const pkgArg = args.pkg || path.join(repoRoot, "wasm", "pkg");
  const budgetsArg =
    args.budgets ||
    path.join(repoRoot, "benchmarks", "wasm_memory_budgets.json");
  const pkgDir = path.isAbsolute(pkgArg) ? pkgArg : path.resolve(repoRoot, pkgArg);
  const budgetsPath = path.isAbsolute(budgetsArg)
    ? budgetsArg
    : path.resolve(repoRoot, budgetsArg);

  if (!fs.existsSync(budgetsPath)) {
    console.error(`Budget file not found: ${budgetsPath}`);
    process.exit(1);
  }

  const cases = loadBudgets(budgetsPath);
  const failures = [];

  console.log("WASM memory budget check");
  console.log(`Package: ${pkgDir}`);
  console.log(`Budgets: ${budgetsPath}`);

  for (const [caseName, cfg] of Object.entries(cases)) {
    const cap =
      typeof cfg === "number" ? cfg : cfg.max_memory_bytes || cfg.max_bytes;
    if (!cap) {
      console.warn(`Skipping ${caseName}: missing max_memory_bytes cap`);
      continue;
    }

    const bytes = runCase(pkgDir, caseName);
    const status = bytes > cap ? "FAIL" : "PASS";
    console.log(
      `  ${caseName}: ${bytes} bytes / ${cap} bytes [${status}]`
    );
    if (bytes > cap) {
      failures.push(`${caseName}: ${bytes} > ${cap}`);
    }
  }

  if (failures.length > 0) {
    console.log("WASM memory failures:");
    for (const failure of failures) {
      console.log(`  - ${failure}`);
    }
    process.exit(1);
  }

  console.log("WASM memory budgets OK");
}

main();
