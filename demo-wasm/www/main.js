import init, { Simulation } from "../pkg/cogwise_demo_wasm.js";

const btCanvas = document.getElementById("bt-canvas");
const curveCanvas = document.getElementById("curve-canvas");
const sandboxCanvas = document.getElementById("sandbox-canvas");
const compareCanvases = [
  document.getElementById("cmp-0"),
  document.getElementById("cmp-1"),
  document.getElementById("cmp-2"),
  document.getElementById("cmp-3"),
];
const curveReadout = document.getElementById("curve-readout");
const speedInput = document.getElementById("speed");
const stepButton = document.getElementById("step");
const addAgentButton = document.getElementById("add-agent");
const tabButtons = [...document.querySelectorAll(".tab")];
const panels = [...document.querySelectorAll(".panel")];

const app = {
  activeTab: "bt",
  hoverX: 0.5,
  speed: 2,
  singleStep: false,
  sim: null,
  snapshot: null,
};

await init();
app.sim = new Simulation();

for (const tab of tabButtons) {
  tab.addEventListener("click", () => {
    app.activeTab = tab.dataset.tab;
    tabButtons.forEach((btn) => btn.classList.toggle("is-active", btn === tab));
    panels.forEach((panel) =>
      panel.classList.toggle("is-active", panel.dataset.panel === app.activeTab),
    );
    drawAll();
  });
}

speedInput.addEventListener("input", () => {
  app.speed = Number(speedInput.value);
});

stepButton.addEventListener("click", () => {
  app.singleStep = true;
});

addAgentButton.addEventListener("click", () => {
  const x = Math.floor(Math.random() * 20);
  const y = Math.floor(Math.random() * 20);
  const preset = Math.floor(Math.random() * 4);
  app.sim.add_agent(x, y, preset);
});

sandboxCanvas.addEventListener("click", (event) => {
  const rect = sandboxCanvas.getBoundingClientRect();
  const x = Math.floor(((event.clientX - rect.left) / rect.width) * 20);
  const y = Math.floor(((event.clientY - rect.top) / rect.height) * 20);
  app.sim.set_waypoint(x, y);
});

curveCanvas.addEventListener("mousemove", (event) => {
  const rect = curveCanvas.getBoundingClientRect();
  app.hoverX = clamp((event.clientX - rect.left) / rect.width, 0, 1);
  drawCurves();
});

function parseSnapshot(raw) {
  const lines = raw.trim().split("\n");
  const snapshot = {
    tick: 0,
    waypoint: { x: 0, y: 0 },
    signal: 0,
    agents: [],
  };

  for (const line of lines) {
    if (line.startsWith("TICK:")) snapshot.tick = Number(line.slice(5));
    if (line.startsWith("WAYPOINT:")) {
      const [x, y] = line
        .slice(9)
        .split(",")
        .map((v) => Number(v));
      snapshot.waypoint = { x, y };
    }
    if (line.startsWith("SIGNAL:")) snapshot.signal = Number(line.slice(7));
    if (line.startsWith("A:")) {
      const [x, y, preset, stamina] = line
        .slice(2)
        .split(",")
        .map((v) => Number(v));
      snapshot.agents.push({ x, y, preset, stamina });
    }
  }

  return snapshot;
}

function frame() {
  const ticksThisFrame = app.singleStep ? 1 : app.speed;
  app.singleStep = false;
  for (let i = 0; i < ticksThisFrame; i += 1) {
    app.sim.tick();
  }
  app.snapshot = parseSnapshot(app.sim.render());
  drawAll();
  requestAnimationFrame(frame);
}

function drawAll() {
  drawBt();
  drawCurves();
  drawSandbox();
  drawCompare();
}

