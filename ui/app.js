// MeikiCtrl — interface. Parle au moteur Rust via les commandes Tauri.
const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

// Bits des boutons DualSense (identiques à src/pad.rs)
const BTN = {
  square: 1 << 0, cross: 1 << 1, circle: 1 << 2, triangle: 1 << 3, l1: 1 << 4, r1: 1 << 5,
  create: 1 << 8, options: 1 << 9, l3: 1 << 10, r3: 1 << 11, ps: 1 << 12, touch: 1 << 13,
  mute: 1 << 14, up: 1 << 15, down: 1 << 16, left: 1 << 17, right: 1 << 18,
};
const CHIPS = [
  ["Croix", BTN.cross], ["Rond", BTN.circle], ["Carré", BTN.square], ["Triangle", BTN.triangle],
  ["L1", BTN.l1], ["R1", BTN.r1], ["L3", BTN.l3], ["R3", BTN.r3],
  ["↑", BTN.up], ["↓", BTN.down], ["←", BTN.left], ["→", BTN.right],
  ["Create", BTN.create], ["Options", BTN.options], ["PS", BTN.ps], ["Pavé", BTN.touch], ["Micro", BTN.mute],
];
// Même ordre que engine::SOURCES (src/engine.rs)
const SOURCES = ["Croix", "Rond", "Carré", "Triangle", "L1", "R1", "L3", "R3", "Create", "Pavé tactile", "Options", "PS", "Croix ↑", "Croix ↓", "Croix ←", "Croix →", "Micro"];
const DEFAULT_REMAP = [0x1000, 0x2000, 0x4000, 0x8000, 0x0100, 0x0200, 0x0040, 0x0080, 0x0020, 0x0020, 0x0010, 0x0400, 0x0001, 0x0002, 0x0004, 0x0008, 0];
const TARGETS = [
  [0, "— Rien —"], [0x1000, "A"], [0x2000, "B"], [0x4000, "X"], [0x8000, "Y"],
  [0x0100, "LB"], [0x0200, "RB"], [0x0040, "Clic stick gauche (LS)"], [0x0080, "Clic stick droit (RS)"],
  [0x0020, "Back"], [0x0010, "Start"], [0x0400, "Guide"],
  [0x0001, "Croix ↑"], [0x0002, "Croix ↓"], [0x0004, "Croix ←"], [0x0008, "Croix →"],
];
const CURVES = [[1.0, "Linéaire"], [1.5, "Visée FPS (précis au centre)"], [2.2, "Précision max"], [0.7, "Réactif (rapide au centre)"]];
const COLORS = [
  ["Cyan", [0.0, 0.85, 1.0]], ["Rouge", [1.0, 0.12, 0.22]], ["Violet", [0.62, 0.25, 1.0]],
  ["Vert", [0.1, 1.0, 0.45]], ["Or", [1.0, 0.72, 0.1]], ["Blanc", [0.9, 0.95, 1.0]],
];

let S = null; // réglages complets (profils, overlay…)
let edit = 0; // profil en cours d'édition
let live = null;

// ---------- Navigation ----------
function goto(page) {
  $$(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.page === page));
  $$(".page").forEach((p) => p.classList.toggle("active", p.id === "page-" + page));
}
$$(".nav-item").forEach((b) => b.addEventListener("click", () => goto(b.dataset.page)));
$$("[data-goto]").forEach((b) => b.addEventListener("click", () => goto(b.dataset.goto)));

// ---------- Sauvegarde (regroupée) ----------
let saveTimer = null;
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => invoke("save_settings", { settings: S }), 200);
}
const P = () => S.profiles[edit];

// ---------- Puces de test ----------
const chipEls = CHIPS.map(([name, bit]) => {
  const el = document.createElement("span");
  el.className = "chip";
  el.textContent = name;
  $("chips").appendChild(el);
  return [el, bit];
});

// ---------- Interrupteurs overlay / émulation ----------
function bindToggle(cls, command) {
  $$(cls).forEach((el) =>
    el.addEventListener("change", () => {
      $$(cls).forEach((o) => (o.checked = el.checked));
      invoke(command, { on: el.checked });
    })
  );
}
bindToggle(".t-overlay", "set_overlay");
bindToggle(".t-emu", "set_emulation");
$("btn-recenter").addEventListener("click", () => invoke("recenter"));

