import init, { diff_files_jsonl_stream, diff_files_outcome_json, get_version } from "./wasm/excel_diff_wasm.js";

let initPromise = null;
let cachedVersion = null;

async function ensureInitialized() {
  if (!initPromise) {
    initPromise = (async () => {
      await init();
      cachedVersion = get_version();
      return cachedVersion;
    })();
  }
  return initPromise;
}

function postProgress(requestId, stage, detail) {
  self.postMessage({ type: "progress", requestId, stage, detail });
}

self.addEventListener("message", async (event) => {
  const msg = event.data || {};
  const requestId = msg.requestId;
  if (!msg.type) return;

  try {
    if (msg.type === "init") {
      postProgress(requestId, "init", "Initializing engine");
      const version = await ensureInitialized();
      self.postMessage({ type: "ready", requestId, version });
      return;
    }

    if (msg.type === "diff") {
      await ensureInitialized();
      postProgress(requestId, "diff", "Diffing workbooks");
      let oldBytes = new Uint8Array(msg.oldBuffer);
      let newBytes = new Uint8Array(msg.newBuffer);
      const optionsJson = JSON.stringify(msg.options || {});
      let json = diff_files_outcome_json(
        oldBytes,
        newBytes,
        msg.oldName || "old",
        msg.newName || "new",
        optionsJson
      );
      postProgress(requestId, "parse", "Parsing results");
      let payload = JSON.parse(json);
      self.postMessage({ type: "result", requestId, payload });
      oldBytes = null;
      newBytes = null;
      json = null;
      payload = null;
      return;
    }

    if (msg.type === "jsonl") {
      await ensureInitialized();
      postProgress(requestId, "diff", "Streaming JSONL output");
      let oldBytes = new Uint8Array(msg.oldBuffer);
      let newBytes = new Uint8Array(msg.newBuffer);
      const optionsJson = JSON.stringify(msg.options || {});
      const onChunk = chunk => {
        if (chunk) {
          self.postMessage({ type: "jsonl-chunk", requestId, chunk });
        }
      };
      diff_files_jsonl_stream(
        oldBytes,
        newBytes,
        msg.oldName || "old",
        msg.newName || "new",
        optionsJson,
        onChunk
      );
      self.postMessage({ type: "jsonl-done", requestId });
      oldBytes = null;
      newBytes = null;
      return;
    }
  } catch (err) {
    const message = err && err.message ? err.message : String(err);
    self.postMessage({ type: "error", requestId, message });
  }
});
