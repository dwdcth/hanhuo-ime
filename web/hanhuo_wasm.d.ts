/* tslint:disable */
/* eslint-disable */

/**
 * 英文候选（纯字母输入; 精确词优先 + 前缀补全频序——CLI --en 同律,
 * enlm 在场时切 LM 语境序: P(w|BOS) 降序, 词表外保序尾）。返回 JSON
 * {cands:[{word,rank}]}。
 */
export function en_candidates(inp: string, topk: number): string;

/**
 * 开启宽档 boost 配方（2026-11-20 升默认口径: 尾槽 B2.5 + B7 句序列
 * 重排 + beam150 段变体束——CLI wire_boost 的 --boost-tail
 * --boost-sent-nn 同律; jianpin_inject 同 wire_boost 默认）。返回
 * true=成功。词典未 init → false。
 */
export function enable_boost(): boolean;

/**
 * T9 九宫模式开关（纯 2-9 数字键串走九宫展开——设计-T9九宫输入;
 * CLI --t9 同律, on=false 恢复默认关）。返回 true=成功。
 */
export function enable_t9(on: boolean): boolean;

/**
 * 装载词典 bin（open/hanhuo_open.bin 等字节流）。返回词条数, 0=失败。
 */
export function init(dict: Uint8Array | number[]): number;

/**
 * 注入英文词表（data/en_open.tsv 字节, tsv; 四小件英文通道）。
 * 返回 true=装载成功。候选见 en_candidates。
 */
export function init_en(bytes: Uint8Array | number[]): boolean;

/**
 * 注入英文 LM（enlm safetensors 字节, NnlmAdv advanced 变体——CLI
 * --enlm 同律）。注入后 en_candidates 自动切 LM 语境序。
 */
export function init_enlm(bytes: Uint8Array | number[]): boolean;

/**
 * 注入 lmcn 双表（uni/bg tsv 字节——中文联想链/深召回面分依赖;
 * data/lmcn_open/ 两件, 缺席时 fcomp 垃圾面抢占）。
 */
export function init_lmcn(uni: Uint8Array | number[], bg: Uint8Array | number[]): boolean;

/**
 * 注入 NNLM 模型字节（mobile/advanced 变体自动路由——CLI
 * attach_model_files 同律; advanced 需 core safetensors feature, 本
 * crate 已开）。返回 true=装载成功。B7/c.3 等 NNLM 消费方即刻换脑。
 */
export function init_nnlm(bytes: Uint8Array | number[]): boolean;

/**
 * 查询 → JSON 候选串（word/kind/score/logp/reading 数组）。
 */
export function query(inp: string, ctx: string, topk: number): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly en_candidates: (a: number, b: number, c: number) => [number, number];
    readonly enable_boost: () => number;
    readonly enable_t9: (a: number) => number;
    readonly init: (a: number, b: number) => number;
    readonly init_en: (a: number, b: number) => number;
    readonly init_enlm: (a: number, b: number) => number;
    readonly init_lmcn: (a: number, b: number, c: number, d: number) => number;
    readonly init_nnlm: (a: number, b: number) => number;
    readonly query: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
