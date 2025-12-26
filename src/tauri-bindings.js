// Tauri bindings for WASM
// This file bridges Rust WASM code to Tauri's invoke API

export function isTauriContext() {
  return typeof window !== 'undefined' && window.__TAURI__ !== undefined;
}

export async function tauri_read_file(path) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('read_file', { path });
}

export async function tauri_write_file(path, contents) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('write_file', { path, contents });
}

export async function tauri_read_dir(path, maxDepth) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('read_dir', {
    path,
    maxDepth: maxDepth || null
  });
}

export async function tauri_create_file(path, contents) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('create_file', { path, contents });
}

export async function tauri_delete_file(path) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('delete_file', { path });
}

export async function tauri_rename_file(oldPath, newPath) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('rename_file', {
    oldPath,
    newPath
  });
}

export async function tauri_get_file_metadata(path) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('get_file_metadata', { path });
}

export async function tauri_search_in_files(query, rootPath, options) {
  if (!isTauriContext()) {
    throw new Error('Not in Tauri context');
  }
  return await window.__TAURI__.core.invoke('search_in_files', {
    query,
    rootPath,
    options: options || null
  });
}
