const invoke = window.__TAURI__.core.invoke;
const output = document.getElementById("output");
const status = document.getElementById("status");
const build = document.getElementById("build");

function render(result) {
  const text = [result.stdout, result.stderr].filter(Boolean).join("\n");
  output.textContent = text || "Command completed without console output.";
  status.textContent = result.success ? "Completed successfully." : "Build/command failed. See diagnostics below.";
  status.style.color = result.success ? "#7ee2a8" : "#ff8e9e";
}

async function run(args) {
  status.textContent = "Running...";
  status.style.color = "#7ed8ff";
  try {
    render(await invoke("run_cli", { args }));
  } catch (error) {
    output.textContent = String(error);
    status.textContent = "Could not start the build engine.";
    status.style.color = "#ff8e9e";
  }
}

build.addEventListener("click", async () => {
  build.disabled = true;
  const project = document.getElementById("project").value.trim() || ".";
  const profile = document.getElementById("profile").value;
  await run(["build", project, "--profile", profile]);
  build.disabled = false;
});

document.querySelectorAll("[data-action]").forEach((button) => {
  button.addEventListener("click", () => {
    const action = button.dataset.action;
    run(action === "doctor" ? ["doctor"] : [action]);
  });
});

document.getElementById("clear").addEventListener("click", () => {
  output.textContent = "No command has been run yet.";
  status.textContent = "Ready.";
  status.style.color = "#7ed8ff";
});