// ---------- Profils ----------
function renderProfiles() {
  edit = Math.min(edit, S.profiles.length - 1);
  // listes déroulantes
  const opts = S.profiles.map((p, i) => `<option value="${i}">${esc(p.name)}</option>`).join("");
  $$(".profile-select").forEach((sel) => { sel.innerHTML = opts; sel.value = edit; });
  $("pr-default").innerHTML = opts;
  $("pr-default").value = S.defaultProfile;
  $("pr-auto").checked = S.autoProfiles;
  // liste
  $("pr-list").innerHTML = S.profiles
    .map((p, i) => `<button class="pitem ${i === edit ? "sel" : ""}" data-i="${i}">
        <span>${esc(p.name)}</span><small>${p.exe ? esc(p.exe) : "aucun jeu lié"}</small></button>`)
    .join("");
  $$("#pr-list .pitem").forEach((b) => b.addEventListener("click", () => { edit = +b.dataset.i; renderAll(); }));
  $("pr-name").value = P().name;
  $("pr-exe").value = P().exe;
  $("pr-del").disabled = S.profiles.length <= 1;
  $("pr-block").checked = !!P().blockEmulation;
}
function esc(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
}
$$(".profile-select").forEach((sel) => sel.addEventListener("change", () => { edit = +sel.value; renderAll(); }));
$("pr-default").addEventListener("change", (e) => { S.defaultProfile = +e.target.value; save(); });
$("pr-auto").addEventListener("change", (e) => { S.autoProfiles = e.target.checked; save(); });
$("pr-name").addEventListener("input", (e) => { P().name = e.target.value || "Sans nom"; save(); renderProfilesLight(); });
$("pr-exe").addEventListener("input", (e) => { P().exe = e.target.value.trim().toLowerCase(); save(); renderProfilesLight(); });
$("pr-block").addEventListener("change", (e) => { P().blockEmulation = e.target.checked; save(); });
$("pr-use-game").addEventListener("click", () => {
  const g = live && live.sys.lastGameExe;
  if (g) { P().exe = g; $("pr-exe").value = g; save(); renderProfilesLight(); }
});
function renderProfilesLight() {
  // met à jour noms/exe sans toucher aux champs en cours de saisie
  $$("#pr-list .pitem").forEach((b) => {
    const p = S.profiles[+b.dataset.i];
    b.querySelector("span").textContent = p.name;
    b.querySelector("small").textContent = p.exe || "aucun jeu lié";
  });
  $$(".profile-select option, #pr-default option").forEach((o) => (o.textContent = S.profiles[+o.value].name));
}
$("pr-new").addEventListener("click", () => {
  const base = JSON.parse(JSON.stringify(S.profiles[0]));
  Object.assign(base, {
    name: "Nouveau profil", exe: "", leftDeadzone: 0.08, rightDeadzone: 0.08, leftOuter: 0.98, rightOuter: 0.98,
    leftAntiDeadzone: 0, rightAntiDeadzone: 0, leftCurve: 1, rightCurve: 1, triggerDeadzone: 0.02,
    triggerCurve: 1, hairTrigger: false, blockEmulation: false, remap: DEFAULT_REMAP.slice(),
  });
  S.profiles.push(base);
  edit = S.profiles.length - 1;
  save(); renderAll();
});
$("pr-dup").addEventListener("click", () => {
  const c = JSON.parse(JSON.stringify(P()));
  c.name = P().name + " (copie)";
  c.exe = "";
  S.profiles.push(c);
  edit = S.profiles.length - 1;
  save(); renderAll();
});
$("pr-del").addEventListener("click", () => {
  if (S.profiles.length <= 1) return;
  S.profiles.splice(edit, 1);
  if (S.defaultProfile >= S.profiles.length) S.defaultProfile = 0;
  edit = Math.max(0, edit - 1);
  save(); renderAll();
});

