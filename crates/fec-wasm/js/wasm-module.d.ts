// js/workerd.ts's import: a compiled module under wrangler's CompiledWasm rule.
declare module "*.wasm" {
  const module: WebAssembly.Module;
  export default module;
}