function drawBt() {
  const ctx = btCanvas.getContext("2d");
  const w = btCanvas.width;
  const h = btCanvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#faf7f1";
  ctx.fillRect(0, 0, w, h);

  const signal = app.snapshot?.signal ?? 0;
  const statuses = [
    signal > 0.7 ? "success" : "running",
    signal > 0.55 ? "success" : "failure",
    signal > 0.4 ? "running" : "failure",
    signal > 0.3 ? "success" : "running",
    signal > 0.2 ? "failure" : "running",
  ];
  const colors = {
    success: "#2f9e44",
    failure: "#d9480f",
    running: "#fab005",
    idle: "#adb5bd",
  };

  const nodes = [
    { x: 440, y: 60, label: "Selector", status: statuses[0] },
    { x: 220, y: 180, label: "Flee", status: statuses[1] },
    { x: 440, y: 180, label: "Attack", status: statuses[2] },
    { x: 660, y: 180, label: "Approach", status: statuses[3] },
    { x: 440, y: 310, label: "Idle", status: statuses[4] },
  ];

  ctx.strokeStyle = "#d0c5b7";
  ctx.lineWidth = 2;
  link(ctx, nodes[0], nodes[1]);
  link(ctx, nodes[0], nodes[2]);
  link(ctx, nodes[0], nodes[3]);
  link(ctx, nodes[2], nodes[4]);

  for (const node of nodes) {
    ctx.beginPath();
    ctx.fillStyle = colors[node.status];
    ctx.strokeStyle = "#fff";
    ctx.lineWidth = 3;
    ctx.arc(node.x, node.y, 34, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = "#1f1f1f";
    ctx.font = "600 14px 'Space Grotesk', sans-serif";
    ctx.textAlign = "center";
    ctx.fillText(node.label, node.x, node.y + 50);
  }

  ctx.fillStyle = "#5f574d";
  ctx.font = "500 13px 'Space Grotesk', sans-serif";
  ctx.textAlign = "left";
  ctx.fillText(
    `Tick ${app.snapshot?.tick ?? 0} | signal ${(app.snapshot?.signal ?? 0).toFixed(2)}`,
    18,
    h - 16,
  );
}

function drawCurves() {
  const ctx = curveCanvas.getContext("2d");
  const w = curveCanvas.width;
  const h = curveCanvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#fefcf8";
  ctx.fillRect(0, 0, w, h);
  ctx.strokeStyle = "#e4d8c8";
  ctx.lineWidth = 1;
  for (let i = 0; i <= 10; i += 1) {
    const x = 60 + (i / 10) * (w - 100);
    const y = 30 + (i / 10) * (h - 80);
    ctx.beginPath();
    ctx.moveTo(x, 30);
    ctx.lineTo(x, h - 50);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(60, y);
    ctx.lineTo(w - 40, y);
    ctx.stroke();
  }

  const curves = [
    { label: "Linear", color: "#0d7a66", fn: (x) => x },
    { label: "Polynomial", color: "#de6c24", fn: (x) => x * x },
    {
      label: "Logistic",
      color: "#1d5fbf",
      fn: (x) => 1 / (1 + Math.exp(-10 * (x - 0.5))),
    },
    { label: "Step", color: "#7a4b9d", fn: (x) => (x >= 0.6 ? 1 : 0) },
    { label: "Inverse", color: "#6b6f00", fn: (x) => Math.min(1, 1 / (x + 0.15)) },
  ];

  for (const curve of curves) {
    ctx.beginPath();
    ctx.strokeStyle = curve.color;
    ctx.lineWidth = 2.2;
    for (let i = 0; i <= 120; i += 1) {
      const x = i / 120;
      const y = curve.fn(x);
      const px = mapX(x, w);
      const py = mapY(y, h);
      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.stroke();
  }

  const x = app.hoverX;
  const px = mapX(x, w);
  ctx.beginPath();
  ctx.strokeStyle = "#222";
  ctx.setLineDash([5, 4]);
  ctx.moveTo(px, 20);
  ctx.lineTo(px, h - 40);
  ctx.stroke();
  ctx.setLineDash([]);

  const values = curves.map((curve) => `${curve.label}: ${curve.fn(x).toFixed(2)}`);
  curveReadout.textContent = `x = ${x.toFixed(2)} | ${values.join(" | ")}`;
}

function drawSandbox() {
  const ctx = sandboxCanvas.getContext("2d");
  const w = sandboxCanvas.width;
  const h = sandboxCanvas.height;
  const grid = 20;
  const cell = w / grid;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#f9f9f7";
  ctx.fillRect(0, 0, w, h);
  ctx.strokeStyle = "#e6e2da";
  ctx.lineWidth = 1;
  for (let i = 0; i <= grid; i += 1) {
    const p = i * cell;
    ctx.beginPath();
    ctx.moveTo(p, 0);
    ctx.lineTo(p, h);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(0, p);
    ctx.lineTo(w, p);
    ctx.stroke();
  }

  const waypoint = app.snapshot?.waypoint ?? { x: 10, y: 10 };
  ctx.fillStyle = "#ff7f50";
  const wx = waypoint.x * cell + cell / 2;
  const wy = waypoint.y * cell + cell / 2;
  ctx.beginPath();
  ctx.moveTo(wx, wy - 10);
  ctx.lineTo(wx + 10, wy);
  ctx.lineTo(wx, wy + 10);
  ctx.lineTo(wx - 10, wy);
  ctx.closePath();
  ctx.fill();

  const agents = app.snapshot?.agents ?? [];
  for (const agent of agents) {
    const color = ["#0d7a66", "#de6c24", "#1d5fbf", "#7a4b9d"][agent.preset % 4];
    ctx.beginPath();
    ctx.fillStyle = color;
    ctx.arc(agent.x * cell + cell / 2, agent.y * cell + cell / 2, cell * 0.28, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawCompare() {
  const tick = app.snapshot?.tick ?? 0;
  const signal = app.snapshot?.signal ?? 0;
  const modes = [
    { label: "Aggressive", hue: "#d9480f" },
    { label: "Defensive", hue: "#2f9e44" },
    { label: "Patrol", hue: "#0d7a66" },
    { label: "Random", hue: "#8a5cf6" },
  ];

  compareCanvases.forEach((canvas, i) => {
    const ctx = canvas.getContext("2d");
    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = "#fffdf8";
    ctx.fillRect(0, 0, w, h);
    ctx.strokeStyle = "#e8e0d5";
    ctx.strokeRect(12, 12, w - 24, h - 24);

    const mode = modes[i];
    let x = 24 + ((tick * (i + 1)) % (w - 48));
    let y = h / 2 + Math.sin((tick + i * 40) * 0.05) * 50;
    if (i === 1) {
      x = 24 + ((tick * 0.7) % (w - 48));
      y = h / 2 + (1 - signal) * 60 - 30;
    }
    if (i === 2) {
      x = 24 + (((Math.floor(tick / 20) * 28) % (w - 48)));
      y = 24 + ((Math.floor(tick / 20) * 18) % (h - 48));
    }

    ctx.fillStyle = mode.hue;
    ctx.beginPath();
    ctx.arc(x, y, 9, 0, Math.PI * 2);
    ctx.fill();
  });
}

function mapX(x, width) {
  return 60 + x * (width - 100);
}

function mapY(y, height) {
  return height - 50 - y * (height - 80);
}

function link(ctx, from, to) {
  ctx.beginPath();
  ctx.moveTo(from.x, from.y + 28);
  ctx.lineTo(to.x, to.y - 28);
  ctx.stroke();
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

frame();
