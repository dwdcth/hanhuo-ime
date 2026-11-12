// assets.js — 资产获取：fetch 字节进度 + IndexedDB cache-first
//
// 流程：
//   1. 查 IDB 缓存 → 命中直接返回（零下载）
//   2. 未命中 → fetch 流式读取，边下边报进度，写入 IDB
//   3. IDB 不可用（隐私模式等）→ 仅 fetch，不缓存

import { IdbAssetCache } from './storage.js';

export const assetCache = new IdbAssetCache();

/**
 * fetch 并累计字节，带进度回调。
 * @param {string} url
 * @param {{onProgress?: (loaded:number,total:number)=>void, signal?: AbortSignal}} opts
 * @returns {Promise<{bytes:Uint8Array, total:number, durationMs:number}>}
 */
export async function fetchWithProgress(url, { onProgress, signal } = {}) {
  const t0 = Date.now();
  const res = await fetch(url, { signal });
  if (!res.ok) throw new Error(`HTTP ${res.status} ${url}`);
  const totalHeader = res.headers.get('Content-Length');
  const total = totalHeader ? Number(totalHeader) : 0;

  if (!res.body || typeof res.body.getReader !== 'function') {
    // 老浏览器兜底：一次性读
    const buf = await res.arrayBuffer();
    onProgress && onProgress(buf.byteLength, buf.byteLength);
    return { bytes: new Uint8Array(buf), total: buf.byteLength, durationMs: Date.now() - t0 };
  }

  const reader = res.body.getReader();
  const chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    loaded += value.byteLength;
    onProgress && onProgress(loaded, total);
  }
  const bytes = new Uint8Array(loaded);
  let off = 0;
  for (const c of chunks) { bytes.set(c, off); off += c.byteLength; }
  return { bytes, total: total || loaded, durationMs: Date.now() - t0 };
}

/**
 * cache-first 资产加载。
 * @param {string} url
 * @param {{onProgress?: (loaded:number,total:number)=>void, forceRefresh?: boolean, label?: string}} opts
 * @returns {Promise<{bytes:Uint8Array, fromCache:boolean, durationMs:number, cached:boolean}>}
 */
export async function loadAsset(url, { onProgress, forceRefresh = false } = {}) {
  if (!forceRefresh) {
    const cached = await assetCache.get(url);
    if (cached && cached.byteLength > 0) {
      onProgress && onProgress(cached.byteLength, cached.byteLength);
      return { bytes: new Uint8Array(cached), fromCache: true, durationMs: 0, cached: true };
    }
  }
  const { bytes, total, durationMs } = await fetchWithProgress(url, { onProgress });
  // 写缓存失败不致命（如配额/隐私模式）
  const cached = await assetCache.put(url, bytes.buffer, { url, size: bytes.byteLength });
  return { bytes, fromCache: false, durationMs, cached, total };
}

/** 探测可选资产是否存在（404 -> false） */
export async function urlExists(url) {
  try {
    const res = await fetch(url, { method: 'HEAD' });
    if (res.status === 405 || res.status === 501) {
      // 个别静态服务不支持 HEAD，退回 GET（只看状态，不读体）
      const r2 = await fetch(url);
      return r2.ok;
    }
    return res.ok;
  } catch {
    return false;
  }
}

/** 人类可读字节数 */
export function fmtBytes(n) {
  if (!n && n !== 0) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
