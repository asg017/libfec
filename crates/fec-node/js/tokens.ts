// Decodes the typed-record token stream written by src/tokens.rs into plain
// objects. See that file for the encoding.
//
// Each struct type gets one factory, compiled with `new Function` the first
// time it is seen (an object literal with its keys in order, which V8 & co.
// turn into a fast shape), or a loop over its keys where code generation is
// blocked (CSP, `--disallow-code-generation-from-strings`). Both give the
// same objects.
//
// Two JS-only reshapes, so users see one convention (`type` = struct name):
// - an itemization (a struct whose first key is serde's `family` tag) drops
//   `family` and starts with `type: "<StructName>"`;
// - the cover (`Cover { form, data }`) is flattened to `{ type: form, ...data }`.

import type * as native from "./native.js";

const NULL = 0;
const STR = 1;
const NUM = 2;
const TRUE = 3;
const FALSE = 4;
const STRUCT = 5;
const SEQ = 6;

type Factory = () => unknown;

let codegenWorks: boolean | undefined;

/** Whether `new Function` is allowed here (checked once). */
function canCodegen(): boolean {
  if (codegenWorks === undefined) {
    try {
      codegenWorks = new Function("return 1")() === 1;
    } catch {
      codegenWorks = false;
    }
  }
  return codegenWorks;
}

export interface TokenDecoderOptions {
  /** `false` forces the loop factories (tests); default: codegen when allowed. */
  codegen?: boolean;
}

/** Decodes the batches of one reader; struct ids accumulate across batches. */
export class TokenDecoder {
  readonly #factories: Factory[] = [];
  readonly #codegen: boolean;

  // The batch being decoded.
  #tags: Uint8Array = new Uint8Array(0);
  #nums: Float64Array = new Float64Array(0);
  #text = "";
  #ends: Uint32Array = new Uint32Array(0);
  #ti = 0;
  #ni = 0;
  #si = 0;
  #pos = 0;

  /** Reads the next value; the factories call it for each field. */
  readonly #d: () => unknown;

  constructor(options: TokenDecoderOptions = {}) {
    this.#codegen = options.codegen ?? canCodegen();
    this.#d = () => {
      switch (this.#tags[this.#ti++]) {
        case STR: {
          const end = this.#ends[this.#si++]!;
          const s = this.#text.slice(this.#pos, end);
          this.#pos = end;
          return s;
        }
        case NUM:
          return this.#nums[this.#ni++];
        case NULL:
          return null;
        case STRUCT: {
          const id = this.#nums[this.#ni++]!;
          const factory = this.#factories[id];
          if (factory === undefined) {
            throw new Error(`corrupt token stream (unknown struct id ${id})`);
          }
          return factory();
        }
        case TRUE:
          return true;
        case FALSE:
          return false;
        case SEQ: {
          const n = this.#nums[this.#ni++]!;
          const out = new Array<unknown>(n);
          for (let i = 0; i < n; i++) out[i] = this.#d();
          return out;
        }
        default:
          throw new Error(`corrupt token stream (tag ${this.#tags[this.#ti - 1]})`);
      }
    };
  }

  /** Decode a batch: one value per row (`null` for a row with no typed record). */
  decode(batch: native.TokenBatch): unknown[] {
    for (const desc of batch.newStructs) this.#register(desc);
    this.#tags = batch.tags;
    this.#nums = batch.nums;
    this.#text = batch.text;
    this.#ends = batch.ends;
    this.#ti = this.#ni = this.#si = this.#pos = 0;
    const out = new Array<unknown>(batch.rows);
    for (let i = 0; i < batch.rows; i++) out[i] = this.#d();
    if (this.#ti !== this.#tags.length || this.#ni !== this.#nums.length) {
      throw new Error("corrupt token stream (batch not fully consumed)");
    }
    this.#text = "";
    return out;
  }

  #register(desc: string): void {
    const [id, name, ...keys] = desc.split("\x1f");
    this.#factories[Number(id)] = this.#codegen
      ? compile(name!, keys, this.#d)
      : loop(name!, keys, this.#d);
  }
}

const isCover = (name: string, keys: string[]) =>
  name === "Cover" && keys.length === 2 && keys[0] === "form" && keys[1] === "data";

function compile(name: string, keys: string[], d: () => unknown): Factory {
  if (isCover(name, keys)) return () => coverOf(d(), d());
  const tagged = keys[0] === "family";
  const fields = (tagged ? keys.slice(1) : keys).map((k) => `${JSON.stringify(k)}: d()`);
  if (tagged) fields.unshift(`type: ${JSON.stringify(name)}`);
  const body = `{${fields.join(", ")}}`;
  // `(d(), {...})` consumes the family value before building the object.
  const src = tagged ? `return () => (d(), ${body});` : `return () => (${body});`;
  return new Function("d", src)(d) as Factory;
}

function loop(name: string, keys: string[], d: () => unknown): Factory {
  if (isCover(name, keys)) return () => coverOf(d(), d());
  const tagged = keys[0] === "family";
  const own = tagged ? keys.slice(1) : keys;
  return () => {
    if (tagged) d();
    const o: Record<string, unknown> = tagged ? { type: name } : {};
    for (const k of own) o[k] = d();
    return o;
  };
}

function coverOf(form: unknown, data: unknown): unknown {
  return { type: form, ...(data as object) };
}
