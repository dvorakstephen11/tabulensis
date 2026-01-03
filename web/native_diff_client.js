function resolveTauri() {
  if (typeof window === "undefined") return null;
  return window.__TAURI__ || null;
}

function resolveInvoke(tauri) {
  if (tauri?.core?.invoke) return tauri.core.invoke;
  if (tauri?.invoke) return tauri.invoke;
  return null;
}

function resolveListen(tauri) {
  if (tauri?.event?.listen) return tauri.event.listen;
  return null;
}

export function getNativeBridge() {
  const tauri = resolveTauri();
  if (!tauri) return null;
  const invoke = resolveInvoke(tauri);
  if (typeof invoke !== "function") return null;
  return {
    invoke,
    listen: resolveListen(tauri)
  };
}

function notify(onStatus, payload) {
  if (typeof onStatus === "function") {
    onStatus(payload);
  }
}

export async function openNativeFileDialog() {
  const bridge = getNativeBridge();
  if (!bridge) {
    throw new Error("Native dialog unavailable.");
  }
  const result = await bridge.invoke("pick_file");
  if (!result) return null;
  return result;
}

export async function openNativeFolderDialog() {
  const bridge = getNativeBridge();
  if (!bridge) {
    throw new Error("Native dialog unavailable.");
  }
  const result = await bridge.invoke("pick_folder");
  if (!result) return null;
  return result;
}

export async function loadNativeRecents() {
  const bridge = getNativeBridge();
  if (!bridge) return [];
  return bridge.invoke("load_recents");
}

export async function saveNativeRecent(entry) {
  const bridge = getNativeBridge();
  if (!bridge) return [];
  return bridge.invoke("save_recent", { entry });
}

export async function loadNativeDiffSummary(diffId) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("load_diff_summary", { diff_id: diffId });
}

export async function loadNativeSheetPayload(diffId, sheetName) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("load_sheet_payload", { diff_id: diffId, sheet_name: sheetName });
}

export async function exportNativeAuditXlsx(diffId) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("export_audit_xlsx", { diff_id: diffId });
}

export async function runNativeBatchCompare(request) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("run_batch_compare", { request });
}

export async function loadNativeBatchSummary(batchId) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("load_batch_summary", { batch_id: batchId });
}

export async function loadNativeCapabilities() {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("get_capabilities");
}

export async function searchNativeDiffOps(diffId, query, limit) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("search_diff_ops", { diff_id: diffId, query, limit });
}

export async function buildNativeSearchIndex(path, side) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("build_search_index", { path, side });
}

export async function searchNativeWorkbookIndex(indexId, query, limit) {
  const bridge = getNativeBridge();
  if (!bridge) throw new Error("Native bridge unavailable.");
  return bridge.invoke("search_workbook_index", { index_id: indexId, query, limit });
}

export function createNativeDiffClient({ onStatus } = {}) {
  const bridge = getNativeBridge();
  if (!bridge) {
    throw new Error("Native bridge unavailable.");
  }

  let current = null;
  let requestCounter = 0;
  let cachedVersion = null;
  let readyPromise = null;
  let disposed = false;
  let unlisten = null;

  function nextRequestId() {
    requestCounter += 1;
    return requestCounter;
  }

  async function ensureListener() {
    if (unlisten || typeof bridge.listen !== "function") return;
    unlisten = await bridge.listen("diff-progress", event => {
      if (!current || !event || !event.payload) return;
      const payload = event.payload;
      if (payload.runId !== current.id) return;
      notify(onStatus, {
        stage: payload.stage,
        detail: payload.detail,
        source: "native"
      });
    });
  }

  async function ready() {
    if (cachedVersion) return cachedVersion;
    if (readyPromise) return readyPromise;
    readyPromise = bridge.invoke("get_version").then(version => {
      cachedVersion = version || "";
      return cachedVersion;
    });
    return readyPromise;
  }

  async function diff(files, options = {}) {
    if (disposed) {
      throw new Error("Native diff client disposed.");
    }
    if (current) {
      throw new Error("Diff already in progress.");
    }
    await ready();
    await ensureListener();
    const id = nextRequestId();
    return new Promise((resolve, reject) => {
      current = { id, resolve, reject };
      bridge
        .invoke("diff_paths_with_sheets", {
          old_path: files.oldPath,
          new_path: files.newPath,
          run_id: id,
          options
        })
        .then(payload => {
          if (!current || current.id !== id) return;
          current = null;
          resolve(payload);
        })
        .catch(err => {
          if (!current || current.id !== id) return;
          current = null;
          reject(err instanceof Error ? err : new Error(String(err)));
        });
    });
  }

  async function downloadJsonl() {
    throw new Error("JSONL download is only available in the web worker.");
  }

  function cancel() {
    if (!current) return false;
    const id = current.id;
    bridge.invoke("cancel_diff", { run_id: id }).catch(() => {});
    current.reject(new Error("Diff canceled."));
    current = null;
    return true;
  }

  function dispose() {
    disposed = true;
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    current = null;
  }

  return {
    ready,
    diff,
    cancel,
    dispose,
    downloadJsonl,
    loadSummary: loadNativeDiffSummary,
    loadSheetPayload: loadNativeSheetPayload,
    exportAuditXlsx: exportNativeAuditXlsx,
    runBatchCompare: runNativeBatchCompare,
    loadBatchSummary: loadNativeBatchSummary,
    getCapabilities: loadNativeCapabilities,
    searchDiffOps: searchNativeDiffOps,
    buildSearchIndex: buildNativeSearchIndex,
    searchWorkbookIndex: searchNativeWorkbookIndex
  };
}