// ---------- Sticks & gâchettes ----------
$$(".curve").forEach((sel) => {
  sel.innerHTML = CURVES.map(([v, n]) => `<option value="${v}">${n}</option>`).join("") + `<option value="custom" hidden>Perso</option>`;
  sel.addEventListener("change", () => { P()[sel.dataset.key] = +sel.value; save(); });
});
$$("input[type=range][data-key]").forEach((el) =>
  el.addEventListener("input", () => {
    P()[el.dataset.key] = el.value / 100;
    $$(`[data-out="${el.dataset.key}"]`).forEach((o) => (o.textContent = el.value + " %"));
    save();
  })
);
$("hair").addEventListener("change", (e) => { P().hairTrigger = e.target.checked; save(); });
function renderSticks() {
  const p = P();
  $$("input[type=range][data-key]").forEach((el) => {
    const v = Math.round(p[el.dataset.key] * 100);
    el.value = v;
    $$(`[data-out="${el.dataset.key}"]`).forEach((o) => (o.textContent = v + " %"));
  });
  $$(".curve").forEach((sel) => {
    const v = p[sel.dataset.key];
    const known = CURVES.find(([c]) => Math.abs(c - v) < 0.01);
    sel.value = known ? String(known[0]) : "custom";
  });
  $("hair").checked = p.hairTrigger;
}
$$("[data-calib]").forEach((btn) =>
  btn.addEventListener("click", async () => {
    const stick = btn.dataset.calib;
    const note = $("calib-" + stick);
    btn.disabled = true;
    note.className = "note";
    note.textContent = "Ne touche pas au stick… mesure en cours (2 s)";
    try {
      const dz = await invoke("calibrate", { stick, profile: edit });
      P()[stick === "left" ? "leftDeadzone" : "rightDeadzone"] = dz;
      renderSticks();
      note.className = "note ok";
      note.textContent = `Zone morte réglée à ${Math.round(dz * 100)} %. La dérive est corrigée.`;
    } catch (e) {
      note.className = "note err";
      note.textContent = e;
    }
    btn.disabled = false;
  })
);

// ---------- Remappage ----------
function renderRemap() {
  const opts = TARGETS.map(([v, n]) => `<option value="${v}">${n}</option>`).join("");
  $("remap").innerHTML = SOURCES.map(
    (name, i) => `<div class="rrow"><span>${name}</span><span class="arrow">→</span><select data-i="${i}">${opts}</select></div>`
  ).join("");
  $$("#remap select").forEach((sel) => {
    const i = +sel.dataset.i;
    sel.value = P().remap[i];
    sel.classList.toggle("changed", P().remap[i] !== DEFAULT_REMAP[i]);
    sel.addEventListener("change", () => {
      P().remap[i] = +sel.value;
      sel.classList.toggle("changed", P().remap[i] !== DEFAULT_REMAP[i]);
      save();
    });
  });
}
$("remap-reset").addEventListener("click", () => { P().remap = DEFAULT_REMAP.slice(); save(); renderRemap(); });

// ---------- Émulation : HidHide ----------
$("hide").addEventListener("change", (e) => { S.hideController = e.target.checked; save(); });

// ---------- Overlay ----------
$$("#corners button").forEach((b) =>
  b.addEventListener("click", () => { S.overlay.corner = +b.dataset.corner; save(); renderOverlay(); })
);
$$("input[data-ov]").forEach((el) =>
  el.addEventListener("input", () => {
    S.overlay[el.dataset.ov] = el.value / 100;
    $$(`[data-out-ov="${el.dataset.ov}"]`).forEach((o) => (o.textContent = el.value + " %"));
    save();
  })
);
$("swatches").innerHTML = COLORS.map(
  ([n, c], i) => `<button class="sw" data-i="${i}" title="${n}" style="--c: rgb(${c.map((x) => Math.round(x * 255)).join(",")})"><span>${n}</span></button>`
).join("");
$$("#swatches .sw").forEach((b) =>
  b.addEventListener("click", () => { S.overlay.accent = COLORS[+b.dataset.i][1].slice(); save(); renderOverlay(); })
);
$$("#models button").forEach((b) =>
  b.addEventListener("click", () => { S.overlay.model = +b.dataset.model; save(); renderOverlay(); })
);
function renderOverlay() {
  const o = S.overlay;
  $$("#models button").forEach((b) => b.classList.toggle("sel", +b.dataset.model === o.model));
  $$("#corners button").forEach((b) => b.classList.toggle("sel", +b.dataset.corner === o.corner));
  $$("input[data-ov]").forEach((el) => {
    const v = Math.round(o[el.dataset.ov] * 100);
    el.value = v;
    $$(`[data-out-ov="${el.dataset.ov}"]`).forEach((x) => (x.textContent = v + " %"));
  });
  $$("#swatches .sw").forEach((b) => {
    const c = COLORS[+b.dataset.i][1];
    b.classList.toggle("sel", c.every((x, k) => Math.abs(x - o.accent[k]) < 0.01));
  });
  document.documentElement.style.setProperty("--cyan", `rgb(${o.accent.map((x) => Math.round(x * 255)).join(",")})`);
}

