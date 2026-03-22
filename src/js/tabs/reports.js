import { invoke } from "../ipc.js";

/**
 * Reports Tab Module - Phase 4
 * Handles data fetching and rendering for the analytical dashboard.
 */
let productivityChart = null;
let emotionsChart = null;
let energyChart = null;
let timeChart = null;
let moodsChart = null;
let taskStatusChart = null;
let dueBucketsChart = null;
let journalConsistencyChart = null;

export async function initReports() {
  console.log("[REPORTS] Initializing...");
  
  const refreshBtn = document.getElementById("btn-refresh-reports");
  const rangeSelect = document.getElementById("report-range");

  if (refreshBtn) refreshBtn.addEventListener("click", updateReports);
  if (rangeSelect) rangeSelect.addEventListener("change", updateReports);

  // Register for global app events
  globalThis.__REPORTS_EVENT_HANDLER__ = (payload) => {
    // Refresh stats if relevant events occur
    if (["journal_saved", "task_updated", "task_created", "task_completed", "task_deleted"].includes(payload.type)) {
      updateReports();
    }
  };

  // Initial load
  await updateReports();
}

async function updateReports() {
  const rangeSelect = document.getElementById("report-range");
  const days = rangeSelect ? parseInt(rangeSelect.value) : 7;
  
  try {
    const data = await invoke("get_report_summary", { days });
    console.log("[REPORTS] Data received:", data);

    renderMetrics(data);
    renderInsights(data);
    renderProductivityChart(data.productivity);
    renderJournalConsistencyChart(data.journal_by_day);
    renderEmotionChart(data.emotions);
    renderMoodChart(data.moods);
    renderProjectHealth(data.projects);
    renderEnergyChart(data.energy);
    renderTimeAllocationChart(data.time_allocation);
    renderTaskStatusChart(data.task_status);
    renderDueBucketsChart(data.due_buckets);
  } catch (err) {
    console.error("[REPORTS] Update failed:", err);
  }
}

function renderMetrics(data) {
  document.getElementById("stat-tasks-done").textContent = data.total_tasks_completed;
  document.getElementById("stat-journal-entries").textContent = data.total_journal_entries;
  document.getElementById("stat-active-projects").textContent = data.projects.length;
  
  // Avg Focus mockup (calculate from time_allocation if available)
  const totalMin = data.time_allocation.reduce((sum, item) => sum + item.total_minutes, 0);
  const avgFocus = data.total_tasks_completed > 0 ? (totalMin / data.total_tasks_completed).toFixed(0) : 0;
  document.getElementById("stat-avg-focus").textContent = avgFocus;
}

function renderInsights(data) {
  const el = document.getElementById("report-insights");
  if (!el) return;

  const insights = Array.isArray(data.insights) ? data.insights : [];
  if (insights.length === 0) {
    el.innerHTML = "<li>No insights yet. Create a few tasks, log time, and save some journal entries.</li>";
    return;
  }

  el.innerHTML = marked.parse(insights.map(s => `- ${s}`).join("\n"));
}

function renderProductivityChart(stats) {
  const ctx = document.getElementById("chart-productivity");
  if (!ctx) return;
  if (!window.Chart) return renderProductivitySvg(ctx, stats);
  prepareCanvasForChartJs(ctx);

  if (productivityChart) productivityChart.destroy();

  productivityChart = new window.Chart(ctx, {
    type: 'line',
    data: {
      labels: stats.map(s => s.date.split('-').slice(1).join('/')),
      datasets: [
        {
          label: 'Created',
          data: stats.map(s => s.created),
          borderColor: '#6c8ef7',
          backgroundColor: 'rgba(108, 142, 247, 0.1)',
          fill: true,
          tension: 0.4
        },
        {
          label: 'Completed',
          data: stats.map(s => s.completed),
          borderColor: '#4caf82',
          backgroundColor: 'rgba(76, 175, 130, 0.1)',
          fill: true,
          tension: 0.4
        }
      ]
    },
    options: chartOptions()
  });
}

