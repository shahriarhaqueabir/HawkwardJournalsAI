import { invoke } from "../ipc.js";

/**
 * Reports Tab Module - Phase 4
 * Handles data fetching and rendering for the analytical dashboard.
 */
let productivityChart = null;
let emotionsChart = null;
let energyChart = null;
let timeChart = null;

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
    renderProductivityChart(data.productivity);
    renderEmotionChart(data.emotions);
    renderProjectHealth(data.projects);
    renderEnergyChart(data.energy);
    renderTimeAllocationChart(data.time_allocation);
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

function renderProductivityChart(stats) {
  const ctx = document.getElementById("chart-productivity");
  if (!ctx) return;
  if (!window.Chart) {
    renderChartFallback(ctx, "Chart.js is not loaded. Productivity trend is unavailable.");
    return;
  }

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

function renderEmotionChart(emotions) {
  const ctx = document.getElementById("chart-emotions");
  if (!ctx) return;
  if (!window.Chart) {
    renderChartFallback(ctx, "Chart.js is not loaded. Emotion chart is unavailable.");
    return;
  }
  
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
  if (!window.Chart) {
    renderChartFallback(ctx, "Chart.js is not loaded. Energy chart is unavailable.");
    return;
  }
  
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
  if (!window.Chart) {
    renderChartFallback(ctx, "Chart.js is not loaded. Time allocation chart is unavailable.");
    return;
  }
  
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
