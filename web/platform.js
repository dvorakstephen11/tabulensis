import { createDiffWorkerClient } from "./diff_worker_client.js";
import {
  createNativeDiffClient,
  getNativeBridge,
  loadNativeRecents,
  openNativeFileDialog,
  openNativeFolderDialog,
  saveNativeRecent,
  loadNativeDiffSummary,
  loadNativeSheetPayload,
  exportNativeAuditXlsx,
  runNativeBatchCompare,
  loadNativeBatchSummary,
  loadNativeCapabilities,
  searchNativeDiffOps,
  buildNativeSearchIndex,
  searchNativeWorkbookIndex
} from "./native_diff_client.js";

export function isDesktop() {
  return Boolean(getNativeBridge());
}

export function createAppDiffClient({ onStatus } = {}) {
  if (isDesktop()) {
    return createNativeDiffClient({ onStatus });
  }
  return createDiffWorkerClient({ onStatus });
}

export async function openFileDialog() {
  if (!isDesktop()) return null;
  return openNativeFileDialog();
}

export async function openFolderDialog() {
  if (!isDesktop()) return null;
  return openNativeFolderDialog();
}

export async function loadRecents() {
  if (!isDesktop()) return [];
  return loadNativeRecents();
}

export async function saveRecent(entry) {
  if (!isDesktop()) return [];
  return saveNativeRecent(entry);
}

export async function loadDiffSummary(diffId) {
  if (!isDesktop()) return null;
  return loadNativeDiffSummary(diffId);
}

export async function loadSheetPayload(diffId, sheetName) {
  if (!isDesktop()) return null;
  return loadNativeSheetPayload(diffId, sheetName);
}

export async function exportAuditXlsx(diffId) {
  if (!isDesktop()) return null;
  return exportNativeAuditXlsx(diffId);
}

export async function runBatchCompare(request) {
  if (!isDesktop()) return null;
  return runNativeBatchCompare(request);
}

export async function loadBatchSummary(batchId) {
  if (!isDesktop()) return null;
  return loadNativeBatchSummary(batchId);
}

export async function getCapabilities() {
  if (!isDesktop()) return null;
  return loadNativeCapabilities();
}

export async function searchDiffOps(diffId, query, limit) {
  if (!isDesktop()) return [];
  return searchNativeDiffOps(diffId, query, limit);
}

export async function buildSearchIndex(path, side) {
  if (!isDesktop()) return null;
  return buildNativeSearchIndex(path, side);
}

export async function searchWorkbookIndex(indexId, query, limit) {
  if (!isDesktop()) return [];
  return searchNativeWorkbookIndex(indexId, query, limit);
}