function renderJournalConsistencyChart(byDay) {
  const ctx = document.getElementById("chart-journal-consistency");
  if (!ctx) return;
  if (!window.Chart) return renderJournalConsistencySvg(ctx, byDay);
  prepareCanvasForChartJs(ctx);

  if (journalConsistencyChart) journalConsistencyChart.destroy();

  const labels = byDay.map(d => d.date.split("-").slice(1).join("/"));
  const entries = byDay.map(d => d.count);
  const words = byDay.map(d => d.total_words);

  journalConsistencyChart = new window.Chart(ctx, {
    type: "bar",
    data: {
      labels,
      datasets: [
        {
          label: "Entries",
          data: entries,
          backgroundColor: "rgba(108, 142, 247, 0.55)",
          borderWidth: 0,
          yAxisID: "y",
        },
        {
          label: "Words",
          data: words,
          type: "line",
          borderColor: "rgba(41, 217, 194, 0.9)",
          backgroundColor: "rgba(41, 217, 194, 0.12)",
          tension: 0.35,
          fill: true,
          yAxisID: "y1",
          pointRadius: 0,
        },
      ],
    },
    options: {
      ...chartOptions(),
      plugins: {
        legend: { display: true, labels: { color: "#7a7f9a" } },
      },
      scales: {
        x: { grid: { color: "#2a2d3e" }, ticks: { color: "#7a7f9a", maxRotation: 0 } },
        y: { grid: { color: "#2a2d3e" }, ticks: { color: "#7a7f9a" }, beginAtZero: true, title: { display: true, text: "Entries", color: "#7a7f9a" } },
        y1: { position: "right", grid: { drawOnChartArea: false }, ticks: { color: "#7a7f9a" }, beginAtZero: true, title: { display: true, text: "Words", color: "#7a7f9a" } },
      },
    },
  });
}

function renderEmotionChart(emotions) {
  const ctx = document.getElementById("chart-emotions");
  if (!ctx) return;
  if (!window.Chart) return renderDonutSvg(ctx, emotions.map(e => ({ label: e.emotion, value: e.count })), "Emotions");
  prepareCanvasForChartJs(ctx);
  
  if (emotionsChart) emotionsChart.destroy();

  emotionsChart = new window.Chart(ctx, {
    type: 'doughnut',
    data: {
      labels: emotions.map(e => e.emotion),
      datasets: [{
        data: emotions.map(e => e.count),
        backgroundColor: [
          '#6c8ef7', '#4caf82', '#e05252', '#f0b429', '#29d9c2', '#ae54f2', '#f680b0'
        ],
        borderWidth: 0
      }]
    },
    options: {
      ...chartOptions(),
      plugins: {
        legend: { position: 'right' }
      }
    }
  });
}

function renderMoodChart(moods) {
  const ctx = document.getElementById("chart-moods");
  if (!ctx) return;
  if (!window.Chart) return renderDonutSvg(ctx, moods.map(m => ({ label: m.mood, value: m.count })), "Moods");
  prepareCanvasForChartJs(ctx);

  if (moodsChart) moodsChart.destroy();
  moodsChart = new window.Chart(ctx, {
    type: "doughnut",
    data: {
      labels: moods.map(m => m.mood),
      datasets: [{
        data: moods.map(m => m.count),
        backgroundColor: [
          '#29d9c2', '#6c8ef7', '#f0b429', '#e05252', '#ae54f2', '#4caf82', '#f680b0'
        ],
        borderWidth: 0
      }]
    },
    options: {
      ...chartOptions(),
      plugins: { legend: { position: "right", labels: { color: "#7a7f9a" } } }
    }
  });
}

