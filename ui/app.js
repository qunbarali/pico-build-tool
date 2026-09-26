const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;
const output = document.getElementById("output");
const status = document.getElementById("status");
const build = document.getElementById("build");

let outputListener;
let busy = false;
const actionButtons = [...document.querySelectorAll("[data-action]")];

async function startOutputStream() {
  if (outputListener) {
    await outputListener();
  }

  outputListener = await listen("pico-build-output", (event) => {
    const payload = event.payload;
    if (!payload || typeof payload.text !== "string") return;
    output.textContent += payload.text;
    output.scrollTop = output.scrollHeight;
  });
}

function resetOutput() {
  output.textContent = "";
}

function render(result) {
  if (!output.textContent.trim()) {
    output.textContent = [result.stdout, result.stderr].filter(Boolean).join("\n");
  }
  status.textContent = result.success
    ? "Completed successfully."
    : "Build/command failed. See diagnostics above.";
  status.style.color = result.success ? "#7ee2a8" : "#ff8e9e";
}

async function run(args) {
  if (busy) return;
  busy = true;
  build.disabled = true;
  actionButtons.forEach((button) => { button.disabled = true; });
  status.textContent = "Running...";
  status.style.color = "#7ed8ff";
  resetOutput();

  try {
    await startOutputStream();
    const result = await invoke("run_cli", { args });
    render(result);
  } catch (error) {
    output.textContent += String(error);
    status.textContent = "Could not start the build engine.";
    status.style.color = "#ff8e9e";
  } finally {
    busy = false;
    build.disabled = false;
    actionButtons.forEach((button) => { button.disabled = false; });
  }
}

build.addEventListener("click", async () => {
  const project = document.getElementById("project").value.trim() || ".";
  const profile = document.getElementById("profile").value;
  await run(["build", project, "--profile", profile]);
});

document.querySelectorAll("[data-action]").forEach((button) => {
  button.addEventListener("click", () => {
    const action = button.dataset.action;
    run(action === "doctor" ? ["doctor"] : [action]);
  });
});

document.getElementById("clear").addEventListener("click", () => {
  resetOutput();
  status.textContent = "Ready.";
  status.style.color = "#7ed8ff";
});
