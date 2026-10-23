// A module worker parsing a filing with the wasm build; items and records go
// back to the page by structured clone (a Record's data are own properties,
// so they survive).
//
// The library is imported dynamically and `onmessage` is set synchronously:
// the browser build initializes its wasm with top-level await, and a message
// that arrives while a module worker's top-level await is pending is
// dispatched before a statically imported handler exists, so it's lost.
const lib = import("../../dist/index.js");

self.onmessage = async (e) => {
  try {
    const { Filing } = await lib;
    const f = new Filing(e.data, { id: "1721696" });
    const items = [...f.itemizations()];
    const [firstRecord] = f.itemizationRecords();
    f.close();
    let codegen;
    try {
      codegen = new Function("return 1")() === 1;
    } catch {
      codegen = false;
    }
    self.postMessage({ n: items.length, first: items[0], items, firstRecord, codegen });
  } catch (err) {
    self.postMessage({ error: String(err) });
  }
};