// ---------- Système ----------
$("autostart").addEventListener("change", async (e) => {
  try {
    const on = await invoke("set_autostart", { on: e.target.checked });
    e.target.checked = on;
    $("autostart-note").className = "note ok";
    $("autostart-note").textContent = on ? "MeikiCtrl se lancera avec Windows." : "Démarrage automatique désactivé.";
  } catch (err) {
    e.target.checked = !e.target.checked;
    $("autostart-note").className = "note err";
    $("autostart-note").textContent = err;
  }
});

// ---------- Mode performance ----------
$("proc-prio").addEventListener("change", (e) => {
  S.processPriority = +e.target.value;
  invoke("set_process_priority", { level: S.processPriority });
});
$("admin-mode").addEventListener("change", async (e) => {
  const on = e.target.checked;
  const note = $("admin-note");
  note.className = "note";
  note.textContent = on ? "Windows va demander l'autorisation administrateur…" : "";
  try {
    const msg = await invoke("set_admin_mode", { on });
    S.adminMode = on;
    if (msg === "restart") {
      note.className = "note ok";
      note.textContent = "MeikiCtrl redémarre en administrateur…";
    } else {
      note.className = "note ok";
      note.textContent = msg || (on ? "Mode administrateur actif." : "Mode administrateur coupé.");
    }
  } catch (err) {
    e.target.checked = !on;
    S.adminMode = !on;
    note.className = "note err";
    note.textContent = err;
  }
});

function renderAll() {
  renderProfiles();
  renderSticks();
  renderRemap();
  renderOverlay();
  $("hide").checked = S.hideController;
  $("proc-prio").value = S.processPriority;
  $("admin-mode").checked = S.adminMode;
}

// ---------- Dessin des sticks ----------
function drawStick(canvas, raw, out, dz, outer) {
  const ctx = canvas.getContext("2d");
  const w = canvas.width, c = w / 2, r = c - 10;
  const accent = getComputedStyle(document.documentElement).getPropertyValue("--cyan").trim() || "#00d9ff";
  ctx.clearRect(0, 0, w, w);
  ctx.strokeStyle = "#20242d";
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.arc(c, c, r, 0, Math.PI * 2); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(c - r, c); ctx.lineTo(c + r, c); ctx.moveTo(c, c - r); ctx.lineTo(c, c + r); ctx.stroke();
  if (outer !== undefined) {
    ctx.setLineDash([4, 5]);
    ctx.strokeStyle = "rgba(255,255,255,0.18)";
    ctx.beginPath(); ctx.arc(c, c, r * outer, 0, Math.PI * 2); ctx.stroke();
    ctx.setLineDash([]);
  }
  if (dz !== undefined) {
    ctx.fillStyle = "rgba(255,46,77,0.18)";
    ctx.strokeStyle = "rgba(255,46,77,0.6)";
    ctx.beginPath(); ctx.arc(c, c, Math.max(r * dz, 1), 0, Math.PI * 2); ctx.fill(); ctx.stroke();
  }
  ctx.fillStyle = "#6b7383";
  ctx.beginPath(); ctx.arc(c + raw[0] * r, c - raw[1] * r, 5, 0, Math.PI * 2); ctx.fill();
  ctx.shadowColor = accent;
  ctx.shadowBlur = 12;
  ctx.fillStyle = accent;
  ctx.beginPath(); ctx.arc(c + out[0] * r, c - out[1] * r, 7, 0, Math.PI * 2); ctx.fill();
  ctx.shadowBlur = 0;
}

