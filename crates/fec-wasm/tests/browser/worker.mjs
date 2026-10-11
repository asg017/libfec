// A module worker parsing a filing with the wasm build; rows go back to the
// page by structured clone (Row's data are own properties, so they survive).
//
// The library is imported dynamically and `onmessage` is set synchronously:
// the browser build initializes its wasm with top-level await, and a message
// that arrives while a module worker's top-level await is pending is
// dispatched before a statically imported handler exists, so it's lost.
const lib = import("../../dist/index.js");

self.onmessage = async (e) => {
  try {
    const { open } = await lib;
    const f = open(e.data, { id: "1721696" });
    const rows = [...f.itemizations()];
    f.close();
    self.postMessage({ n: rows.length, first: rows[0], rows });
  } catch (err) {
    self.postMessage({ error: String(err) });
  }
};
