const PREFIX = "com.flowingspdg.roland.v60hd.";

function addOption(select, value, label) {
  const opt = document.createElement("option");
  opt.value = value;
  opt.textContent = label;
  select.appendChild(opt);
}

function fillVideoSources(select) {
  addOption(select, "sdi:1", "SDI IN 1");
  addOption(select, "sdi:2", "SDI IN 2");
  addOption(select, "sdi:3", "SDI IN 3");
  addOption(select, "sdi:4", "SDI IN 4");
  addOption(select, "hdmi:5", "HDMI IN 5");
  addOption(select, "hdmi_rgb:6", "HDMI/RGB IN 6");
  addOption(select, "still:7", "STILL/BKG IN 7");
  addOption(select, "still:8", "STILL/BKG IN 8");
}

function fillSelects() {
  document.querySelectorAll("select.video-source").forEach((el) => {
    if (el.options.length === 0) fillVideoSources(el);
  });
  const slot = document.getElementById("slot");
  if (slot && slot.options.length === 0) {
    for (let i = 1; i <= 8; i++) addOption(slot, String(i), String(i));
  }
}

function applyActionVisibility() {
  const short = (actionInfo.action || "").replace(PREFIX, "");
  document.querySelectorAll("[data-actions]").forEach((el) => {
    const allowed = el.getAttribute("data-actions").split(/\s+/);
    const show = allowed.includes("*") || allowed.includes(short);
    el.style.display = show ? "" : "none";
  });
  const tally = document.getElementById("tally_check");
  if (tally && !tally.value) {
    if (short === "select.pgm") tally.value = "pgm";
    if (short === "select.pst") tally.value = "prv";
  }
  applyConnectionUi();
}

function renderEndpoints(endpoints) {
  const pick = document.getElementById("connection_pick");
  if (!pick) return;
  const host = document.getElementById("host");
  const mode = document.getElementById("connection_mode");
  pick.innerHTML = "";
  addOption(pick, "manual", "Enter IP");
  (endpoints || []).forEach((endpoint) => {
    addOption(pick, endpoint.host, `${endpoint.host} · ${endpoint.status}`);
  });
  const savedValue = host && host.value.trim() ? host.value.trim() : "";
  const hasMatch = savedValue && [...pick.options].some((opt) => opt.value === savedValue);
  const others = (endpoints || []).filter((endpoint) => endpoint.host !== savedValue);
  if (mode && mode.value === "saved" && hasMatch) {
    pick.value = savedValue;
  } else if (mode && !mode.value && hasMatch && others.length > 0) {
    pick.value = savedValue;
    mode.value = "saved";
  } else {
    pick.value = "manual";
    if (mode && !mode.value) mode.value = "manual";
  }
  applyConnectionUi();
}

function applyConnectionUi() {
  const pick = document.getElementById("connection_pick");
  const manual = !pick || pick.value === "manual";
  const short = (actionInfo.action || "").replace(PREFIX, "");
  document.querySelectorAll("[data-manual-only]").forEach((el) => {
    const allowed = (el.getAttribute("data-actions") || "*").split(/\s+/);
    const actionShow = allowed.includes("*") || allowed.includes(short);
    el.style.display = actionShow && manual ? "" : "none";
  });
}

function onConnectionPick() {
  const pick = document.getElementById("connection_pick");
  const mode = document.getElementById("connection_mode");
  const host = document.getElementById("host");
  if (!pick || !mode) return;
  if (pick.value === "manual") {
    mode.value = "manual";
  } else {
    mode.value = "saved";
    if (host) host.value = pick.value;
  }
  applyConnectionUi();
  setSettings();
}

function attachPiMessages() {
  if (!websocket) return;
  const original = websocket.onmessage;
  websocket.onmessage = function (evt) {
    const jsonObj = JSON.parse(evt.data);
    if (jsonObj.event === "sendToPropertyInspector" && jsonObj.payload) {
      if (jsonObj.payload.status) {
        const el = document.getElementById("connectionStatus");
        if (el) el.textContent = jsonObj.payload.status;
      }
      if (jsonObj.payload.endpoints) {
        renderEndpoints(jsonObj.payload.endpoints);
      }
      return;
    }
    if (original) original(evt);
    applyConnectionUi();
  };
}

function testConnection() {
  const pick = document.getElementById("connection_pick");
  if (pick && pick.value !== "manual") return;
  const status = document.getElementById("connectionStatus");
  if (status) status.textContent = "Testing…";
  sendPayloadToPlugin({
    command: "test_connection",
    host: (document.getElementById("host") || {}).value || "",
  });
}

document.addEventListener("websocketCreate", () => {
  fillSelects();
  applyActionVisibility();
  attachPiMessages();
});
