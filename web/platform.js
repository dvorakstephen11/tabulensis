import { createDiffWorkerClient } from "./diff_worker_client.js";

function createNativeRpcBridge() {
  if (typeof window === "undefined") return null;
  const sender = window.__tabulensisPostMessage;
  if (typeof sender !== "function") return null;

  let nextId = 1;
  const pending = new Map();
  let notifyHandler = null;

  function send(payload) {
    sender(JSON.stringify(payload));
  }

  function request(method, params) {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      try {
        send({ id, method, params });
      } catch (err) {
        pending.delete(id);
        reject(err);
      }
    });
  }

  function handleMessage(raw) {
    let payload = raw;
    if (typeof raw === "string") {
      try {
        payload = JSON.parse(raw);
      } catch (err) {
        console.warn("Failed to parse native message:", err);
        return;
      }
    }
    if (!payload || typeof payload !== "object") return;

    if (payload.id != null) {
      const entry = pending.get(payload.id);
      if (!entry) return;
      pending.delete(payload.id);
      if (payload.ok) {
        entry.resolve(payload.result);
      } else {
        const error = payload.error || {};
        const message = error.message || payload.message || "Request failed";
        const err = new Error(message);
        err.data = error;
        entry.reject(err);
      }
      return;
    }

    if (payload.method && notifyHandler) {
      notifyHandler(payload);
    }
  }

  window.__tabulensisReceive = handleMessage;

  return {
    request,
    setNotifyHandler(fn) {
      notifyHandler = fn;
    }
  };
}

let nativeRpc = null;

function getNativeRpc() {
  if (!nativeRpc) {
    nativeRpc = createNativeRpcBridge();
  }
  return nativeRpc;
}

export function isDesktop() {
  return Boolean(getNativeRpc());
}

export function createAppDiffClient({ onStatus } = {}) {
  const rpc = getNativeRpc();
  if (!rpc) {
    return createDiffWorkerClient({ onStatus });
  }

  if (typeof onStatus === "function") {
    rpc.setNotifyHandler(msg => {
      if (msg.method === "status") {
        onStatus(msg.params || {});
      }
    });
  }

  return {
    ready() {
      return rpc.request("ready");
    },
    diff(files, options = {}) {
      return rpc.request("diff", {
        oldPath: files.oldPath,
        newPath: files.newPath,
        oldName: files.oldName,
        newName: files.newName,
        options
      });
    },
    cancel() {
      rpc.request("cancel");
      return true;
    }
  };
}

export async function openFileDialog() {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("openFileDialog");
}

export async function openFolderDialog() {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("openFolderDialog");
}

export async function loadRecents() {
  const rpc = getNativeRpc();
  if (!rpc) return [];
  return rpc.request("loadRecents");
}

export async function saveRecent(entry) {
  const rpc = getNativeRpc();
  if (!rpc) return [];
  return rpc.request("saveRecent", entry);
}

export async function loadDiffSummary(diffId) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("loadDiffSummary", { diffId });
}

export async function loadSheetPayload(diffId, sheetName) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("loadSheetPayload", { diffId, sheetName });
}

export async function exportAuditXlsx(diffId) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("exportAuditXlsx", { diffId });
}

export async function openPath(path, reveal = false) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("openPath", { path, reveal });
}

export async function loadSheetMeta(diffId, sheetName) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("loadSheetMeta", { diffId, sheetName });
}

export async function loadOpsInRange(diffId, sheetName, range) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("loadOpsInRange", { diffId, sheetName, range });
}

export async function loadCellsInRange(diffId, sheetName, side, range) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("loadCellsInRange", { diffId, sheetName, side, range });
}

export async function runBatchCompare(request) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("runBatchCompare", request);
}

export async function loadBatchSummary(batchId) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("loadBatchSummary", { batchId });
}

export async function getCapabilities() {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("getCapabilities");
}

export async function searchDiffOps(diffId, query, limit) {
  const rpc = getNativeRpc();
  if (!rpc) return [];
  return rpc.request("searchDiffOps", { diffId, query, limit });
}

export async function buildSearchIndex(path, side) {
  const rpc = getNativeRpc();
  if (!rpc) return null;
  return rpc.request("buildSearchIndex", { path, side });
}

export async function searchWorkbookIndex(indexId, query, limit) {
  const rpc = getNativeRpc();
  if (!rpc) return [];
  return rpc.request("searchWorkbookIndex", { indexId, query, limit });
}
