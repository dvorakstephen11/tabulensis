import { createDiffWorkerClient } from "./diff_worker_client.js";

export function isDesktop() {
  return false;
}

export function createAppDiffClient({ onStatus } = {}) {
  return createDiffWorkerClient({ onStatus });
}

export async function openFileDialog() {
  return null;
}

export async function openFolderDialog() {
  return null;
}

export async function loadRecents() {
  return [];
}

export async function saveRecent(entry) {
  void entry;
  return [];
}

export async function loadDiffSummary(diffId) {
  void diffId;
  return null;
}

export async function loadSheetPayload(diffId, sheetName) {
  void diffId;
  void sheetName;
  return null;
}

export async function exportAuditXlsx(diffId) {
  void diffId;
  return null;
}

export async function runBatchCompare(request) {
  void request;
  return null;
}

export async function loadBatchSummary(batchId) {
  void batchId;
  return null;
}

export async function getCapabilities() {
  return null;
}

export async function searchDiffOps(diffId, query, limit) {
  void diffId;
  void query;
  void limit;
  return [];
}

export async function buildSearchIndex(path, side) {
  void path;
  void side;
  return null;
}

export async function searchWorkbookIndex(indexId, query, limit) {
  void indexId;
  void query;
  void limit;
  return [];
}
