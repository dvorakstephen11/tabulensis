import { createDiffWorkerClient } from "./diff_worker_client.js";
import {
  createNativeDiffClient,
  getNativeBridge,
  loadNativeRecents,
  openNativeFileDialog,
  saveNativeRecent
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

export async function loadRecents() {
  if (!isDesktop()) return [];
  return loadNativeRecents();
}

export async function saveRecent(entry) {
  if (!isDesktop()) return [];
  return saveNativeRecent(entry);
}
