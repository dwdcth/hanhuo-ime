// storage.js — 浏览器存储封装
//
// 大资产（词典/wasm/模型，69.6MB 级）→ IndexedDB；小设置 → localStorage。
// 完整评估与数据支撑见 STORAGE.md。
//
// 所有 API 在非浏览器环境（node 冒烟测试）安全降级为 no-op。

const DB_NAME = 'hanhuo-assets';
const DB_VERSION = 1;
const STORE = 'assets';

/** IndexedDB 二进制资产缓存（cache-first）。key 通常为资产相对 URL。 */
export class IdbAssetCache {
  constructor() {
    this._dbPromise = null;
    this.available = typeof indexedDB !== 'undefined';
  }

  _open() {
    if (!this.available) return Promise.resolve(null);
    if (!this._dbPromise) {
      this._dbPromise = new Promise((resolve) => {
        try {
          const req = indexedDB.open(DB_NAME, DB_VERSION);
          req.onupgradeneeded = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE);
          };
          req.onsuccess = () => resolve(req.result);
          req.onerror = () => resolve(null); // 隐私模式/配额等：降级，不致命
        } catch {
          resolve(null);
        }
      });
    }
    return this._dbPromise;
  }

  /** @returns {Promise<ArrayBuffer|null>} */
  async get(key) {
    const db = await this._open();
    if (!db) return null;
    return new Promise((resolve) => {
      try {
        const tx = db.transaction(STORE, 'readonly');
        const req = tx.objectStore(STORE).get(key);
        req.onsuccess = () => {
          const rec = req.result;
          resolve(rec && rec.bytes ? rec.bytes : null);
        };
        req.onerror = () => resolve(null);
      } catch {
        resolve(null);
      }
    });
  }

  async put(key, bytes, meta = {}) {
    const db = await this._open();
    if (!db) return false;
    return new Promise((resolve) => {
      try {
        const tx = db.transaction(STORE, 'readwrite');
        tx.objectStore(STORE).put({ bytes, ts: Date.now(), ...meta }, key);
        tx.oncomplete = () => resolve(true);
        tx.onerror = () => resolve(false);
        tx.onabort = () => resolve(false);
      } catch {
        resolve(false);
      }
    });
  }

  /** @returns {Promise<Array<{key, size, ts}>>} */
  async entries() {
    const db = await this._open();
    if (!db) return [];
    return new Promise((resolve) => {
      try {
        const tx = db.transaction(STORE, 'readonly');
        const store = tx.objectStore(STORE);
        const out = [];
        const cursorReq = store.openCursor();
        cursorReq.onsuccess = () => {
          const cur = cursorReq.result;
          if (cur) {
            const v = cur.value || {};
            out.push({ key: cur.key, size: v.bytes ? v.bytes.byteLength : 0, ts: v.ts || 0 });
            cur.continue();
          } else {
            resolve(out);
          }
        };
        cursorReq.onerror = () => resolve([]);
      } catch {
        resolve([]);
      }
    });
  }

  async clear() {
    const db = await this._open();
    if (!db) return false;
    return new Promise((resolve) => {
      try {
        const tx = db.transaction(STORE, 'readwrite');
        tx.objectStore(STORE).clear();
        tx.oncomplete = () => resolve(true);
        tx.onerror = () => resolve(false);
      } catch {
        resolve(false);
      }
    });
  }
}

// ---- 小状态（设置）走 localStorage：亚 KB 级、启动时同步读一次 ----

const SETTINGS_KEY = 'hanhuo.settings';

const memoryFallback = new Map(); // node / 隐私模式降级

export function loadSettings(defaults = {}) {
  try {
    const raw = typeof localStorage !== 'undefined' ? localStorage.getItem(SETTINGS_KEY) : memoryFallback.get(SETTINGS_KEY);
    if (!raw) return { ...defaults };
    return { ...defaults, ...JSON.parse(raw) };
  } catch {
    return { ...defaults };
  }
}

export function saveSettings(patch) {
  try {
    const next = { ...loadSettings(), ...patch };
    const json = JSON.stringify(next);
    if (typeof localStorage !== 'undefined') localStorage.setItem(SETTINGS_KEY, json);
    else memoryFallback.set(SETTINGS_KEY, json);
    return next;
  } catch {
    return loadSettings();
  }
}

// ---- 配额查询（状态栏展示用） ----

export async function storageEstimate() {
  try {
    if (typeof navigator !== 'undefined' && navigator.storage && navigator.storage.estimate) {
      const { usage, quota } = await navigator.storage.estimate();
      return { supported: true, usage: usage || 0, quota: quota || 0 };
    }
  } catch { /* ignore */ }
  return { supported: false, usage: 0, quota: 0 };
}

/** 请求持久化存储，降低浏览器在磁盘压力下逐出缓存的概率 */
export async function requestPersistence() {
  try {
    if (typeof navigator !== 'undefined' && navigator.storage && navigator.storage.persist) {
      return await navigator.storage.persist();
    }
  } catch { /* ignore */ }
  return false;
}
