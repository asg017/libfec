/*
import init, {greet} from './pkg/fec_wasm.js';
await init();
console.log(greet('asdf'));
*/

import {greet} from './pkg/fec_wasm.js';
const res = greet('asdf');
console.log(res.fec_version);