function renderProjectHealth(projects) {
  const list = document.getElementById("report-project-list");
  if (!list) return;

  if (projects.length === 0) {
    list.innerHTML = '<div class="report-list-item">No project data available.</div>';
    return;
  }

  list.innerHTML = projects.slice(0, 5).map(p => {
    const progress = p.total_tasks > 0 ? Math.round((p.completed_tasks / p.total_tasks) * 100) : 0;
    const isHealthy = p.status === 'active';
    return `
      <div class="report-list-item ${isHealthy ? 'healthy' : ''}">
        <span>${p.name}</span>
        <div style="display:flex; gap: 8px; align-items:center;">
          <span class="status">${progress}% Done</span>
          <span class="status">${p.status}</span>
        </div>
      </div>
    `;
  }).join('');
}

function renderEnergyChart(energy) {
  const ctx = document.getElementById("chart-energy");
  if (!ctx) return;
  if (!window.Chart) return renderBarsSvg(ctx, energy.map(e => ({ label: (e.energy_level || "none").replace("_", " "), value: e.count })), { color: "#6c8ef7" });
  prepareCanvasForChartJs(ctx);
  
  if (energyChart) energyChart.destroy();

  energyChart = new window.Chart(ctx, {
    type: 'bar',
    data: {
      labels: energy.map(e => (e.energy_level || 'None').replace('_', ' ')),
      datasets: [{
        label: 'Tasks',
        data: energy.map(e => e.count),
        backgroundColor: 'rgba(108, 142, 247, 0.6)'
      }]
    },
    options: chartOptions()
  });
}

function renderTimeAllocationChart(allocation) {
  const ctx = document.getElementById("chart-time");
  if (!ctx) return;
  if (!window.Chart) return renderBarsSvg(ctx, allocation.map(a => ({ label: a.category, value: Number((a.total_minutes / 60).toFixed(1)) })), { color: "#29d9c2", valueSuffix: "h" });
  prepareCanvasForChartJs(ctx);
  
  if (timeChart) timeChart.destroy();

  timeChart = new window.Chart(ctx, {
    type: 'bar',
    indexAxis: 'y',
    data: {
      labels: allocation.map(a => a.category),
      datasets: [{
        label: 'Hours',
        data: allocation.map(a => (a.total_minutes / 60).toFixed(1)),
        backgroundColor: 'rgba(41, 191, 162, 0.6)'
      }]
    },
    options: chartOptions()
  });
}

function renderTaskStatusChart(statuses) {
  const ctx = document.getElementById("chart-task-status");
  if (!ctx) return;
  const items = statuses.map(s => ({ label: s.status, value: s.count }));
  if (!window.Chart) return renderDonutSvg(ctx, items, "Status");
  prepareCanvasForChartJs(ctx);

  if (taskStatusChart) taskStatusChart.destroy();
  taskStatusChart = new window.Chart(ctx, {
    type: "doughnut",
    data: {
      labels: items.map(i => i.label),
      datasets: [{
        data: items.map(i => i.value),
        backgroundColor: ['#6c8ef7', '#29d9c2', '#4caf82', '#f0b429', '#e05252', '#7a7f9a'],
        borderWidth: 0
      }]
    },
    options: {
      ...chartOptions(),
      plugins: { legend: { position: "bottom", labels: { color: "#7a7f9a" } } }
    }
  });
}

function renderDueBucketsChart(buckets) {
  const ctx = document.getElementById("chart-due-buckets");
  if (!ctx) return;

  const labelMap = {
    overdue: "Overdue",
    today: "Today",
    next_7: "Next 7 days",
    next_30: "Next 30 days",
    later: "Later",
    no_date: "No date",
  };
  const items = buckets.map(b => ({ label: labelMap[b.bucket] || b.bucket, value: b.count }));

  if (!window.Chart) return renderBarsSvg(ctx, items, { color: "#f0b429", horizontal: true });
  prepareCanvasForChartJs(ctx);

  if (dueBucketsChart) dueBucketsChart.destroy();
  dueBucketsChart = new window.Chart(ctx, {
    type: "bar",
    data: {
      labels: items.map(i => i.label),
      datasets: [{
        label: "Open tasks",
        data: items.map(i => i.value),
        backgroundColor: "rgba(240, 180, 41, 0.6)",
        borderWidth: 0,
      }]
    },
    options: {
      ...chartOptions(),
      indexAxis: "y",
      plugins: { legend: { display: false } },
      scales: {
        x: { grid: { color: "#2a2d3e" }, ticks: { color: "#7a7f9a" }, beginAtZero: true },
        y: { grid: { color: "#2a2d3e" }, ticks: { color: "#7a7f9a" }, beginAtZero: true },
      }
    }
  });
}

