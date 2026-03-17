const state = {
  socket: null,
  pending: new Map(),
  initialized: false,
  nextRequestId: 1,
  currentThreadId: "",
  session: null,
  thread: null,
  selectedTeamId: null,
};

const elements = {
  wsUrl: document.querySelector("#ws-url"),
  threadId: document.querySelector("#thread-id"),
  connectButton: document.querySelector("#connect-button"),
  attachButton: document.querySelector("#attach-button"),
  refreshButton: document.querySelector("#refresh-button"),
  sendButton: document.querySelector("#send-button"),
  connectionPill: document.querySelector("#connection-pill"),
  statusText: document.querySelector("#status-text"),
  overviewGrid: document.querySelector("#overview-grid"),
  teamGrid: document.querySelector("#team-grid"),
  selectedTeamSummary: document.querySelector("#selected-team-summary"),
  governanceActions: document.querySelector("#governance-actions"),
  artifactActions: document.querySelector("#artifact-actions"),
  resourceSummary: document.querySelector("#resource-summary"),
  readingPath: document.querySelector("#reading-path"),
  documentViewer: document.querySelector("#document-viewer"),
  instructionInput: document.querySelector("#instruction-input"),
  threadView: document.querySelector("#thread-view"),
};

function deriveDefaultWsUrl() {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}`;
}

function setStatus(kind, text) {
  elements.connectionPill.className = "pill";
  if (kind === "ready") {
    elements.connectionPill.classList.add("pill-ready");
    elements.connectionPill.textContent = "Connected";
  } else if (kind === "loading" || kind === "warn") {
    elements.connectionPill.classList.add("pill-warn");
    elements.connectionPill.textContent = kind === "warn" ? "Watching" : "Working";
  } else if (kind === "error") {
    elements.connectionPill.classList.add("pill-danger");
    elements.connectionPill.textContent = "Attention";
  } else {
    elements.connectionPill.classList.add("pill-neutral");
    elements.connectionPill.textContent = "Disconnected";
  }
  elements.statusText.textContent = text;
}

function isAbsolutePath(path) {
  return /^[A-Za-z]:[\\/]/.test(path) || path.startsWith("/") || path.startsWith("\\\\");
}

function dirname(path) {
  if (!path) {
    return "";
  }
  const normalized = path.replace(/[\\/]+$/, "");
  const separator = normalized.includes("\\") ? "\\" : "/";
  const index = normalized.lastIndexOf(separator);
  if (index <= 0) {
    return normalized;
  }
  return normalized.slice(0, index);
}

function joinPath(base, relative) {
  if (!base) {
    return relative;
  }
  const separator = base.includes("\\") ? "\\" : "/";
  const cleanBase = base.replace(/[\\/]+$/, "");
  const cleanRelative = relative.replace(/^[\\/]+/, "");
  return `${cleanBase}${separator}${cleanRelative}`;
}

function workspaceRoot() {
  const indexPath = state.session?.teamStateIndexPath;
  if (!indexPath || !isAbsolutePath(indexPath)) {
    return null;
  }
  return dirname(dirname(dirname(indexPath)));
}

function resolveReadablePath(path) {
  if (!path) {
    return null;
  }
  if (isAbsolutePath(path)) {
    return path;
  }
  const root = workspaceRoot();
  return root ? joinPath(root, path) : null;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function decodeBase64(dataBase64) {
  const binary = window.atob(dataBase64);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

function sendRequest(method, params) {
  if (!state.socket || state.socket.readyState !== WebSocket.OPEN || !state.initialized) {
    return Promise.reject(new Error("socket is not initialized"));
  }
  const id = state.nextRequestId++;
  state.socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => {
    state.pending.set(id, { resolve, reject, method });
  });
}

function sendNotification(method, params) {
  if (!state.socket || state.socket.readyState !== WebSocket.OPEN) {
    return;
  }
  const message = { method };
  if (params !== undefined) {
    message.params = params;
  }
  state.socket.send(JSON.stringify(message));
}

function onSocketClose() {
  state.initialized = false;
  setStatus("idle", "Connection closed.");
}

async function onSocketMessage(event) {
  let message;
  try {
    message = JSON.parse(event.data);
  } catch (error) {
    console.error("Failed to parse websocket payload", error);
    return;
  }

  if (Object.prototype.hasOwnProperty.call(message, "id")) {
    const pending = state.pending.get(message.id);
    if (!pending) {
      return;
    }
    state.pending.delete(message.id);
    if (message.error) {
      pending.reject(new Error(message.error.message));
    } else {
      pending.resolve(message.result);
    }
    return;
  }

  const method = message.method;
  const params = message.params || {};
  if (method === "initialized") {
    return;
  }

  if (
    method === "teamWorkflow/sessionUpdated" &&
    params.session &&
    params.session.rootThreadId === state.currentThreadId
  ) {
    state.session = params.session;
    if (!state.selectedTeamId) {
      state.selectedTeamId = params.session.rootTeamId;
    }
    renderSession();
    return;
  }

  if (
    (method === "turn/completed" || method === "turn/aborted" || method === "thread/status/changed") &&
    params.threadId === state.currentThreadId
  ) {
    await refreshAttachedSession({ keepStatus: true });
  }
}

async function initializeSocket() {
  const wsUrl = elements.wsUrl.value.trim();
  if (!wsUrl) {
    throw new Error("websocket URL is required");
  }

  if (state.socket) {
    state.socket.close();
  }

  setStatus("loading", "Opening websocket and negotiating capabilities.");
  state.initialized = false;
  state.socket = new WebSocket(wsUrl);

  await new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      reject(new Error("websocket connection timed out"));
    }, 8000);

    state.socket.onopen = async () => {
      try {
        state.socket.onmessage = onSocketMessage;
        state.socket.onclose = onSocketClose;
        const initializeId = state.nextRequestId++;
        const initializePromise = new Promise((resolveInitialize, rejectInitialize) => {
          state.pending.set(initializeId, {
            method: "initialize",
            resolve: resolveInitialize,
            reject: rejectInitialize,
          });
        });
        state.socket.send(
          JSON.stringify({
            id: initializeId,
            method: "initialize",
            params: {
              clientInfo: {
                name: "codex_team_ops_ui",
                title: "Codex Team Ops UI",
                version: "0.1.0",
              },
              capabilities: {
                experimentalApi: true,
              },
            },
          }),
        );
        await initializePromise;
        sendNotification("initialized");
        state.initialized = true;
        window.clearTimeout(timer);
        setStatus("ready", "Connected. Attach a root scheduler thread to load the team session.");
        resolve();
      } catch (error) {
        window.clearTimeout(timer);
        reject(error);
      }
    };

    state.socket.onerror = () => {
      window.clearTimeout(timer);
      reject(new Error("websocket failed to open"));
    };
  });
}

async function attachThread() {
  const threadId = elements.threadId.value.trim();
  if (!threadId) {
    throw new Error("root thread ID is required");
  }
  state.currentThreadId = threadId;
  state.selectedTeamId = null;
  localStorage.setItem("codex.teamOps.threadId", threadId);
  await refreshAttachedSession({ keepStatus: false });
}

async function refreshAttachedSession({ keepStatus }) {
  if (!state.currentThreadId) {
    return;
  }
  if (!keepStatus) {
    setStatus("loading", `Loading team workflow session for ${state.currentThreadId}.`);
  }
  const [sessionResponse, threadResponse] = await Promise.all([
    sendRequest("teamWorkflow/sessionRead", {
      threadId: state.currentThreadId,
      recentTapeLimit: 24,
    }),
    sendRequest("thread/read", {
      threadId: state.currentThreadId,
      includeTurns: true,
    }),
  ]);
  state.session = sessionResponse.session;
  state.thread = threadResponse.thread;
  if (!state.selectedTeamId) {
    state.selectedTeamId = state.session.rootTeamId;
  }
  renderSession();
  renderThread();
  if (!keepStatus) {
    setStatus("ready", `Attached to root scheduler thread ${state.currentThreadId}.`);
  }
}

function metricCard(label, value, extra = "") {
  return `
    <article class="metric-card">
      <p class="metric-label">${escapeHtml(label)}</p>
      <p class="metric-value">${escapeHtml(value)}</p>
      ${extra ? `<p class="team-role">${escapeHtml(extra)}</p>` : ""}
    </article>
  `;
}

function renderOverview() {
  if (!state.session) {
    elements.overviewGrid.innerHTML = `<div class="empty-state">No team workflow session loaded yet.</div>`;
    return;
  }
  elements.overviewGrid.innerHTML = [
    metricCard("Root Role", state.session.rootRole, `Thread ${state.session.rootThreadId}`),
    metricCard("Phase", state.session.currentPhase),
    metricCard("Teams", String(state.session.activeTeamCount)),
    metricCard("Blocked", String(state.session.blockedTeamCount)),
    metricCard("Stale Resources", String(state.session.staleResourceCount)),
    metricCard("Max Depth", String(state.session.maxDepth)),
  ].join("");
}

function renderTeamGrid() {
  if (!state.session?.teams?.length) {
    elements.teamGrid.innerHTML = `<div class="empty-state">No visible teams.</div>`;
    return;
  }

  elements.teamGrid.innerHTML = state.session.teams
    .map((team) => {
      const blocked = (team.blockers || []).length > 0;
      const staleCount = (team.environment?.staleResources || []).length;
      const classes = [
        "team-card",
        team.teamId === state.selectedTeamId ? "is-selected" : "",
        blocked ? "is-blocked" : "",
        staleCount > 0 ? "is-stale" : "",
      ]
        .filter(Boolean)
        .join(" ");
      return `
        <article class="${classes}">
          <header>
            <div>
              <h3>${escapeHtml(team.nickname || team.teamId)}</h3>
              <p class="team-role">${escapeHtml(team.role)}</p>
            </div>
            <span class="tag">${escapeHtml(team.currentPhase)}</span>
          </header>
          <div class="tag-row">
            <span class="tag">Depth ${escapeHtml(team.depth)}</span>
            <span class="tag">${escapeHtml(team.kind)}</span>
            <span class="tag">${escapeHtml(team.producedArtifacts.length)} artifacts</span>
          </div>
          <div class="team-meta">
            <p>${blocked ? escapeHtml(team.blockers.join(" | ")) : "No active blockers."}</p>
            <p>${team.nextSteps?.length ? escapeHtml(team.nextSteps.join(" | ")) : "No next steps recorded."}</p>
            <p>${staleCount > 0 ? `${staleCount} stale resource(s) need cleanup.` : "No stale resources flagged."}</p>
          </div>
          <button class="ghost" data-team-id="${escapeHtml(team.teamId)}" type="button">Inspect Team</button>
        </article>
      `;
    })
    .join("");

  elements.teamGrid.querySelectorAll("[data-team-id]").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedTeamId = button.getAttribute("data-team-id");
      renderSession();
    });
  });
}

function collectArtifactEntries(team) {
  const entries = new Map();
  for (const artifact of team.producedArtifacts || []) {
    const readablePath = resolveReadablePath(artifact);
    entries.set(`artifact:${artifact}`, {
      label: artifact,
      path: readablePath,
      readable: Boolean(readablePath),
    });
  }
  for (const tapeEntry of team.recentTape || []) {
    for (const ref of tapeEntry.artifactRefs || []) {
      const path = typeof ref === "string" ? ref : "";
      const readablePath = resolveReadablePath(path);
      entries.set(`tape:${path}`, {
        label: path,
        path: readablePath,
        readable: Boolean(readablePath),
      });
    }
  }
  return Array.from(entries.values());
}

async function viewFile(path, label) {
  const readablePath = resolveReadablePath(path);
  if (!readablePath) {
    elements.documentViewer.textContent = `Unable to resolve a readable path for ${label}.`;
    return;
  }
  setStatus("loading", `Reading ${label}.`);
  try {
    const response = await sendRequest("fs/readFile", { path: readablePath });
    elements.documentViewer.textContent = decodeBase64(response.dataBase64);
    setStatus("ready", `Viewing ${label}.`);
  } catch (error) {
    elements.documentViewer.textContent = `Failed to read ${label}.\n\n${error.message}`;
    setStatus("error", `Failed to read ${label}.`);
  }
}

function renderSelectedTeam() {
  const team = state.session?.teams?.find((entry) => entry.teamId === state.selectedTeamId);
  if (!team) {
    elements.selectedTeamSummary.textContent = "Select a team to inspect details.";
    elements.governanceActions.innerHTML = "";
    elements.artifactActions.innerHTML = "";
    elements.resourceSummary.innerHTML = `<div class="empty-state">No team selected.</div>`;
    elements.readingPath.innerHTML = `<div class="empty-state">No tape entries available.</div>`;
    return;
  }

  elements.selectedTeamSummary.textContent =
    `${team.role} | phase ${team.currentPhase} | ${team.activeChildTeamIds.length} child team(s)`;

  const governanceEntries = [
    { label: "Open AGENT.md", path: state.session.globalGovernancePath },
    { label: "Open AGENT_TEAM.md", path: team.governanceDocPath },
  ];
  elements.governanceActions.innerHTML = governanceEntries
    .map(
      (entry, index) => `
        <button class="secondary" data-governance-index="${index}" type="button">
          ${escapeHtml(entry.label)}
        </button>
      `,
    )
    .join("");
  elements.governanceActions.querySelectorAll("[data-governance-index]").forEach((button) => {
    button.addEventListener("click", () => {
      const entry = governanceEntries[Number(button.getAttribute("data-governance-index"))];
      viewFile(entry.path, entry.label);
    });
  });

  const artifactEntries = collectArtifactEntries(team);
  elements.artifactActions.innerHTML =
    artifactEntries.length === 0
      ? `<div class="empty-state">No persisted artifacts for this team yet.</div>`
      : artifactEntries
          .map(
            (artifact, index) => `
              <button
                class="${artifact.readable ? "ghost" : "secondary"}"
                data-artifact-index="${index}"
                type="button"
                ${artifact.readable ? "" : "disabled"}
              >
                ${escapeHtml(artifact.label)}
              </button>
            `,
          )
          .join("");
  elements.artifactActions.querySelectorAll("[data-artifact-index]").forEach((button) => {
    button.addEventListener("click", () => {
      const entry = artifactEntries[Number(button.getAttribute("data-artifact-index"))];
      viewFile(entry.path, entry.label);
    });
  });

  const blockers = team.blockers || [];
  const staleResources = team.environment?.staleResources || [];
  const cleanupNotes = team.environment?.cleanupNotes || [];
  const resourceRows = [];
  if (blockers.length > 0) {
    resourceRows.push(
      `<div class="empty-state"><strong>Workflow blockers</strong><br />${escapeHtml(blockers.join("\n"))}</div>`,
    );
  }
  if (staleResources.length > 0) {
    resourceRows.push(
      `<ul class="resource-list">${staleResources
        .map(
          (resource) =>
            `<li><strong>${escapeHtml(resource.resourceId)}</strong> | ${escapeHtml(resource.kind)} | ${escapeHtml(resource.status)}</li>`,
        )
        .join("")}</ul>`,
    );
  }
  if (cleanupNotes.length > 0) {
    resourceRows.push(
      `<ul class="list-plain">${cleanupNotes
        .map((note) => `<li>${escapeHtml(note)}</li>`)
        .join("")}</ul>`,
    );
  }
  if (resourceRows.length === 0) {
    resourceRows.push(`<div class="empty-state">No active blockers or stale cleanup issues.</div>`);
  }
  elements.resourceSummary.innerHTML = resourceRows.join("");

  elements.readingPath.innerHTML =
    team.recentTape?.length > 0
      ? team.recentTape
          .map(
            (entry) => `
              <article class="timeline-entry">
                <h4>${escapeHtml(entry.kind)} | ${escapeHtml(entry.createdAt)}</h4>
                <p>${escapeHtml(entry.summary)}</p>
                <p>${escapeHtml(entry.phase || "phase pending")}${
                  entry.counterpartTeamId ? ` | counterpart ${escapeHtml(entry.counterpartTeamId)}` : ""
                }</p>
              </article>
            `,
          )
          .join("")
      : `<div class="empty-state">No reading path entries recorded for this team.</div>`;
}

function summarizeThreadItem(item) {
  if (item.type === "userMessage") {
    const text = (item.content || [])
      .filter((entry) => entry.type === "text")
      .map((entry) => entry.text)
      .join("\n");
    return text || "[non-text user input]";
  }
  if (item.type === "agentMessage") {
    return item.text || "";
  }
  if (item.type === "reasoning") {
    return (item.summary || []).join(" ");
  }
  if (item.type === "plan") {
    return (item.items || []).map((entry) => `${entry.status}: ${entry.step}`).join("\n");
  }
  return `[${item.type}]`;
}

function renderThread() {
  if (!state.thread?.turns?.length) {
    elements.threadView.innerHTML = `<div class="empty-state">No root-scheduler turns loaded yet.</div>`;
    return;
  }

  const turns = state.thread.turns.slice(-8).reverse();
  elements.threadView.innerHTML = turns
    .map(
      (turn) => `
        <article class="thread-turn">
          <header>
            <div>
              <h4>${escapeHtml(turn.id)}</h4>
              <p>${escapeHtml(turn.status)}</p>
            </div>
          </header>
          <ul class="thread-items">
            ${(turn.items || [])
              .map(
                (item) => `
                  <li>
                    <strong>${escapeHtml(item.type)}</strong>
                    <p>${escapeHtml(summarizeThreadItem(item))}</p>
                  </li>
                `,
              )
              .join("")}
          </ul>
        </article>
      `,
    )
    .join("");
}

function renderSession() {
  renderOverview();
  renderTeamGrid();
  renderSelectedTeam();
}

async function refreshUntilTurnSettles(turnId) {
  for (let attempt = 0; attempt < 90; attempt += 1) {
    await new Promise((resolve) => window.setTimeout(resolve, 1500));
    await refreshAttachedSession({ keepStatus: true });
    const currentTurn = state.thread?.turns?.find((turn) => turn.id === turnId);
    if (!currentTurn || currentTurn.status !== "inProgress") {
      setStatus("ready", "Root scheduler turn finished. Session view refreshed.");
      return;
    }
  }
  setStatus("warn", "Turn is still running. Continue monitoring from the session view.");
}

async function sendInstruction() {
  const text = elements.instructionInput.value.trim();
  if (!text) {
    setStatus("error", "Instruction text must not be empty.");
    return;
  }
  if (!state.currentThreadId) {
    setStatus("error", "Attach a root scheduler thread first.");
    return;
  }

  elements.sendButton.disabled = true;
  setStatus("loading", "Sending instruction to the root scheduler.");
  try {
    const response = await sendRequest("turn/start", {
      threadId: state.currentThreadId,
      input: [
        {
          type: "text",
          text,
          textElements: [],
        },
      ],
    });
    elements.instructionInput.value = "";
    await refreshUntilTurnSettles(response.turn.id);
  } catch (error) {
    setStatus("error", `Failed to start turn: ${error.message}`);
  } finally {
    elements.sendButton.disabled = false;
  }
}

async function connectAndMaybeAttach() {
  try {
    await initializeSocket();
    if (elements.threadId.value.trim()) {
      await attachThread();
    }
  } catch (error) {
    setStatus("error", error.message);
  }
}

async function attachWithAutoConnect() {
  try {
    if (!state.initialized) {
      await initializeSocket();
    }
    await attachThread();
  } catch (error) {
    setStatus("error", error.message);
  }
}

function bootstrap() {
  elements.wsUrl.value = localStorage.getItem("codex.teamOps.wsUrl") || deriveDefaultWsUrl();
  const threadIdFromQuery = new URLSearchParams(window.location.search).get("threadId");
  elements.threadId.value =
    threadIdFromQuery || localStorage.getItem("codex.teamOps.threadId") || "";

  elements.connectButton.addEventListener("click", async () => {
    localStorage.setItem("codex.teamOps.wsUrl", elements.wsUrl.value.trim());
    await connectAndMaybeAttach();
  });
  elements.attachButton.addEventListener("click", async () => {
    localStorage.setItem("codex.teamOps.wsUrl", elements.wsUrl.value.trim());
    await attachWithAutoConnect();
  });
  elements.refreshButton.addEventListener("click", async () => {
    try {
      await refreshAttachedSession({ keepStatus: false });
    } catch (error) {
      setStatus("error", error.message);
    }
  });
  elements.sendButton.addEventListener("click", sendInstruction);
  setStatus("idle", "Enter a websocket URL and root thread ID to attach the operations view.");
}

bootstrap();
