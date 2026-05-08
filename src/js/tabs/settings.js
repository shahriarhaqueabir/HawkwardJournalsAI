import { invoke } from "../ipc.js";

export function initSettings() {
  const btnSubmitBug = document.getElementById("btn-submit-bug");
  const feedbackInput = document.getElementById("bug-report-feedback");
  const statusEl = document.getElementById("bug-report-status");

  if (!btnSubmitBug) return;

  btnSubmitBug.addEventListener("click", async () => {
    const feedback = feedbackInput.value.trim();
    if (!feedback) {
      statusEl.textContent = "Please provide some details about the bug first.";
      statusEl.style.color = "var(--color-error)";
      return;
    }

    try {
      btnSubmitBug.disabled = true;
      btnSubmitBug.textContent = "Generating...";
      statusEl.textContent = "";

      const reportPath = await invoke("report_bug", { userFeedback: feedback });
      
      feedbackInput.value = "";
      statusEl.textContent = `Bug report generated successfully at:\n${reportPath}`;
      statusEl.style.color = "#4caf50"; // success green
    } catch (err) {
      console.error("Failed to generate bug report:", err);
      statusEl.textContent = `Error: ${err}`;
      statusEl.style.color = "var(--color-error)";
    } finally {
      btnSubmitBug.disabled = false;
      btnSubmitBug.textContent = "Generate Bug Report";
    }
  });
}