function chartOptions() {
  return {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false }
    },
    scales: {
      x: { grid: { color: '#2a2d3e' }, ticks: { color: '#7a7f9a' } },
      y: { grid: { color: '#2a2d3e' }, ticks: { color: '#7a7f9a' }, beginAtZero: true }
    }
  };
}

function renderChartFallback(canvas, message) {
  const card = canvas.closest(".report-card");
  if (!card) return;
  let fallback = card.querySelector(".chart-fallback");
  if (!fallback) {
    fallback = document.createElement("div");
    fallback.className = "chart-fallback";
    card.appendChild(fallback);
  }
  fallback.textContent = message;
}

function prepareCanvasForChartJs(canvas) {
  const card = canvas.closest(".report-card");
  if (card) {
    const svg = card.querySelector("svg.report-svg");
    if (svg) svg.remove();
  }
  canvas.style.display = "";
}

// ---------- Lightweight SVG chart fallbacks (no external deps) ----------

function renderProductivitySvg(canvas, stats) {
  if (!Array.isArray(stats) || stats.length === 0) {
    renderChartFallback(canvas, "No productivity data yet.");
    return;
  }

  const labels = stats.map(s => s.date.split("-").slice(1).join("/"));
  const created = stats.map(s => Number(s.created || 0));
  const completed = stats.map(s => Number(s.completed || 0));
  renderDualLineSvg(canvas, labels, [
    { name: "Created", values: created, color: "#6c8ef7" },
    { name: "Completed", values: completed, color: "#4caf82" },
  ]);
}

function renderJournalConsistencySvg(canvas, byDay) {
  if (!Array.isArray(byDay) || byDay.length === 0) {
    renderChartFallback(canvas, "No journal data yet.");
    return;
  }

  const labels = byDay.map(d => d.date.split("-").slice(1).join("/"));
  const entries = byDay.map(d => Number(d.count || 0));
  renderBarsSvg(canvas, labels.map((l, idx) => ({ label: l, value: entries[idx] })), { color: "#6c8ef7" });
}

function renderBarsSvg(canvas, items, { color = "#6c8ef7", horizontal = false, valueSuffix = "" } = {}) {
  if (!Array.isArray(items) || items.length === 0) {
    renderChartFallback(canvas, "No data yet.");
    return;
  }

  const w = 960;
  const h = 300;
  const pad = { t: 16, r: 18, b: 34, l: 40 };
  const max = Math.max(1, ...items.map(i => Number(i.value || 0)));

  const card = canvas.closest(".report-card");
  if (!card) return;
  const svg = createSvg(card, w, h);

  const plotW = w - pad.l - pad.r;
  const plotH = h - pad.t - pad.b;

  // Grid lines (3)
  for (let i = 0; i <= 3; i++) {
    const y = pad.t + (plotH * i) / 3;
    svg.appendChild(svgLine(pad.l, y, w - pad.r, y, "grid"));
  }

  if (!horizontal) {
    const bw = plotW / items.length;
    items.forEach((it, idx) => {
      const v = Number(it.value || 0);
      const bh = (v / max) * plotH;
      const x = pad.l + idx * bw + bw * 0.14;
      const y = pad.t + (plotH - bh);
      const barW = bw * 0.72;
      svg.appendChild(svgRect(x, y, barW, bh, color, "bar"));
      if (items.length <= 14) {
        svg.appendChild(svgText(pad.l + idx * bw + bw / 2, h - 12, it.label, "middle"));
      }
    });
    // Y axis labels
    svg.appendChild(svgText(10, pad.t + 10, String(max), "start"));
    svg.appendChild(svgText(10, pad.t + plotH, "0", "start"));
  } else {
    const rowH = plotH / items.length;
    items.forEach((it, idx) => {
      const v = Number(it.value || 0);
      const bw = (v / max) * plotW;
      const x = pad.l;
      const y = pad.t + idx * rowH + rowH * 0.18;
      const barH = rowH * 0.64;
      svg.appendChild(svgRect(x, y, bw, barH, color, "bar"));
      svg.appendChild(svgText(pad.l, y - 3, it.label, "start"));
      svg.appendChild(svgText(pad.l + bw + 6, y + barH * 0.78, `${v}${valueSuffix}`, "start"));
    });
  }

  hideCanvas(canvas);
}

