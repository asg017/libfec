// Everything the public API says about one filing, as JSON-able data. Runs
// in the browser (on the wasm build) and in Node (on the napi build); the
// runner compares the two. No Node or DOM APIs here.

/** @param {any} lib  either build's module @param {Uint8Array} data */
export function summarize(lib, data) {
  const out = {};
  try {
    out.header = lib.readHeader(data);
  } catch (e) {
    return { error: { name: e.constructor.name, code: e.code, message: e.message } };
  }
  try {
    const f = lib.open(data, { id: "x" });
    try {
      out.cover = f.cover;
      out.coverSummary = f.coverSummary;
      out.coverRow = f.coverRow.toJSON();
      out.itemizations = [...f.itemizations()].map((r) => r.toJSON());
      out.invalidValueCount = f.invalidValueCount;
    } finally {
      f.close();
    }
    const g = lib.open(data, { unknownRows: "skip" });
    try {
      out.rows = [...g.rows()].map((r) => r.toJSON());
      out.skippedRows = g.skippedRows;
    } finally {
      g.close();
    }
  } catch (e) {
    out.error = { name: e.constructor.name, code: e.code, message: e.message };
  }
  return out;
}
