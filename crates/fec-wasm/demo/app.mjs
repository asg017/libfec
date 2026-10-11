// FEC Filing Explorer: the page. Parsing happens in worker.mjs; this module
// loads sources (a dropped file, a filing ID, the RSS feed) and renders.
import { rss } from "../dist/index.js";

const $ = (id) => document.getElementById(id);
const PROXY = `${location.origin}/proxy?url=`;
const PAGE = 50;

// ---- worker ---------------------------------------------------------------

const worker = new Worker(new URL("./worker.mjs", import.meta.url), { type: "module" });
const pending = new Map();
let nextId = 0;
worker.onmessage = ({ data }) => {
  const p = pending.get(data.id);
  pending.delete(data.id);
  if (!p) return;
  data.kind === "error" ? p.reject(Object.assign(new Error(data.error), { code: data.code })) : p.resolve(data.result);
};
worker.onerror = (e) => {
  for (const p of pending.values()) p.reject(new Error(e.message));
  pending.clear();
};

/** Ask the worker; replies are matched by id, so requests may overlap. */
function ask(msg, transfer = []) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    worker.postMessage({ ...msg, id }, transfer);
  });
}

// ---- formatting -------------------------------------------------------------

const usd = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", maximumFractionDigits: 0 });
const int = new Intl.NumberFormat("en-US");
const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const mb = (n) => `${(n / 2 ** 20).toFixed(n < 2 ** 20 ? 2 : 1)} MB`;

function setStatus(text, error = false) {
  $("status").textContent = text;
  $("status").classList.toggle("error", error);
}

function busy(on) {
  for (const el of document.querySelectorAll("button, input, select")) el.disabled = on;
}

// ---- sources ----------------------------------------------------------------

async function download(url, label) {
  setStatus(`Downloading ${label}…`);
  const res = await fetch(PROXY + encodeURIComponent(url));
  if (!res.ok) throw new Error(`${label}: HTTP ${res.status}${res.status === 404 ? " (no such filing)" : ""}`);
  const total = Number(res.headers.get("content-length")) || 0;
  const reader = res.body.getReader();
  const chunks = [];
  let got = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    got += value.length;
    setStatus(`Downloading ${label}… ${mb(got)}${total ? ` of ${mb(total)}` : ""}`);
  }
  const bytes = new Uint8Array(got);
  let at = 0;
  for (const c of chunks) {
    bytes.set(c, at);
    at += c.length;
  }
  return bytes;
}

async function load(getBytes, id) {
  busy(true);
  try {
    const bytes = await getBytes();
    setStatus(`Parsing ${mb(bytes.length)}…`);
    const size = bytes.length;
    const result = await ask({ kind: "parse", bytes, filingId: id }, [bytes.buffer]);
    render(result, size);
    const secs = (result.timings.open + result.timings.itemizations) / 1000;
    setStatus(
      `Parsed ${int.format(result.itemizations)} itemizations from ${mb(size)} in ${secs.toFixed(2)} s ` +
        `(${int.format(Math.round(result.itemizations / Math.max(secs, 1e-3)))} rows/s) in a Web Worker.`,
    );
  } catch (e) {
    setStatus(e.message, true);
  } finally {
    busy(false);
  }
}

function loadFile(file) {
  load(async () => new Uint8Array(await file.arrayBuffer()), file.name.replace(/\.fec$/i, "").replace(/^FEC-/, ""));
}

function loadId(id) {
  load(() => download(`https://docquery.fec.gov/dcdev/posted/${id}.fec`, `filing ${id}`), id);
}

const drop = $("drop");
drop.addEventListener("click", () => $("file").click());
drop.addEventListener("keydown", (e) => (e.key === "Enter" || e.key === " ") && $("file").click());
$("file").addEventListener("change", (e) => e.target.files[0] && loadFile(e.target.files[0]));
drop.addEventListener("dragover", (e) => {
  e.preventDefault();
  drop.classList.add("over");
});
drop.addEventListener("dragleave", () => drop.classList.remove("over"));
drop.addEventListener("drop", (e) => {
  e.preventDefault();
  drop.classList.remove("over");
  const file = e.dataTransfer.files[0];
  if (file) loadFile(file);
});
$("by-id").addEventListener("submit", (e) => {
  e.preventDefault();
  loadId($("filing-id").value.trim());
});

// ---- RSS feed ---------------------------------------------------------------

