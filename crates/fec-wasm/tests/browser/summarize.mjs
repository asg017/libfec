// Everything the public API says about one filing, as JSON-able data. Runs
// in the browser (on the wasm build) and in Node (on the napi build); the
// runner compares the two. No Node or DOM APIs here.

/** A record as plain data, fields included. */
const plainRecord = (r) => ({ ...r.toJSON(), fields: r.fields });

/** @param {any} lib  either build's module @param {Uint8Array} data */
export function summarize(lib, data) {
  const out = {};
  try {
    out.header = lib.readHeader(data);
  } catch (e) {
    return { error: { name: e.constructor.name, code: e.code, message: e.message } };
  }
  try {
    const f = new lib.Filing(data, { id: "x" });
    try {
      out.filer = [f.id, f.formType, f.filerId, f.filerName, f.reportCode, f.coverageFromDate, f.coverageThroughDate, f.sourceLength];
      out.cover = f.cover instanceof lib.Unknown ? { unknown: f.cover.toJSON() } : f.cover;
      out.coverRecord = plainRecord(f.coverRecord);
      // Each pass runs until it ends or throws (a row with no column
      // mapping), and how it ended is part of the summary.
      out.itemizations = [];
      out.itemizationRecords = [];
      out.ends = {};
      for (const [view, list, map] of [
        ["itemizations", out.itemizations, (x) => (x instanceof lib.Unknown ? { unknown: x.toJSON(), line: x.record.line } : x)],
        ["itemizationRecords", out.itemizationRecords, plainRecord],
      ]) {
        try {
          for (const x of f[view]()) list.push(map(x));
          out.ends[view] = "done";
        } catch (e) {
          if (!(e instanceof lib.MissingMappingError)) throw e;
          out.ends[view] = { name: e.name, rowType: e.rowType, line: e.line };
        }
      }
    } finally {
      f.close();
    }
  } catch (e) {
    out.error = { name: e.constructor.name, code: e.code, message: e.message };
  }
  return out;
}
