export function createDiffWorkerClient({ onStatus } = {}) {
  let worker = null;
  let current = null;
  let requestCounter = 0;
  let readyPromise = null;
  let cachedVersion = null;
  let disposed = false;

  function nextRequestId() {
    requestCounter += 1;
    return requestCounter;
  }

  function notify(status) {
    if (typeof onStatus === "function") {
      onStatus(status);
    }
  }

  function resetWorker() {
    if (worker) {
      worker.terminate();
      worker = null;
    }
    current = null;
    readyPromise = null;
    cachedVersion = null;
  }

  function handleWorkerError(error) {
    const message = error && error.message ? error.message : String(error);
    if (current && current.reject) {
      current.reject(new Error(message));
      current = null;
    }
    resetWorker();
  }

  function handleMessage(event) {
    const msg = event.data || {};
    if (!current || msg.requestId !== current.id) return;

    if (msg.type === "progress") {
      notify({ stage: msg.stage, detail: msg.detail, source: "worker" });
      return;
    }

    if (msg.type === "ready") {
      cachedVersion = msg.version || "";
      const resolve = current.resolve;
      current = null;
      resolve(cachedVersion);
      return;
    }

    if (msg.type === "result") {
      const resolve = current.resolve;
      current = null;
      resolve(msg.payload);
      return;
    }

    if (msg.type === "jsonl-chunk") {
      if (current && current.kind === "jsonl") {
        current.chunks.push(msg.chunk || "");
      }
      return;
    }

    if (msg.type === "jsonl-done") {
      if (current && current.kind === "jsonl") {
        const resolve = current.resolve;
        const chunks = current.chunks || [];
        current = null;
        resolve(new Blob(chunks, { type: "application/x-ndjson" }));
      }
      return;
    }

    if (msg.type === "error") {
      const reject = current.reject;
      current = null;
      reject(new Error(msg.message || "Worker error"));
    }
  }

  function ensureWorker() {
    if (disposed) {
      throw new Error("Diff worker client disposed.");
    }
    if (!worker) {
      worker = new Worker(new URL("./diff_worker.js", import.meta.url), { type: "module" });
      worker.addEventListener("message", handleMessage);
      worker.addEventListener("error", handleWorkerError);
      worker.addEventListener("messageerror", handleWorkerError);
    }
    return worker;
  }

  async function ready() {
    if (cachedVersion) return cachedVersion;
    if (readyPromise) return readyPromise;
    const w = ensureWorker();
    const id = nextRequestId();
    readyPromise = new Promise((resolve, reject) => {
      current = { id, resolve, reject, kind: "init" };
      w.postMessage({ type: "init", requestId: id });
    });
    return readyPromise;
  }

  async function diff(files, options = {}) {
    if (current) {
      throw new Error("Diff already in progress.");
    }
    await ready();
    const w = ensureWorker();
    const id = nextRequestId();
    return new Promise((resolve, reject) => {
      current = { id, resolve, reject, kind: "diff" };
      w.postMessage(
        {
          type: "diff",
          requestId: id,
          oldName: files.oldName,
          newName: files.newName,
          oldBuffer: files.oldBuffer,
          newBuffer: files.newBuffer,
          options
        },
        [files.oldBuffer, files.newBuffer]
      );
    });
  }

  async function downloadJsonl(files, options = {}) {
    if (current) {
      throw new Error("Diff already in progress.");
    }
    await ready();
    const w = ensureWorker();
    const id = nextRequestId();
    return new Promise((resolve, reject) => {
      current = { id, resolve, reject, kind: "jsonl", chunks: [] };
      w.postMessage(
        {
          type: "jsonl",
          requestId: id,
          oldName: files.oldName,
          newName: files.newName,
          oldBuffer: files.oldBuffer,
          newBuffer: files.newBuffer,
          options
        },
        [files.oldBuffer, files.newBuffer]
      );
    });
  }

  function cancel() {
    if (!worker) return false;
    if (current && current.reject) {
      current.reject(new Error("Diff canceled."));
      current = null;
    }
    resetWorker();
    return true;
  }

  function dispose() {
    disposed = true;
    resetWorker();
  }

  return {
    ready,
    diff,
    downloadJsonl,
    cancel,
    dispose
  };
}