async function loadFeed() {
  $("feed-status").textContent = "Loading the FEC RSS feed…";
  $("feed").replaceChildren();
  try {
    const feed = await rss.fetch({ preset: $("preset").value, proxy: PROXY });
    $("feed-status").textContent = `${int.format(feed.items.length)} filings in the last week, newest first. Click one to parse it.`;
    const rtf = new Intl.RelativeTimeFormat("en", { numeric: "auto" });
    const ago = (d) => {
      const mins = Math.round((d - Date.now()) / 60_000);
      return Math.abs(mins) < 60 ? rtf.format(mins, "minute") : Math.abs(mins) < 1440 ? rtf.format(Math.round(mins / 60), "hour") : rtf.format(Math.round(mins / 1440), "day");
    };
    $("feed").replaceChildren(
      ...feed.items.slice(0, 200).map((item) => {
        const li = document.createElement("li");
        li.innerHTML = `<span class="form">${esc(item.formType)}</span><span class="id muted">${esc(item.filingId)}</span>` +
          `<span>${esc(item.committeeName)}${item.reportType ? ` <span class="muted">· ${esc(item.reportType)}</span>` : ""}</span>` +
          `<span class="when muted">${item.pubDate ? ago(item.pubDate) : ""}</span>`;
        li.addEventListener("click", () => item.filingId && loadId(item.filingId));
        return li;
      }),
    );
  } catch (e) {
    $("feed-status").textContent =
      `The feed needs a CORS proxy, which this server doesn't have (${e.message}). ` +
      "Run `make demo` in crates/fec-wasm, or drop a .fec file above.";
  }
}
$("preset").addEventListener("change", loadFeed);

// ---- rendering --------------------------------------------------------------

function table(el, head, rows, empty = "None in this filing.") {
  el.innerHTML = rows.length
    ? `<thead><tr>${head.map((h) => `<th>${h}</th>`).join("")}</tr></thead><tbody>${rows.join("")}</tbody>`
    : `<tbody><tr><td class="empty">${empty}</td></tr></tbody>`;
}

function ranked(el, list, label) {
  const max = Math.max(...list.map((r) => r.total), 1);
  table(
    el,
    [label, "#", "Total"],
    list.map(
      (r) =>
        `<tr><td class="bar" style="--w:${((100 * r.total) / max).toFixed(1)}%"><span>${esc(r.name)}${r.kind ? ` <span class="muted">· ${esc(r.kind)}</span>` : ""}</span></td>` +
        `<td class="n">${int.format(r.n)}</td><td class="n">${usd.format(r.total)}</td></tr>`,
    ),
  );
}

function render(r, size) {
  $("result").hidden = false;
  const s = r.coverSummary;
  $("filer-name").textContent = s.filerName || "(no name)";
  const coverage = s.coverageFromDate ? `${s.coverageFromDate} to ${s.coverageThroughDate}` : "no coverage period";
  $("filer-meta").textContent =
    `${s.formType}${s.reportCode ? ` · ${s.reportCode}` : ""} · ${s.filerId} · ${coverage} · ` +
    `FEC ${r.header.fecVersion} · ${r.header.softwareName} ${r.header.softwareVersion}` +
    (r.id ? ` · filing ${r.id}` : "");

  const kpis = [
    ["Itemizations", int.format(r.itemizations)],
    ["Receipts (Sch. A)", usd.format(r.totals.receipts)],
    ["Disbursements (Sch. B)", usd.format(r.totals.disbursements)],
  ];
  if (r.totals.independentExpenditures) kpis.push(["Independent exp.", usd.format(r.totals.independentExpenditures)]);
  const c = r.cover;
  if (c?.type === "Form3X") kpis.push(["Cash on hand", usd.format(c.summary.line8_cash_on_hand_close_of_period.column_a)]);
  if (c?.type === "Form3") kpis.push(["Cash on hand", usd.format(c.summary.line8_cash_on_hand_close_of_period)]);
  if (c?.type === "Form3P") kpis.push(["Cash on hand", usd.format(c.summary.line10_cash_on_hand_end_period)]);
  kpis.push(["Parse time", `${((r.timings.open + r.timings.itemizations) / 1000).toFixed(2)} s`], ["Size", mb(size)]);
  $("kpis").innerHTML = kpis.map(([k, v]) => `<div class="kpi"><div class="v">${esc(v)}</div><div class="k">${esc(k)}</div></div>`).join("");

  const counts = Object.entries(r.counts);
  table($("counts"), ["Type", "Rows"], counts.map(([t, n]) => `<tr class="click" data-type="${esc(t)}"><td>${esc(t)}</td><td class="n">${int.format(n)}</td></tr>`), "No itemizations.");
  for (const tr of $("counts").querySelectorAll("tr[data-type]")) tr.addEventListener("click", () => browseType(tr.dataset.type));
  ranked($("contributors"), r.topContributors, "Contributor");
  ranked($("payees"), r.topPayees, "Payee");
  ranked($("ie"), r.independentExpenditures, "Support/oppose · candidate");

  $("browse-type").innerHTML = counts.map(([t, n]) => `<option value="${esc(t)}">${esc(t)} (${int.format(n)})</option>`).join("");
  $("cover-json").textContent = JSON.stringify({ cover: r.cover, header: r.header }, null, 2);
  browse.offset = 0;
  browse.query = "";
  $("browse-q").value = "";
  if (counts.length) browseType(counts[0][0], false);
  else table($("browse"), [], []);
}

