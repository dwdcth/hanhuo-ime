/* @ts-self-types="./hanhuo_wasm.d.ts" */

/**
 * 英文候选（纯字母输入; 精确词优先 + 前缀补全频序——CLI --en 同律,
 * enlm 在场时切 LM 语境序: P(w|BOS) 降序, 词表外保序尾）。返回 JSON
 * {cands:[{word,rank}]}。
 * @param {string} inp
 * @param {number} topk
 * @returns {string}
 */
export function en_candidates(inp, topk) {
    let deferred3_0;
    let deferred3_1;
    try {
        const val2 = +topk;
        const ptr0 = passStringToWasm0(inp, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.en_candidates(ptr0, len0, val2);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * 开启宽档 boost 配方（2026-11-20 升默认口径: 尾槽 B2.5 + B7 句序列
 * 重排 + beam150 段变体束——CLI wire_boost 的 --boost-tail
 * --boost-sent-nn 同律; jianpin_inject 同 wire_boost 默认）。返回
 * true=成功。词典未 init → false。
 * @returns {boolean}
 */
export function enable_boost() {
    const ret = wasm.enable_boost();
    return ret !== 0;
}

/**
 * T9 九宫模式开关（纯 2-9 数字键串走九宫展开——设计-T9九宫输入;
 * CLI --t9 同律, on=false 恢复默认关）。返回 true=成功。
 * @param {boolean} on
 * @returns {boolean}
 */
export function enable_t9(on) {
    const ret = wasm.enable_t9(on);
    return ret !== 0;
}

/**
 * 装载词典 bin（open/hanhuo_open.bin 等字节流）。返回词条数, 0=失败。
 * @param {Uint8Array | number[]} dict
 * @returns {number}
 */
export function init(dict) {
    const ptr0 = passArray8ToWasm0(dict, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.init(ptr0, len0);
    return ret >>> 0;
}

/**
 * 注入英文词表（data/en_open.tsv 字节, tsv; 四小件英文通道）。
 * 返回 true=装载成功。候选见 en_candidates。
 * @param {Uint8Array | number[]} bytes
 * @returns {boolean}
 */
export function init_en(bytes) {
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.init_en(ptr0, len0);
    return ret !== 0;
}

/**
 * 注入英文 LM（enlm safetensors 字节, NnlmAdv advanced 变体——CLI
 * --enlm 同律）。注入后 en_candidates 自动切 LM 语境序。
 * @param {Uint8Array | number[]} bytes
 * @returns {boolean}
 */
export function init_enlm(bytes) {
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.init_enlm(ptr0, len0);
    return ret !== 0;
}

/**
 * 注入 lmcn 双表（uni/bg tsv 字节——中文联想链/深召回面分依赖;
 * data/lmcn_open/ 两件, 缺席时 fcomp 垃圾面抢占）。
 * @param {Uint8Array | number[]} uni
 * @param {Uint8Array | number[]} bg
 * @returns {boolean}
 */
export function init_lmcn(uni, bg) {
    let ptr0, len0, ptr1, len1;
    try {
        ptr0 = passArray8ToWasm0(uni, wasm.__wbindgen_malloc);
        len0 = WASM_VECTOR_LEN;
        ptr1 = passArray8ToWasm0(bg, wasm.__wbindgen_malloc);
        len1 = WASM_VECTOR_LEN;
    } catch (e) {
        if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
        if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
        throw e;
    }
    const ret = wasm.init_lmcn(ptr0, len0, ptr1, len1);
    return ret !== 0;
}

/**
 * 注入 NNLM 模型字节（mobile/advanced 变体自动路由——CLI
 * attach_model_files 同律; advanced 需 core safetensors feature, 本
 * crate 已开）。返回 true=装载成功。B7/c.3 等 NNLM 消费方即刻换脑。
 * @param {Uint8Array | number[]} bytes
 * @returns {boolean}
 */
export function init_nnlm(bytes) {
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.init_nnlm(ptr0, len0);
    return ret !== 0;
}

/**
 * 查询 → JSON 候选串（word/kind/score/logp/reading 数组）。
 * @param {string} inp
 * @param {string} ctx
 * @param {number} topk
 * @returns {string}
 */
export function query(inp, ctx, topk) {
    let deferred4_0;
    let deferred4_1;
    try {
        const val3 = +topk;
        let ptr0, len0, ptr1, len1;
        try {
            ptr0 = passStringToWasm0(inp, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len0 = WASM_VECTOR_LEN;
            ptr1 = passStringToWasm0(ctx, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            len1 = WASM_VECTOR_LEN;
        } catch (e) {
            if (ptr1) { wasm.__wbindgen_free(ptr1, len1 * 1, 1); }
            if (ptr0) { wasm.__wbindgen_free(ptr0, len0 * 1, 1); }
            throw e;
        }
        const ret = wasm.query(ptr0, len0, ptr1, len1, val3);
        deferred4_0 = ret[0];
        deferred4_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_log_59ef980b5ec39889: function(arg0, arg1) {
            console.log(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./hanhuo_wasm_bg.js": import0,
    };
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('hanhuo_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