// ---------- Mise à jour en direct ----------
function renderLive(st) {
  const l = st.live, sys = st.sys;
  const name = !l.connected ? "Aucune manette" : l.xbox ? "Manette Xbox" : l.edge ? "DualSense Edge" : "DualSense";
  $("pad-name").textContent = name;
  $("pad-badge").textContent = l.connected ? "Connectée" : "Déconnectée";
  $("pad-badge").classList.toggle("on", l.connected);
  $("pad-conn").textContent = !l.connected ? "—" : l.xbox ? "XInput" : l.bluetooth ? "Bluetooth" : "USB";
  $("pad-batt").textContent = l.connected && !l.xbox ? `${l.battery} %${l.charging ? " · en charge" : ""}` : "—";
  $("batt-bar").style.width = (l.connected && !l.xbox ? l.battery : 0) + "%";
  $("model-hint").textContent = sys.xboxModel
    ? "Automatique : l'overlay affiche la manette que tu as branchée."
    : "Le modèle Xbox n'est pas encore installé : l'overlay affiche la DualSense.";
  $("foot-pad").textContent = "Manette : " + (l.connected ? name : "—");

  const active = S.profiles[sys.activeProfile] || S.profiles[0];
  $("foot-profile").textContent = "Profil : " + active.name;
  $("dash-profile").textContent = active.name;
  $("dash-game").textContent = sys.activeByGame ? `Activé par ${sys.lastGameExe}` : "Profil hors jeu";
  $("pr-detect").textContent = "Jeu détecté : " + (sys.lastGameExe || "—");

  chipEls.forEach(([el, bit]) => el.classList.toggle("on", (l.buttons & bit) !== 0));
  $("bar-l2").style.width = l.l2 * 100 + "%";
  $("bar-r2").style.width = l.r2 * 100 + "%";

  $$(".t-overlay").forEach((el) => (el.checked = st.overlay));
  $$(".t-emu").forEach((el) => (el.checked = st.emulation));

  // Émulation
  let text, cls;
  if (l.xbox) { text = "Ta manette est déjà une manette Xbox : pas besoin d'émulation."; cls = "ok"; }
  else if (!st.emulation) { text = "Émulation coupée."; cls = ""; }
  else if (sys.emulationBlockedBy) { text = `En pause : « ${sys.emulationBlockedBy} » bloque les manettes virtuelles. Ta DualSense est utilisée en direct.`; cls = ""; }
  else if (l.emulationError) { text = l.emulationError; cls = "err"; }
  else if (l.emulationActive) { text = "Active : les jeux voient une manette Xbox 360."; cls = "ok"; }
  else if (!l.connected) { text = "En attente de ta manette…"; cls = ""; }
  else { text = "Branchement de la manette virtuelle…"; cls = ""; }
  $("e-state").textContent = text;
  $("e-state").className = "state " + cls;
  $("q-emu-state").textContent = st.emulation ? text : "";
  $("q-emu-state").className = "note " + cls;

  // HidHide
  let h, hc = "";
  if (!sys.hidhideInstalled) { h = "HidHide n'est pas installé. Télécharge-le sur github.com/nefarius/HidHide/releases, puis relance MeikiCtrl."; hc = "err"; }
  else if (sys.hidhideBusy) h = "Autorisation administrateur demandée…";
  else if (sys.hidhideError) { h = sys.hidhideError; hc = "err"; }
  else if (sys.hidhideHidden) { h = "DualSense masquée : les jeux ne voient que la manette Xbox."; hc = "ok"; }
  else if (S.hideController && sys.emulationBlockedBy) h = `DualSense visible : « ${sys.emulationBlockedBy} » est lancé.`;
  else if (S.hideController && !st.emulation) h = "S'activera avec l'émulation.";
  else if (S.hideController && !l.connected) h = "En attente de ta manette…";
  else h = "DualSense visible par les jeux.";
  $("h-state").textContent = h;
  $("h-state").className = "state " + hc;
  $("hide").disabled = !sys.hidhideInstalled;

  if (document.activeElement !== $("autostart")) $("autostart").checked = sys.autostart;
  const gp = sys.gpuPriority || "—";
  $("gpu-prio").textContent = gp.charAt(0).toUpperCase() + gp.slice(1);
  $("gpu-prio").className = "perf-v " + (gp === "temps réel" ? "good" : gp === "haute" ? "mid" : "bad");
  $("elevated").textContent = sys.elevated ? "Oui" : "Non";
  $("elevated").className = "perf-v " + (sys.elevated ? "good" : "mid");

  drawStick($("mini-l"), l.rawL, l.outL);
  drawStick($("mini-r"), l.rawR, l.outR);
  if ($("page-sticks").classList.contains("active")) {
    const p = P();
    drawStick($("view-l"), l.rawL, l.outL, p.leftDeadzone, p.leftOuter);
    drawStick($("view-r"), l.rawR, l.outR, p.rightDeadzone, p.rightOuter);
  }
}

async function tick() {
  try {
    live = await invoke("get_live");
    renderLive(live);
  } catch (e) {
    console.error(e);
  }
  setTimeout(tick, 33); // ~30 fois par seconde, seulement quand la fenêtre est ouverte
}

(async () => {
  S = await invoke("get_settings");
  renderAll();
  tick();
})();
