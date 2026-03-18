let toastContainer = null;

function ensureToastContainer() {
  if (toastContainer) return toastContainer;
  toastContainer = document.getElementById("toast-container");
  if (toastContainer) return toastContainer;

  toastContainer = document.createElement("div");
  toastContainer.id = "toast-container";
  document.body.appendChild(toastContainer);
  return toastContainer;
}

function stringifyError(err) {
  if (err == null) return "Unknown error";
  if (typeof err === "string") return err;
  if (typeof err === "object") {
    if (typeof err.message === "string" && err.message.trim()) return err.message;
    try {
      return JSON.stringify(err);
    } catch {
      return String(err);
    }
  }
  return String(err);
}

export function showToast(message, { variant = "info", timeoutMs = 4500 } = {}) {
  const container = ensureToastContainer();
  const toast = document.createElement("div");
  toast.className = `toast toast--${variant}`;
  toast.textContent = message;

  container.appendChild(toast);

  const timeout = window.setTimeout(() => {
    toast.classList.add("toast--hide");
    window.setTimeout(() => toast.remove(), 180);
  }, timeoutMs);

  toast.addEventListener("click", () => {
    window.clearTimeout(timeout);
    toast.remove();
  });
}

export function showError(err, { title } = {}) {
  const msg = stringifyError(err);
  showToast(title ? `${title}: ${msg}` : msg, { variant: "error" });
}