function renderDonutSvg(canvas, items, title) {
  if (!Array.isArray(items) || items.length === 0) {
    renderChartFallback(canvas, `No ${title.toLowerCase()} data yet.`);
    return;
  }

  const total = items.reduce((s, i) => s + Number(i.value || 0), 0);
  if (!total) {
    renderChartFallback(canvas, `No ${title.toLowerCase()} data yet.`);
    return;
  }

  const colors = ["#6c8ef7", "#29d9c2", "#4caf82", "#f0b429", "#e05252", "#ae54f2", "#f680b0", "#7a7f9a"];
  const w = 960;
  const h = 300;
  const cx = 200;
  const cy = 150;
  const r = 88;
  const r2 = 56;

  const card = canvas.closest(".report-card");
  if (!card) return;
  const svg = createSvg(card, w, h);

  let start = -Math.PI / 2;
  items.slice(0, 8).forEach((it, idx) => {
    const v = Number(it.value || 0);
    const ang = (v / total) * Math.PI * 2;
    const end = start + ang;
    svg.appendChild(svgDonutSlice(cx, cy, r, r2, start, end, colors[idx % colors.length]));
    start = end;
  });

  // Center label
  svg.appendChild(svgText(cx, cy + 4, `${total}`, "middle", { className: "legend", fill: "#cfd3e6" }));
  svg.appendChild(svgText(cx, cy + 22, title, "middle", { className: "legend" }));

  // Legend
  const lx = 340;
  let ly = 70;
  items.slice(0, 8).forEach((it, idx) => {
    const label = `${it.label} (${it.value})`;
    svg.appendChild(svgRect(lx, ly - 10, 10, 10, colors[idx % colors.length], ""));
    svg.appendChild(svgText(lx + 16, ly, label, "start", { className: "legend" }));
    ly += 18;
  });

  hideCanvas(canvas);
}

function renderDualLineSvg(canvas, labels, series) {
  const w = 960;
  const h = 300;
  const pad = { t: 16, r: 18, b: 34, l: 40 };

  const card = canvas.closest(".report-card");
  if (!card) return;
  const svg = createSvg(card, w, h);

  const plotW = w - pad.l - pad.r;
  const plotH = h - pad.t - pad.b;
  const all = series.flatMap(s => s.values);
  const max = Math.max(1, ...all.map(v => Number(v || 0)));

  for (let i = 0; i <= 3; i++) {
    const y = pad.t + (plotH * i) / 3;
    svg.appendChild(svgLine(pad.l, y, w - pad.r, y, "grid"));
  }

  const xAt = (i) => pad.l + (plotW * i) / Math.max(1, labels.length - 1);
  const yAt = (v) => pad.t + plotH - (plotH * v) / max;

  series.forEach((s) => {
    const d = s.values
      .map((v, i) => `${i === 0 ? "M" : "L"} ${xAt(i).toFixed(2)} ${yAt(Number(v || 0)).toFixed(2)}`)
      .join(" ");
    svg.appendChild(svgPath(d, s.color, "line"));

    const areaD = [
      `M ${xAt(0).toFixed(2)} ${yAt(Number(s.values[0] || 0)).toFixed(2)}`,
      ...s.values.slice(1).map((v, i) => `L ${xAt(i + 1).toFixed(2)} ${yAt(Number(v || 0)).toFixed(2)}`),
      `L ${xAt(labels.length - 1).toFixed(2)} ${(pad.t + plotH).toFixed(2)}`,
      `L ${xAt(0).toFixed(2)} ${(pad.t + plotH).toFixed(2)}`,
      "Z"
    ].join(" ");
    svg.appendChild(svgPath(areaD, s.color, "area"));
  });

  // X labels (sparse)
  const step = labels.length > 14 ? Math.ceil(labels.length / 8) : 1;
  labels.forEach((lab, i) => {
    if (i % step !== 0 && i !== labels.length - 1) return;
    svg.appendChild(svgText(xAt(i), h - 12, lab, "middle"));
  });

  // Legend
  let lx = pad.l;
  const ly = pad.t + 14;
  series.forEach((s, idx) => {
    const x = lx + idx * 120;
    svg.appendChild(svgLine(x, ly, x + 18, ly, "", { stroke: s.color, strokeWidth: 3 }));
    svg.appendChild(svgText(x + 24, ly + 4, s.name, "start", { className: "legend" }));
  });

  hideCanvas(canvas);
}

