// Parses filings off the main thread with the browser build and answers the
// page's questions about the last one. Plain data in and out: typed
// itemizations are plain objects, and an Unknown is sent as its record's
// values. A filing ID is downloaded here, with Filing.fromId through the
// page's CORS proxy.
//
// onmessage is set synchronously and the library imported dynamically: the
// browser build initializes its wasm with top-level await, and a message that
// arrives while a module worker's top-level await is pending would be
// dispatched before a statically imported handler exists.
const lib = import("../dist/index.js");

/** The last filing's itemizations, by type, for paging. */
let byType = new Map();

const money = (n) => Math.round(n * 100) / 100;

function entityName(e) {
  if (!e) return "";
  if (e.organization_name) return e.organization_name;
  const n = e.name ?? {};
  return [n.prefix, n.first_name, n.middle_name, n.last_name, n.suffix].filter(Boolean).join(" ");
}

function personName(n) {
  return n ? [n.first_name, n.last_name].filter(Boolean).join(" ") : "";
}

/** Sum amounts by key; top `k` by total. */
function top(rows, key, amount, k = 15) {
  const m = new Map();
  for (const r of rows) {
    const id = key(r);
    if (!id) continue;
    const e = m.get(id) ?? { name: id, total: 0, n: 0 };
    e.total += amount(r);
    e.n++;
    m.set(id, e);
  }
  return [...m.values()]
    .sort((a, b) => b.total - a.total)
    .slice(0, k)
    .map((e) => ({ ...e, total: money(e.total) }));
}

/** Read every itemization of an open filing and summarize it; closes the filing. */
function analyze(L, f, timings) {
  const t1 = performance.now();
  // Built locally: if a row fails to parse, the previous filing stays browsable.
  const types = new Map();
  let rows = 0;
  try {
    for (const item of f.itemizations()) {
      rows++;
      let list = types.get(item.type);
      if (!list) types.set(item.type, (list = []));
      // A record with no typed class (a summary record, …): browse its raw values.
      list.push(item.type === "Unknown" ? item.record.values : item);
    }
  } finally {
    f.close();
  }
  const t2 = performance.now();
  byType = types;

  const a = byType.get("ScheduleA") ?? [];
  const b = byType.get("ScheduleB") ?? [];
  const e = byType.get("ScheduleE") ?? [];
  const notMemo = (r) => !r.memo;
  return {
    id: f.id,
    bytes: f.sourceLength,
    header: f.header,
    filer: {
      formType: f.formType,
      filerId: f.filerId,
      filerName: f.filerName,
      reportCode: f.reportCode,
      coverageFromDate: f.coverageFromDate,
      coverageThroughDate: f.coverageThroughDate,
    },
    cover: f.cover.type === "Unknown" ? f.cover.toJSON() : f.cover,
    counts: Object.fromEntries([...byType].map(([t, l]) => [t, l.length]).sort((x, y) => y[1] - x[1])),
    itemizations: rows,
    timings: { ...timings, itemizations: t2 - t1 },
    totals: {
      receipts: money(a.filter(notMemo).reduce((s, r) => s + r.contribution_amount, 0)),
      disbursements: money(b.filter(notMemo).reduce((s, r) => s + r.expenditure_amount, 0)),
      independentExpenditures: money(e.filter(notMemo).reduce((s, r) => s + r.expenditure_amount, 0)),
    },
    topContributors: top(a.filter(notMemo), (r) => entityName(r.contributor), (r) => r.contribution_amount).map((t) => ({
      ...t,
      kind: L.entityTypeLabel(a.find((r) => entityName(r.contributor) === t.name)?.contributor.entity_type) ?? "",
    })),
    topPayees: top(b.filter(notMemo), (r) => entityName(r.payee), (r) => r.expenditure_amount),
    independentExpenditures: top(
      e.filter(notMemo),
      (r) => {
        const c = r.candidate;
        const who = personName(c?.name) || c?.fec_id || "(no candidate)";
        const where = [L.officeLabel(c?.office), c?.state, c?.district].filter(Boolean).join(" ");
        return `${L.supportOpposeLabel(r.support_oppose_code) ?? "?"} · ${who}${where ? ` (${where})` : ""}`;
      },
      (r) => r.expenditure_amount,
    ),
  };
}

function page({ type, offset, limit, query }) {
  let list = byType.get(type) ?? [];
  if (query) {
    const q = query.toLowerCase();
    list = list.filter((r) => JSON.stringify(r).toLowerCase().includes(q));
  }
  return { type, total: list.length, offset, rows: list.slice(offset, offset + limit) };
}

self.onmessage = async ({ data: msg }) => {
  try {
    const L = await lib;
    const reply = (result) => self.postMessage({ id: msg.id, kind: msg.kind, result });
    if (msg.kind === "parse") {
      const t0 = performance.now();
      const f = new L.Filing(msg.bytes, { id: msg.filingId });
      reply(analyze(L, f, { download: 0, open: performance.now() - t0 }));
    } else if (msg.kind === "fromId") {
      const t0 = performance.now();
      const f = await L.Filing.fromId(msg.filingId, { proxy: msg.proxy });
      reply(analyze(L, f, { download: performance.now() - t0, open: 0 }));
    } else if (msg.kind === "page") reply(page(msg));
    else if (msg.kind === "version") reply(L.version);
  } catch (e) {
    self.postMessage({ id: msg.id, kind: "error", error: `${e.name}: ${e.message}`, code: e.code });
  }
};