// ---- browsing ---------------------------------------------------------------

const browse = { type: null, offset: 0, query: "" };

/** Columns for the common types; anything else shows its first scalar fields. */
const name = (e) => (e?.organization_name ? e.organization_name : [e?.name?.first_name, e?.name?.last_name].filter(Boolean).join(" "));
const COLUMNS = {
  ScheduleA: [["Date", (r) => r.contribution_date], ["Contributor", (r) => name(r.contributor)], ["Employer / occupation", (r) => [r.contributor_employer, r.contributor_occupation].filter(Boolean).join(" · ")], ["Amount", (r) => r.contribution_amount, true]],
  ScheduleB: [["Date", (r) => r.expenditure_date], ["Payee", (r) => name(r.payee)], ["Purpose", (r) => r.expenditure_purpose_description], ["Amount", (r) => r.expenditure_amount, true]],
  ScheduleE: [["Date", (r) => r.dissemination_date ?? r.disbursement_date], ["Payee", (r) => name(r.payee)], ["Candidate", (r) => [r.candidate?.name?.first_name, r.candidate?.name?.last_name].filter(Boolean).join(" ")], ["S/O", (r) => r.support_oppose_code], ["Amount", (r) => r.expenditure_amount, true]],
};

function genericColumns(row) {
  return Object.entries(row)
    .filter(([k, v]) => k !== "type" && (v === null || typeof v !== "object"))
    .slice(0, 6)
    .map(([k, v]) => [k, (r) => r[k], typeof v === "number"]);
}

async function browseType(type, scroll = true) {
  browse.type = type;
  browse.offset = 0;
  $("browse-type").value = type;
  await showPage();
  if (scroll) $("browse").scrollIntoView({ behavior: "smooth", block: "center" });
}

async function showPage() {
  const res = await ask({ kind: "page", type: browse.type, offset: browse.offset, limit: PAGE, query: browse.query });
  const cols = COLUMNS[browse.type] ?? (res.rows[0] ? genericColumns(res.rows[0]) : []);
  table(
    $("browse"),
    cols.map(([h]) => esc(h)),
    res.rows.map(
      (r) =>
        `<tr${r.memo ? ' class="muted"' : ""}>${cols
          .map(([, f, num]) => {
            const v = f(r);
            return num ? `<td class="n">${typeof v === "number" ? usd.format(v) : ""}</td>` : `<td>${esc(v)}</td>`;
          })
          .join("")}</tr>`,
    ),
    "No rows match.",
  );
  const end = Math.min(res.offset + PAGE, res.total);
  $("page-info").textContent = res.total ? `${int.format(res.offset + 1)}–${int.format(end)} of ${int.format(res.total)}` : "";
  $("prev").disabled = res.offset === 0;
  $("next").disabled = end >= res.total;
}

$("browse-type").addEventListener("change", (e) => browseType(e.target.value, false));
let searchTimer;
$("browse-q").addEventListener("input", (e) => {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    browse.query = e.target.value.trim();
    browse.offset = 0;
    showPage();
  }, 200);
});
$("prev").addEventListener("click", () => {
  browse.offset = Math.max(0, browse.offset - PAGE);
  showPage();
});
$("next").addEventListener("click", () => {
  browse.offset += PAGE;
  showPage();
});

// ---- start ------------------------------------------------------------------

ask({ kind: "version" }).then((v) => ($("version").textContent = `libfec ${v}.`));
loadFeed();
const fromUrl = new URLSearchParams(location.search).get("id");
if (fromUrl && /^\d+$/.test(fromUrl)) loadId(fromUrl);
