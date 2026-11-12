// composer.js — 组合文本状态机（纯逻辑，无 DOM，可在 node 中测试）
//
// 模型：与引擎契约 Hanhuo.query(input) -> candidates 对齐——
//   查询以「整个待转换拼音串」为单位，选中候选即消耗全部 pending。
//   已上屏文本累积在 committed，用于拼句（多次选择）。

export class Composer {
  constructor() {
    this.committed = ''; // 已上屏文本
    this.pending = '';   // 待转换拼音（仅 [a-z]）
  }

  /** 追加/替换待转换拼音（过滤非小写字母），返回实际生效的串 */
  setPending(raw, t9 = false) {
    // T9 模式保留 2-9 键域（数字是键串非调号）; 默认拼音域仅 [a-z]
    const re = t9 ? /[^2-9]/g : /[^a-z]/g;
    const clean = String(raw == null ? '' : raw).toLowerCase().replace(re, '');
    this.pending = clean;
    return clean;
  }

  /** 是否需要发起查询 */
  get needsQuery() {
    return this.pending.length > 0;
  }

  /**
   * 选中候选：上屏 + 清空 pending，返回上屏的词。
   * at（可选，按 Unicode 码点计）：插入位置，默认追加到末尾——
   * 上屏文本区点击定位后，新词插在光标处而非句尾。
   */
  pick(candidate, at = null) {
    if (!candidate || typeof candidate.word !== 'string' || !candidate.word) return null;
    const cps = Array.from(this.committed);
    if (at == null || at < 0 || at > cps.length) at = cps.length;
    cps.splice(at, 0, ...Array.from(candidate.word));
    this.committed = cps.join('');
    this.pending = '';
    return candidate.word;
  }

  /**
   * 退格删掉已上屏中的一个字符（按 Unicode 码点切分，防止切断代理对）。
   * at（可选，码点位置）：删 at 前面那个字符，默认删末尾。
   * at === 0（光标在句首且句中还有字）：改删句首第一个字符——
   * 保证「退格 = 删已上屏」在任何光标位置都有可见效果。
   */
  uncommitLast(at = null) {
    const cps = Array.from(this.committed);
    if (!cps.length) return null;
    if (at == null || at < 0 || at > cps.length) at = cps.length;
    const removed = at === 0 ? cps.shift() : cps.splice(at - 1, 1)[0];
    this.committed = cps.join('');
    return removed;
  }

  /** 清空待转换拼音（Esc），不动已上屏文本 */
  clearPending() {
    this.pending = '';
  }

  reset() {
    this.committed = '';
    this.pending = '';
  }
}
