// Both Workers' handler: the version for GET, a POSTed filing's summary
// (tests/browser/summarize.mjs, copied next to this file) otherwise.
import { summarize } from "./summarize.mjs";

export async function respond(lib, request) {
  if (request.method === "GET") return Response.json({ version: lib.version });
  return Response.json(summarize(lib, new Uint8Array(await request.arrayBuffer())));
}
