#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

function usage() {
  console.log(`Usage: scripts/ui_diff.js --scenario <name> [options]

Options:
  --baseline <path>   Baseline PNG path
  --current <path>    Current PNG path
  --diff <path>       Diff PNG output path
  --out <path>        Diff metrics JSON output path
  --threshold <num>   Threshold percent (default: 0.15)
  --update-baseline   Copy current -> baseline if baseline missing
`);
}

function parseArgs(argv) {
  const args = { updateBaseline: false };
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--update-baseline") {
      args.updateBaseline = true;
      continue;
    }
    if (arg === "-h" || arg === "--help") {
      args.help = true;
      continue;
    }
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

function readPngSize(filePath) {
  const fd = fs.openSync(filePath, "r");
  const buffer = Buffer.alloc(24);
  fs.readSync(fd, buffer, 0, 24, 0);
  fs.closeSync(fd);
  const signature = buffer.slice(0, 8).toString("hex");
  if (signature !== "89504e470d0a1a0a") {
    throw new Error(`Not a PNG file: ${filePath}`);
  }
  const width = buffer.readUInt32BE(16);
  const height = buffer.readUInt32BE(20);
  return { width, height };
}

function tryPixelmatch(baselinePath, currentPath, diffPath) {
  let pixelmatch;
  let PNG;
  try {
    pixelmatch = require("pixelmatch");
    PNG = require("pngjs").PNG;
  } catch (err) {
    return null;
  }

  const img1 = PNG.sync.read(fs.readFileSync(baselinePath));
  const img2 = PNG.sync.read(fs.readFileSync(currentPath));
  if (img1.width !== img2.width || img1.height !== img2.height) {
    throw new Error("Baseline and current images differ in size.");
  }
  const diff = new PNG({ width: img1.width, height: img1.height });
  const diffPixels = pixelmatch(
    img1.data,
    img2.data,
    diff.data,
    img1.width,
    img1.height,
    { threshold: 0.1 }
  );
  fs.writeFileSync(diffPath, PNG.sync.write(diff));
  return {
    engine: "pixelmatch",
    diffPixels,
    width: img1.width,
    height: img1.height,
  };
}

function diffWithImagemagick(baselinePath, currentPath, diffPath) {
  const size = readPngSize(baselinePath);
  const currentSize = readPngSize(currentPath);
  if (size.width !== currentSize.width || size.height !== currentSize.height) {
    throw new Error("Baseline and current images differ in size.");
  }

  const result = spawnSync("compare", ["-metric", "AE", baselinePath, currentPath, diffPath], {
    encoding: "utf8",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status === 2) {
    throw new Error(`ImageMagick compare failed: ${(result.stderr || "").trim()}`);
  }
  const stderr = (result.stderr || "").trim();
  const diffPixels = parseInt(stderr.split(/\s+/)[0], 10);
  if (Number.isNaN(diffPixels)) {
    throw new Error(`Unable to parse ImageMagick compare output: ${stderr}`);
  }
  return {
    engine: "imagemagick",
    diffPixels,
    width: size.width,
    height: size.height,
  };
}

function readBaselineMeta(baselinePath) {
  const dir = path.dirname(baselinePath);
  const metaPath = path.join(dir, "baseline.json");
  if (!fs.existsSync(metaPath)) {
    return null;
  }
  try {
    const content = fs.readFileSync(metaPath, "utf8");
    return JSON.parse(content);
  } catch (err) {
    return null;
  }
}

function readJsonIfExists(pathToFile) {
  if (!fs.existsSync(pathToFile)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(pathToFile, "utf8"));
  } catch (err) {
    return null;
  }
}

function writeBaselineMeta(baselinePath, thresholdPercent) {
  const dir = path.dirname(baselinePath);
  const metaPath = path.join(dir, "baseline.json");
  const payload = {
    baseline: path.basename(baselinePath),
    thresholdPercent,
    createdAtUnix: Math.floor(Date.now() / 1000),
  };
  fs.writeFileSync(metaPath, JSON.stringify(payload, null, 2));
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function main() {
  const args = parseArgs(process.argv);
  if (args.help) {
    usage();
    process.exit(0);
  }

  const scenario = args.scenario || args.s;
  if (!scenario && (!args.baseline || !args.current)) {
    usage();
    process.exit(1);
  }

  const root = process.cwd();
  const baseDir = scenario
    ? path.join(root, "desktop", "ui_snapshots", scenario)
    : root;

  const baselinePath = args.baseline || path.join(baseDir, "baseline.png");
  const currentPath = args.current || path.join(baseDir, "current.png");
  const diffPath = args.diff || path.join(baseDir, "diff.png");
  const outPath = args.out || path.join(baseDir, "diff.json");
  const currentMeta = readJsonIfExists(path.join(baseDir, "current.json")) || {};
  let readyMeta = {};
  if (currentMeta.readyFile && fs.existsSync(currentMeta.readyFile)) {
    readyMeta = readJsonIfExists(currentMeta.readyFile) || {};
  }

  const baselineMeta = readBaselineMeta(baselinePath);
  const thresholdPercent = args.threshold
    ? parseFloat(args.threshold)
    : baselineMeta?.thresholdPercent ?? 0.15;

  if (!fs.existsSync(currentPath)) {
    console.error(`Current image not found: ${currentPath}`);
    process.exit(1);
  }

  if (!fs.existsSync(baselinePath)) {
    if (args.updateBaseline) {
      ensureDir(baselinePath);
      fs.copyFileSync(currentPath, baselinePath);
      writeBaselineMeta(baselinePath, thresholdPercent);
      const payload = {
        scenario,
        baseline: baselinePath,
        current: currentPath,
        diff: diffPath,
        status: "baseline_created",
        thresholdPercent,
      };
      ensureDir(outPath);
      fs.writeFileSync(outPath, JSON.stringify(payload, null, 2));
      console.log("Baseline created.");
      process.exit(0);
    }
    console.error(`Baseline image not found: ${baselinePath}`);
    process.exit(1);
  }

  let result;
  try {
    result = tryPixelmatch(baselinePath, currentPath, diffPath);
    if (!result) {
      result = diffWithImagemagick(baselinePath, currentPath, diffPath);
    }
  } catch (err) {
    console.error(`Diff failed: ${err.message}`);
    process.exit(1);
  }

  const totalPixels = result.width * result.height;
  const mismatchPercent = totalPixels > 0 ? (result.diffPixels / totalPixels) * 100 : 0;
  const status = mismatchPercent <= thresholdPercent ? "pass" : "fail";

  const payload = {
    scenario,
    baseline: baselinePath,
    current: currentPath,
    diff: diffPath,
    engine: result.engine,
    width: result.width,
    height: result.height,
    pixelsDifferent: result.diffPixels,
    mismatchPercent: Number(mismatchPercent.toFixed(4)),
    thresholdPercent,
    status,
    tag: currentMeta.tag,
    windowSize: currentMeta.windowSize,
    expected_mode: readyMeta.expected_mode,
    actual_mode: readyMeta.actual_mode,
    ready_status: readyMeta.status,
    createdAtUnix: Math.floor(Date.now() / 1000),
  };

  ensureDir(outPath);
  fs.writeFileSync(outPath, JSON.stringify(payload, null, 2));

  console.log(`Diff ${status}: ${payload.mismatchPercent}% (threshold ${thresholdPercent}%)`);
  process.exit(status === "pass" ? 0 : 2);
}

main();