function createSvg(card, w, h) {
  const existing = card.querySelector("svg.report-svg");
  if (existing) existing.remove();

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", `0 0 ${w} ${h}`);
  svg.setAttribute("class", "report-svg");
  card.appendChild(svg);
  return svg;
}

function hideCanvas(canvas) {
  canvas.style.display = "none";
}

function svgLine(x1, y1, x2, y2, cls = "", extra = {}) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", "line");
  el.setAttribute("x1", x1);
  el.setAttribute("y1", y1);
  el.setAttribute("x2", x2);
  el.setAttribute("y2", y2);
  if (cls) el.setAttribute("class", cls);
  Object.entries(extra).forEach(([k, v]) => el.setAttribute(k, v));
  return el;
}

function svgRect(x, y, w, h, fill, cls = "") {
  const el = document.createElementNS("http://www.w3.org/2000/svg", "rect");
  el.setAttribute("x", x);
  el.setAttribute("y", y);
  el.setAttribute("width", Math.max(0, w));
  el.setAttribute("height", Math.max(0, h));
  el.setAttribute("rx", 6);
  el.setAttribute("fill", fill);
  if (cls) el.setAttribute("class", cls);
  return el;
}

function svgText(x, y, text, anchor = "start", { className = "", fill = "", } = {}) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", "text");
  el.setAttribute("x", x);
  el.setAttribute("y", y);
  el.setAttribute("text-anchor", anchor);
  if (className) el.setAttribute("class", className);
  if (fill) el.setAttribute("fill", fill);
  el.textContent = text;
  return el;
}

function svgPath(d, color, cls) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", "path");
  el.setAttribute("d", d);
  el.setAttribute("class", cls);
  if (cls === "line") {
    el.setAttribute("stroke", color);
  } else {
    el.setAttribute("fill", color);
  }
  return el;
}

function svgDonutSlice(cx, cy, rOuter, rInner, a0, a1, fill) {
  const p = (r, a) => [cx + r * Math.cos(a), cy + r * Math.sin(a)];
  const [x0, y0] = p(rOuter, a0);
  const [x1, y1] = p(rOuter, a1);
  const [x2, y2] = p(rInner, a1);
  const [x3, y3] = p(rInner, a0);
  const large = a1 - a0 > Math.PI ? 1 : 0;

  const d = [
    `M ${x0.toFixed(2)} ${y0.toFixed(2)}`,
    `A ${rOuter} ${rOuter} 0 ${large} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`,
    `L ${x2.toFixed(2)} ${y2.toFixed(2)}`,
    `A ${rInner} ${rInner} 0 ${large} 0 ${x3.toFixed(2)} ${y3.toFixed(2)}`,
    "Z",
  ].join(" ");

  const el = document.createElementNS("http://www.w3.org/2000/svg", "path");
  el.setAttribute("d", d);
  el.setAttribute("fill", fill);
  return el;
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text ?? "";
  return div.innerHTML;
}
