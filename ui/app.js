// MeikiCtrl — interface. Parle au moteur Rust via les commandes Tauri.
const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

// Bits des boutons DualSense (identiques à src/pad.rs)
const BTN_NAMES = ["Carré", "Croix", "Rond", "Triangle", "L1", "R1", "L2", "R2", "Create", "Options", "L3", "R3", "PS", "Pavé tactile", "Micro", "Haut", "Bas", "Gauche", "Droite"];
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
  ["Blanc", [0.92, 0.95, 1.0]], ["Cyan", [0.0, 0.85, 1.0]], ["Rouge", [1.0, 0.12, 0.22]], ["Violet", [0.62, 0.25, 1.0]],
  ["Vert", [0.1, 1.0, 0.45]], ["Or", [1.0, 0.72, 0.1]],
];
const LIGHT_COLORS = [
  ["Blanc", [1, 1, 1]], ["Rouge", [1, 0.02, 0.05]], ["Orange", [1, 0.35, 0]], ["Or", [1, 0.75, 0]], ["Vert", [0.05, 1, 0.2]],
  ["Cyan", [0, 0.85, 1]], ["Bleu", [0, 0.2, 1]], ["Violet", [0.55, 0.1, 1]], ["Rose", [1, 0.15, 0.6]],
];
const PLAYER_COLORS = [[0, 0.25, 1], [1, 0.05, 0.1], [0.05, 1, 0.2], [1, 0.2, 0.7]];
const TRIGGER_FX = [[0, "Aucun effet"], [1, "Résistance"], [2, "Clic d'arme"]];
const LIGHT_HINTS = {
  0: "MeikiCtrl ne touche pas à la lumière : la manette ou le jeu gère.",
  1: "Toujours la même couleur.",
  2: "Vert, jaune puis rouge selon la batterie. Elle pulse en charge et clignote quand c'est presque vide.",
  3: "Chaque profil a sa couleur : la barre change toute seule avec le jeu.",
  4: "La couleur de ton numéro de joueur, comme sur PS5.",
  5: "Ta couleur, qui respire doucement.",
  6: "Toutes les couleurs qui défilent en boucle. Règle la vitesse juste en dessous.",
};
const KOFI = "https://ko-fi.com/meikitoow17";

let S = null; // réglages complets (profils, overlay…)
let edit = 0; // profil en cours d'édition
let live = null;
let health = null;

const rgb = (c) => `rgb(${c.map((x) => Math.round(x * 255)).join(",")})`;
const hexToRgb = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
const rgbToHex = (c) => "#" + c.map((x) => Math.round(x * 255).toString(16).padStart(2, "0")).join("");
const sameColor = (a, b) => a && b && a.every((x, k) => Math.abs(x - b[k]) < 0.01);
function esc(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
}
const colon = () => (LANG === "en" ? ": " : " : ");
const nf = () => new Intl.NumberFormat(LANG === "en" ? "en-US" : "fr-FR");
function fmtMin(m) {
  m = Math.round(m || 0);
  const h = Math.floor(m / 60), mm = m % 60;
  if (!h) return `${mm} min`;
  return mm ? `${h} h ${String(mm).padStart(2, "0")}` : `${h} h`;
}
function fmtDate(unix) {
  return new Date(unix * 1000).toLocaleString(LANG === "en" ? "en-GB" : "fr-FR", {
    weekday: "short", day: "numeric", month: "short", hour: "2-digit", minute: "2-digit",
  });
}

// ---------- Liens (Ko-fi, téléchargements) ----------
document.addEventListener("click", (e) => {
  const b = e.target.closest("[data-url]");
  if (b) invoke("open_url", { url: b.dataset.url });
});

// ---------- Navigation ----------
function goto(page) {
  $$(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.page === page));
  $$(".page").forEach((p) => p.classList.toggle("active", p.id === "page-" + page));
  document.querySelector("main").scrollTop = 0;
  if (page === "health" || page === "sessions" || page === "dashboard") pollHealth();
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

// ---------- Motif MEIKI / TOOW du héro ----------
(function pattern() {
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='260' height='64'>
    <style>text{font-family:Bahnschrift,'Arial Narrow',sans-serif;font-weight:700;font-size:27px;letter-spacing:.5px}</style>
    <rect x='0' y='0' width='130' height='32' fill='#fff'/><text x='65' y='27' text-anchor='middle' fill='#000'>MEIKI</text>
    <text x='195' y='27' text-anchor='middle' fill='#fff'>TOOW</text>
    <text x='65' y='59' text-anchor='middle' fill='#fff'>TOOW</text>
    <rect x='130' y='32' width='130' height='32' fill='#fff'/><text x='195' y='59' text-anchor='middle' fill='#000'>MEIKI</text></svg>`;
  document.documentElement.style.setProperty("--pattern", `url("data:image/svg+xml,${encodeURIComponent(svg)}")`);
})();

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
  const opts = S.profiles.map((p, i) => `<option value="${i}">${esc(p.name)}</option>`).join("");
  $$(".profile-select").forEach((sel) => { sel.innerHTML = opts; sel.value = edit; });
  $("pr-default").innerHTML = opts;
  $("pr-default").value = S.defaultProfile;
  $("pr-auto").checked = S.autoProfiles;
  $("pr-list").innerHTML = S.profiles
    .map((p, i) => `<button class="pitem ${i === edit ? "sel" : ""}" data-i="${i}">
        <span>${esc(p.name)}</span><small>${p.exe ? esc(p.exe) : t("aucun jeu lié")}</small></button>`)
    .join("");
  $$("#pr-list .pitem").forEach((b) => b.addEventListener("click", () => { edit = +b.dataset.i; renderAll(); }));
  $("pr-name").value = P().name;
  $("pr-exe").value = P().exe;
  $("pr-del").disabled = S.profiles.length <= 1;
  $("pr-block").checked = !!P().blockEmulation;
}
$$(".profile-select").forEach((sel) => sel.addEventListener("change", () => { edit = +sel.value; renderAll(); }));
$("pr-default").addEventListener("change", (e) => { S.defaultProfile = +e.target.value; save(); });
$("pr-auto").addEventListener("change", (e) => { S.autoProfiles = e.target.checked; save(); });
$("pr-name").addEventListener("input", (e) => { P().name = e.target.value || t("Sans nom"); save(); renderProfilesLight(); });
$("pr-exe").addEventListener("input", (e) => { P().exe = e.target.value.trim().toLowerCase(); save(); renderProfilesLight(); });
$("pr-block").addEventListener("change", (e) => { P().blockEmulation = e.target.checked; save(); });
$("pr-use-game").addEventListener("click", () => {
  const g = live && live.sys.lastGameExe;
  if (g) { P().exe = g; $("pr-exe").value = g; save(); renderProfilesLight(); }
});
function renderProfilesLight() {
  $$("#pr-list .pitem").forEach((b) => {
    const p = S.profiles[+b.dataset.i];
    b.querySelector("span").textContent = p.name;
    b.querySelector("small").textContent = p.exe || t("aucun jeu lié");
  });
  $$(".profile-select option, #pr-default option").forEach((o) => (o.textContent = S.profiles[+o.value].name));
}
function newProfile() {
  return {
    name: t("Nouveau profil"), exe: "", leftDeadzone: 0.08, rightDeadzone: 0.08, leftOuter: 0.98, rightOuter: 0.98,
    leftAntiDeadzone: 0, rightAntiDeadzone: 0, leftCurve: 1, rightCurve: 1, triggerDeadzone: 0.02,
    triggerCurve: 1, hairTrigger: false, blockEmulation: false, remap: DEFAULT_REMAP.slice(),
    lightColor: [1, 1, 1], triggerL: 0, triggerR: 0, triggerStrength: 0.6,
  };
}
$("pr-new").addEventListener("click", () => {
  S.profiles.push(newProfile());
  edit = S.profiles.length - 1;
  save(); renderAll();
});
$("pr-dup").addEventListener("click", () => {
  const c = JSON.parse(JSON.stringify(P()));
  c.name = P().name + " " + t("(copie)");
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
$$("input[type=range][data-key]").forEach((el) =>
  el.addEventListener("input", () => {
    P()[el.dataset.key] = el.value / 100;
    $$(`[data-out="${el.dataset.key}"]`).forEach((o) => (o.textContent = el.value + " %"));
    save();
  })
);
$$(".curve").forEach((sel) => sel.addEventListener("change", () => { P()[sel.dataset.key] = +sel.value; save(); }));
$("hair").addEventListener("change", (e) => { P().hairTrigger = e.target.checked; save(); });
function renderSticks() {
  const p = P();
  $$(".curve").forEach((sel) => {
    sel.innerHTML = CURVES.map(([v, n]) => `<option value="${v}">${t(n)}</option>`).join("") + `<option value="custom" hidden>${t("Perso")}</option>`;
    const v = p[sel.dataset.key];
    const known = CURVES.find(([c]) => Math.abs(c - v) < 0.01);
    sel.value = known ? String(known[0]) : "custom";
  });
  $$("input[type=range][data-key]").forEach((el) => {
    const v = Math.round(p[el.dataset.key] * 100);
    el.value = v;
    $$(`[data-out="${el.dataset.key}"]`).forEach((o) => (o.textContent = v + " %"));
  });
  $("hair").checked = p.hairTrigger;
}
$$("[data-calib]").forEach((btn) =>
  btn.addEventListener("click", async () => {
    const stick = btn.dataset.calib;
    const note = $("calib-" + stick);
    btn.disabled = true;
    note.className = "note";
    note.textContent = t("Ne touche pas au stick… mesure en cours (2 s)");
    try {
      const dz = await invoke("calibrate", { stick, profile: edit });
      P()[stick === "left" ? "leftDeadzone" : "rightDeadzone"] = dz;
      renderSticks();
      note.className = "note ok";
      note.textContent = t("Zone morte réglée à {n} %. La dérive est corrigée.", { n: Math.round(dz * 100) });
    } catch (e) {
      note.className = "note err";
      note.textContent = tErr(e);
    }
    btn.disabled = false;
  })
);

// ---------- Lumière & effets ----------
function makeSwatches(box, list, get, set) {
  box.innerHTML =
    list.map(([n, c], i) => `<button class="sw" data-i="${i}" title="${esc(t(n))}" style="--c:${rgb(c)}"></button>`).join("") +
    `<label class="sw custom" title="${esc(t("Couleur perso"))}"><input type="color" /></label>`;
  box.querySelectorAll(".sw[data-i]").forEach((b) =>
    b.addEventListener("click", () => { set(list[+b.dataset.i][1].slice()); save(); renderAll(); })
  );
  const input = box.querySelector("input[type=color]");
  input.value = rgbToHex(get());
  input.addEventListener("input", () => { set(hexToRgb(input.value)); save(); refreshSwatches(box, list, get); });
  refreshSwatches(box, list, get);
}
function refreshSwatches(box, list, get) {
  const cur = get();
  let found = false;
  box.querySelectorAll(".sw[data-i]").forEach((b) => {
    const on = sameColor(list[+b.dataset.i][1], cur);
    found = found || on;
    b.classList.toggle("sel", on);
  });
  const custom = box.querySelector(".sw.custom");
  custom.classList.toggle("sel", !found);
  if (!found) custom.style.background = rgb(cur);
  else custom.style.background = "";
}
$$("#light-modes button").forEach((b) => b.addEventListener("click", () => { S.lightbar.mode = +b.dataset.mode; save(); renderLight(); }));
$$("#players button").forEach((b) => b.addEventListener("click", () => { S.lightbar.player = +b.dataset.player; save(); renderLight(); }));
$("light-br").addEventListener("input", (e) => { S.lightbar.brightness = e.target.value / 100; $("light-br-out").textContent = e.target.value + " %"; save(); });
$("light-sp").addEventListener("input", (e) => { S.lightbar.speed = e.target.value / 100; $("light-sp-out").textContent = e.target.value + " %"; save(); });
$("light-flash").addEventListener("change", (e) => { S.lightbar.flashOnFire = e.target.checked; save(); renderLight(); });
$("flash-btn").addEventListener("change", (e) => { S.lightbar.flashButton = +e.target.value; save(); });
// Ordre du menu : les gâchettes et boutons de tir d'abord.
const FLASH_ORDER = [7, 6, 5, 4, 1, 0, 2, 3, 10, 11, 15, 16, 17, 18, 13, 8, 9, 12, 14];
$("trig-on").addEventListener("change", (e) => { S.adaptiveTriggers = e.target.checked; save(); renderLight(); });
$("trig-l").addEventListener("change", (e) => { P().triggerL = +e.target.value; save(); });
$("trig-r").addEventListener("change", (e) => { P().triggerR = +e.target.value; save(); });
$("trig-s").addEventListener("input", (e) => { P().triggerStrength = e.target.value / 100; $("trig-s-out").textContent = e.target.value + " %"; save(); });
function renderLight() {
  const L = S.lightbar;
  $$("#light-modes button").forEach((b) => b.classList.toggle("sel", +b.dataset.mode === L.mode));
  $$("#players button").forEach((b) => b.classList.toggle("sel", +b.dataset.player === L.player));
  $("light-hint").textContent = t(LIGHT_HINTS[L.mode] || "");
  $("light-color-box").hidden = !(L.mode === 1 || L.mode === 5);
  makeSwatches($("light-swatches"), LIGHT_COLORS, () => S.lightbar.color, (c) => (S.lightbar.color = c));
  makeSwatches($("profile-swatches"), LIGHT_COLORS, () => P().lightColor, (c) => (P().lightColor = c));
  $("light-speed-box").hidden = !(L.mode === 5 || L.mode === 6);
  const sp = Math.round((L.speed ?? 0.5) * 100);
  $("light-sp").value = sp;
  $("light-sp-out").textContent = sp + " %";
  $("light-br").value = Math.round(L.brightness * 100);
  $("light-br-out").textContent = Math.round(L.brightness * 100) + " %";
  $("light-flash").checked = L.flashOnFire;
  const xb = live && live.live.xbox;
  const XBOX_NAMES = { 0: "X", 1: "A", 2: "B", 3: "Y", 4: "LB", 5: "RB", 6: "LT", 7: "RT", 8: "Back", 9: "Start", 10: "LS", 11: "RS", 12: "Guide" };
  $("flash-btn").innerHTML = FLASH_ORDER.map((b) => `<option value="${b}">${esc(t(BTN_NAMES[b]))}${xb && XBOX_NAMES[b] ? " / " + XBOX_NAMES[b] : ""}</option>`).join("");
  $("flash-btn").value = L.flashButton ?? 7;
  $("flash-btn-box").hidden = !L.flashOnFire;
  $("trig-on").checked = S.adaptiveTriggers;
  $("trig-box").style.opacity = S.adaptiveTriggers ? 1 : 0.4;
  const opts = TRIGGER_FX.map(([v, n]) => `<option value="${v}">${t(n)}</option>`).join("");
  $("trig-l").innerHTML = opts;
  $("trig-r").innerHTML = opts;
  $("trig-l").value = P().triggerL;
  $("trig-r").value = P().triggerR;
  const s = Math.round(P().triggerStrength * 100);
  $("trig-s").value = s;
  $("trig-s-out").textContent = s + " %";
}
// Aperçu (approximatif) de la couleur envoyée à la barre.
function lightPreview(l, sys) {
  const L = S.lightbar;
  let c = null;
  if (L.mode === 1 || L.mode === 5) c = L.color;
  else if (L.mode === 3) c = (S.profiles[sys.activeProfile] || S.profiles[0]).lightColor;
  else if (L.mode === 4) c = PLAYER_COLORS[Math.max(1, L.player) - 1];
  else if (L.mode === 2) {
    const b = l.connected ? l.battery : 100;
    c = b >= 60 ? [0.1, 1, 0.25] : b >= 30 ? [1, 0.75, 0] : [1, 0.03, 0.03];
  }
  if (L.mode === 6) {
    // Même calcul que le moteur : un tour en 12 s (lent) à 1 s (rapide).
    const period = 12 - 11 * (L.speed ?? 0.5);
    const h = ((Date.now() / 1000) / period) % 1;
    c = [0, 2, 4].map((o) => Math.max(0, Math.min(1, Math.abs(((h * 6 + o) % 6) - 3) - 1)));
  }
  const el = $("light-prev");
  el.style.background = c ? rgb(c) : "#222";
  el.style.boxShadow = c ? `0 0 16px ${rgb(c)}` : "none";
}

// ---------- Remappage ----------
function renderRemap() {
  const opts = TARGETS.map(([v, n]) => `<option value="${v}">${t(n)}</option>`).join("");
  $("remap").innerHTML = SOURCES.map(
    (name, i) => `<div class="rrow"><span>${t(name)}</span><span class="arrow">→</span><select data-i="${i}">${opts}</select></div>`
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
$$("#corners button").forEach((b) => b.addEventListener("click", () => { S.overlay.corner = +b.dataset.corner; save(); renderOverlay(); }));
$$("#models button").forEach((b) => b.addEventListener("click", () => { S.overlay.model = +b.dataset.model; save(); renderOverlay(); }));
$$("#skins button").forEach((b) => b.addEventListener("click", () => { S.overlay.skin = +b.dataset.skin; save(); renderOverlay(); }));
$$("input[data-ov]").forEach((el) =>
  el.addEventListener("input", () => {
    S.overlay[el.dataset.ov] = el.value / 100;
    $$(`[data-out-ov="${el.dataset.ov}"]`).forEach((o) => (o.textContent = el.value + " %"));
    save();
  })
);
function renderOverlay() {
  const o = S.overlay;
  $$("#models button").forEach((b) => b.classList.toggle("sel", +b.dataset.model === o.model));
  $$("#corners button").forEach((b) => b.classList.toggle("sel", +b.dataset.corner === o.corner));
  $$("#skins button").forEach((b) => b.classList.toggle("sel", +b.dataset.skin === o.skin));
  $$("input[data-ov]").forEach((el) => {
    const v = Math.round(o[el.dataset.ov] * 100);
    el.value = v;
    $$(`[data-out-ov="${el.dataset.ov}"]`).forEach((x) => (x.textContent = v + " %"));
  });
  makeSwatches($("swatches"), COLORS, () => S.overlay.accent, (c) => (S.overlay.accent = c));
}

// ---------- Sessions : pause et notifications ----------
$("brk-on").addEventListener("change", (e) => { S.breakReminder = e.target.checked; save(); });
$("brk-min").addEventListener("change", (e) => { S.breakMinutes = +e.target.value; save(); });
$("toasts").addEventListener("change", (e) => { S.toasts = e.target.checked; save(); });
$("toast-test").addEventListener("click", () => invoke("test_toast"));

// ---------- Réglages ----------
$$("#langs button, #wiz-langs button").forEach((b) => b.addEventListener("click", () => setLang(b.dataset.lang)));
function setLang(l) {
  S.lang = l;
  LANG = l;
  save();
  translateDom();
  renderAll();
  if (live) renderLive(live);
  if (health) renderHealth(health);
}
$("autostart").addEventListener("change", async (e) => {
  try {
    const on = await invoke("set_autostart", { on: e.target.checked });
    e.target.checked = on;
    $("autostart-note").className = "note ok";
    $("autostart-note").textContent = on ? t("MeikiCtrl se lancera avec Windows.") : t("Démarrage automatique désactivé.");
  } catch (err) {
    e.target.checked = !e.target.checked;
    $("autostart-note").className = "note err";
    $("autostart-note").textContent = tErr(err);
  }
});
$("proc-prio").addEventListener("change", (e) => {
  S.processPriority = +e.target.value;
  invoke("set_process_priority", { level: S.processPriority });
});
$("admin-mode").addEventListener("change", async (e) => {
  const on = e.target.checked;
  const note = $("admin-note");
  note.className = "note";
  note.textContent = on ? t("Windows va demander l'autorisation administrateur…") : "";
  try {
    const msg = await invoke("set_admin_mode", { on });
    S.adminMode = on;
    note.className = "note ok";
    if (msg === "restart") note.textContent = t("MeikiCtrl redémarre en administrateur…");
    else note.textContent = msg ? tErr(msg) : on ? t("Mode administrateur actif.") : t("Mode administrateur coupé.");
  } catch (err) {
    e.target.checked = !on;
    S.adminMode = !on;
    note.className = "note err";
    note.textContent = tErr(err);
  }
});
$("wiz-again").addEventListener("click", () => openWizard());

function renderAll() {
  renderProfiles();
  renderSticks();
  renderLight();
  renderRemap();
  renderOverlay();
  $("hide").checked = S.hideController;
  $("proc-prio").value = S.processPriority;
  $("admin-mode").checked = S.adminMode;
  $("brk-on").checked = S.breakReminder;
  $("brk-min").value = String(S.breakMinutes);
  $("toasts").checked = S.toasts;
  $$("#langs button, #wiz-langs button").forEach((b) => b.classList.toggle("sel", b.dataset.lang === LANG));
}

// ---------- Dessin des sticks ----------
function drawStick(canvas, raw, out, dz, outer) {
  const ctx = canvas.getContext("2d");
  const w = canvas.width, c = w / 2, r = c - 10;
  ctx.clearRect(0, 0, w, w);
  ctx.strokeStyle = "#262626";
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.arc(c, c, r, 0, Math.PI * 2); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(c - r, c); ctx.lineTo(c + r, c); ctx.moveTo(c, c - r); ctx.lineTo(c, c + r); ctx.stroke();
  if (outer !== undefined) {
    ctx.setLineDash([4, 5]);
    ctx.strokeStyle = "rgba(255,255,255,0.22)";
    ctx.beginPath(); ctx.arc(c, c, r * outer, 0, Math.PI * 2); ctx.stroke();
    ctx.setLineDash([]);
  }
  if (dz !== undefined) {
    ctx.fillStyle = "rgba(255,66,66,0.16)";
    ctx.strokeStyle = "rgba(255,66,66,0.6)";
    ctx.beginPath(); ctx.arc(c, c, Math.max(r * dz, 1), 0, Math.PI * 2); ctx.fill(); ctx.stroke();
  }
  ctx.fillStyle = "#6a6a6a";
  ctx.beginPath(); ctx.arc(c + raw[0] * r, c - raw[1] * r, 5, 0, Math.PI * 2); ctx.fill();
  ctx.shadowColor = "#fff";
  ctx.shadowBlur = 12;
  ctx.fillStyle = "#fff";
  ctx.beginPath(); ctx.arc(c + out[0] * r, c - out[1] * r, 7, 0, Math.PI * 2); ctx.fill();
  ctx.shadowBlur = 0;
}

// ---------- Manette en direct (SVG) ----------
const svgBtns = $$("#padsvg .pb[data-bit]").map((el) => [el, 1 << +el.dataset.bit, $("gl-" + el.dataset.bit)]);
let lastGlyphXbox = null;
function renderPad(l) {
  svgBtns.forEach(([el, bit, gl]) => {
    const on = (l.buttons & bit) !== 0;
    el.classList.toggle("on", on);
    if (gl) gl.classList.toggle("on", on);
  });
  if (lastGlyphXbox !== l.xbox) {
    lastGlyphXbox = l.xbox;
    const g = l.xbox ? { 3: "Y", 2: "B", 1: "A", 0: "X" } : { 3: "△", 2: "○", 1: "✕", 0: "□" };
    Object.entries(g).forEach(([k, v]) => ($("gl-" + k).textContent = v));
  }
  const th = (id, cx, s, pressed) => {
    const el = $(id);
    el.setAttribute("cx", cx + s[0] * 14);
    el.setAttribute("cy", 178 - s[1] * 14);
    el.classList.toggle("on", pressed);
  };
  th("thumb-l", 146, l.outL, (l.buttons & (1 << 10)) !== 0);
  th("thumb-r", 254, l.outR, (l.buttons & (1 << 11)) !== 0);
  $("sv-l2").setAttribute("width", 66 * l.l2);
  $("sv-r2").setAttribute("width", 66 * l.r2);
}

// ---------- Mise à jour en direct ----------
function padName(l) {
  return !l.connected ? t("Aucune manette") : l.xbox ? t("Manette Xbox") : l.edge ? "DualSense Edge" : "DualSense";
}
function renderLive(st) {
  const l = st.live, sys = st.sys;
  const name = padName(l);
  const conn = !l.connected ? "—" : l.xbox ? "XInput" : l.bluetooth ? "Bluetooth" : "USB";
  $("pad-name").textContent = name;
  $("pad-badge").textContent = l.connected ? t("Connectée") : t("Déconnectée");
  $("pad-badge").classList.toggle("on", l.connected);
  $("pad-conn").textContent = conn;
  const batt = l.connected && !l.xbox ? `${l.battery} %${l.charging ? " · " + t("en charge") : ""}` : "—";
  $("pad-batt").textContent = batt;
  $("pad-hz").textContent = l.connected && !l.xbox && l.hz > 0 ? `${Math.round(l.hz)} Hz` : "—";
  $("batt-bar").style.width = (l.connected && !l.xbox ? l.battery : 0) + "%";
  $("hero-sub").textContent = l.connected
    ? [name, conn, l.xbox ? null : `${l.battery} %`].filter(Boolean).join(" · ")
    : t("Branche ta manette pour commencer.");
  $("model-hint").textContent = sys.xboxModel
    ? t("Automatique : l'overlay affiche la manette que tu as branchée.")
    : t("Le modèle Xbox n'est pas installé : l'overlay affiche la DualSense.");
  $("foot-pad").textContent = l.connected ? name : t("Aucune manette");
  $("foot-dot").classList.toggle("on", l.connected);

  const active = S.profiles[sys.activeProfile] || S.profiles[0];
  $("foot-profile").textContent = t("Profil") + colon() + active.name;
  $("dash-profile").textContent = active.name;
  $("dash-game").textContent = sys.activeByGame ? t("Activé par {g}", { g: sys.lastGameExe }) : t("Profil hors jeu");
  $("pr-detect").textContent = t("Jeu détecté") + colon() + (sys.lastGameExe || "—");

  renderPad(l);
  $$(".t-overlay").forEach((el) => (el.checked = st.overlay));
  $$(".t-emu").forEach((el) => (el.checked = st.emulation));

  // Émulation
  let text, cls;
  if (l.xbox) { text = t("Ta manette est déjà une manette Xbox : pas besoin d'émulation."); cls = "ok"; }
  else if (!st.emulation) { text = t("Émulation coupée."); cls = ""; }
  else if (sys.emulationBlockedBy) { text = t("En pause : « {g} » bloque les manettes virtuelles. Ta DualSense est utilisée en direct.", { g: sys.emulationBlockedBy }); cls = ""; }
  else if (l.emulationError) { text = tErr(l.emulationError); cls = "err"; }
  else if (l.emulationActive) { text = t("Active : les jeux voient une manette Xbox 360."); cls = "ok"; }
  else if (!l.connected) { text = t("En attente de ta manette…"); cls = ""; }
  else { text = t("Branchement de la manette virtuelle…"); cls = ""; }
  $("e-state").textContent = text;
  $("e-state").className = "state " + cls;
  $("tile-emu-sub").textContent = st.emulation ? text : t("Les jeux voient une manette Xbox 360");

  // HidHide
  let h, hc = "";
  if (!sys.hidhideInstalled) { h = t("HidHide n'est pas installé. Télécharge-le sur github.com/nefarius/HidHide/releases, puis relance MeikiCtrl."); hc = "err"; }
  else if (sys.hidhideBusy) h = t("Autorisation administrateur demandée…");
  else if (sys.hidhideError) { h = tErr(sys.hidhideError); hc = "err"; }
  else if (sys.hidhideHidden) { h = t("DualSense masquée : les jeux ne voient que la manette Xbox."); hc = "ok"; }
  else if (S.hideController && sys.emulationBlockedBy) h = t("DualSense visible : « {g} » est lancé.", { g: sys.emulationBlockedBy });
  else if (S.hideController && !st.emulation) h = t("S'activera avec l'émulation.");
  else if (S.hideController && !l.connected) h = t("En attente de ta manette…");
  else h = t("DualSense visible par les jeux.");
  $("h-state").textContent = h;
  $("h-state").className = "state " + hc;
  $("hide").disabled = !sys.hidhideInstalled;

  // Lumière : pause pendant un jeu qui pilote la manette lui-même
  $("light-pause").textContent = l.xbox
    ? t("La lumière et les gâchettes adaptatives sont réservées à la DualSense.")
    : sys.emulationBlockedBy
      ? t("En pause : « {g} » gère lui-même la lumière et les gâchettes.", { g: sys.emulationBlockedBy })
      : "";
  lightPreview(l, sys);

  if (document.activeElement !== $("autostart")) $("autostart").checked = sys.autostart;
  const gp = sys.gpuPriority || "—";
  $("gpu-prio").textContent = t(gp);
  $("gpu-prio").className = "perf-v " + (gp === "temps réel" ? "" : gp === "haute" ? "mid" : "bad");
  $("elevated").textContent = sys.elevated ? t("Oui") : t("Non");
  $("elevated").className = "perf-v " + (sys.elevated ? "" : "mid");

  if ($("page-sticks").classList.contains("active")) {
    const p = P();
    drawStick($("view-l"), l.rawL, l.outL, p.leftDeadzone, p.leftOuter);
    drawStick($("view-r"), l.rawR, l.outR, p.rightDeadzone, p.rightOuter);
  }
  if (circ.running) circSample(l);
  renderWizardLive(l);
}

// ---------- Santé ----------
function renderHealth(hd) {
  const l = live ? live.live : { connected: false };
  // Fréquence
  if (l.connected && !l.xbox && hd.hz > 0) {
    $("h-hz").textContent = `${Math.round(hd.hz)} Hz`;
    const steady = hd.worstMs < Math.max(12, hd.avgMs * 3);
    $("h-hz-sub").textContent = t("{a} ms en moyenne, pire {w} ms.", { a: hd.avgMs.toFixed(1), w: hd.worstMs.toFixed(1) }) + " " +
      (steady ? t("Signal régulier.") : t("Quelques à-coups : rapproche la manette ou passe en USB."));
  } else {
    $("h-hz").textContent = "—";
    $("h-hz-sub").textContent = l.xbox ? t("Mesure réservée à la DualSense.") : t("Branche ta DualSense pour mesurer.");
  }
  // Dérive
  const d = hd.drift[hd.drift.length - 1];
  const prof = live ? S.profiles[live.sys.activeProfile] || S.profiles[0] : S.profiles[0];
  if (d && (d.nLeft || d.nRight)) {
    const pl = Math.round(d.left * 100), pr = Math.round(d.right * 100);
    $("h-drift").textContent = `${t("G")} ${pl} % · ${t("D")} ${pr} %`;
    const over = d.left > prof.leftDeadzone * 0.9 || d.right > prof.rightDeadzone * 0.9;
    $("h-drift-sub").textContent = over
      ? t("Plus fort que ta zone morte : corrige la dérive.")
      : t("Sous ta zone morte : aucun mouvement fantôme.");
  } else {
    $("h-drift").textContent = "—";
    $("h-drift-sub").textContent = t("Laisse ta manette posée quelques secondes pour mesurer.");
  }
  // Rebonds
  const totalB = hd.bounces.reduce((a, b) => a + b, 0);
  const worn = hd.bounces.map((b, i) => [i, b]).filter(([i, b]) => b >= 5 && b > hd.presses[i] * 0.003);
  $("h-bounce").textContent = worn.length ? t("À surveiller") : t("Nickel");
  $("h-bounce-sub").textContent = worn.length
    ? t("Rebonds sur : {b}.", { b: worn.map(([i]) => t(BTN_NAMES[i])).join(", ") })
    : totalB
      ? t("{n} rebond(s) isolé(s), rien d'inquiétant.", { n: totalB })
      : t("Aucun rebond détecté.");
  drawDriftChart(hd.drift);
  // Appuis par bouton
  const rows = hd.presses.map((n, i) => [i, n]).filter(([, n]) => n > 0).sort((a, b) => b[1] - a[1]);
  const max = rows.length ? rows[0][1] : 1;
  $("press-bars").innerHTML = rows.length
    ? rows.map(([i, n]) => {
        const b = hd.bounces[i];
        return `<div class="bar-row"><span>${esc(t(BTN_NAMES[i]))}</span><div class="track"><div class="fill" style="width:${(n / max) * 100}%"></div></div>
          <span class="n">${nf().format(n)}${b ? ` <span class="warn">· ${b}</span>` : ""}</span></div>`;
      }).join("")
    : `<div class="muted">${t("Pas encore d'appui enregistré.")}</div>`;
  renderSessions(hd);
}
function drawDriftChart(days) {
  const svg = $("drift-chart");
  if (!days.length) { svg.innerHTML = ""; return; }
  const vmax = Math.max(0.12, ...days.map((d) => Math.max(d.left, d.right))) * 1.25;
  const W = 400, H = 140, n = days.length;
  const x = (i) => (n === 1 ? W / 2 : (i / (n - 1)) * (W - 20) + 10);
  const y = (v) => H - 10 - (v / vmax) * (H - 20);
  const path = (key) => days.map((d, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(d[key]).toFixed(1)}`).join(" ");
  const grid = [0.25, 0.5, 0.75].map((f) => `<line class="gl" x1="0" x2="${W}" y1="${y(vmax * f)}" y2="${y(vmax * f)}"/>`).join("");
  svg.innerHTML = grid + `<path class="l" d="${path("left")}"/><path class="r" d="${path("right")}"/>`;
}
$("h-reset").addEventListener("click", (e) => {
  const b = e.target;
  if (!b.dataset.armed) {
    b.dataset.armed = "1";
    b.textContent = t("Clique encore pour confirmer");
    setTimeout(() => { delete b.dataset.armed; b.textContent = t("Remettre les compteurs à zéro"); }, 3000);
    return;
  }
  delete b.dataset.armed;
  b.textContent = t("Remettre les compteurs à zéro");
  invoke("reset_health").then(pollHealth);
});

// Test de circularité : rayon max par tranche d'angle.
const circ = { running: false, l: [], r: [], timer: null };
$("circ-btn").addEventListener("click", () => (circ.running ? circStop() : circStart()));
function circStart() {
  circ.running = true;
  circ.l = new Array(72).fill(0);
  circ.r = new Array(72).fill(0);
  $("circ-btn").textContent = t("Terminer");
  $("circ-l-v").textContent = $("circ-r-v").textContent = t("Tourne…");
  clearTimeout(circ.timer);
  circ.timer = setTimeout(circStop, 20000);
}
function circSample(l) {
  for (const [arr, s] of [[circ.l, l.rawL], [circ.r, l.rawR]]) {
    const m = Math.hypot(s[0], s[1]);
    if (m < 0.5) continue;
    const a = Math.floor(((Math.atan2(s[1], s[0]) + Math.PI) / (2 * Math.PI)) * 72) % 72;
    arr[a] = Math.max(arr[a], Math.min(m, 1.4));
  }
  drawCirc($("circ-l"), circ.l);
  drawCirc($("circ-r"), circ.r);
}
function drawCirc(canvas, arr) {
  const ctx = canvas.getContext("2d");
  const w = canvas.width, c = w / 2, r = c - 14;
  ctx.clearRect(0, 0, w, w);
  ctx.strokeStyle = "rgba(255,255,255,0.25)";
  ctx.setLineDash([4, 4]);
  ctx.beginPath(); ctx.arc(c, c, r, 0, Math.PI * 2); ctx.stroke();
  ctx.setLineDash([]);
  ctx.strokeStyle = "#fff";
  ctx.lineWidth = 2;
  ctx.beginPath();
  let first = true;
  arr.forEach((m, i) => {
    if (!m) return;
    const a = ((i + 0.5) / 72) * 2 * Math.PI - Math.PI;
    const px = c + Math.cos(a) * m * r, py = c - Math.sin(a) * m * r;
    if (first) { ctx.moveTo(px, py); first = false; } else ctx.lineTo(px, py);
  });
  if (!first) ctx.closePath();
  ctx.stroke();
}
function circResult(arr) {
  const got = arr.filter((m) => m > 0);
  if (got.length < 54) return t("Pas assez de tours");
  const err = got.reduce((a, m) => a + Math.abs(m - 1), 0) / got.length;
  const pct = Math.round(err * 100);
  return pct <= 8 ? t("{p} % d'écart : excellent", { p: pct }) : pct <= 15 ? t("{p} % d'écart : correct", { p: pct }) : t("{p} % d'écart : stick fatigué", { p: pct });
}
function circStop() {
  circ.running = false;
  clearTimeout(circ.timer);
  $("circ-btn").textContent = t("Lancer");
  $("circ-l-v").textContent = circResult(circ.l);
  $("circ-r-v").textContent = circResult(circ.r);
}

// ---------- Sessions ----------
function sessionLine(s) {
  const top = (s.top || []).map(([b]) => t(BTN_NAMES[b])).join(", ");
  return t("{p} appuis", { p: nf().format(s.presses) }) + (top ? ` · ${top}` : "");
}
function renderSessions(hd) {
  const cur = hd.current;
  if (cur && cur.minutes >= 0.5) {
    $("s-cur").textContent = fmtMin(cur.minutes);
    $("s-cur-sub").textContent = [cur.game, t("{p} appuis", { p: nf().format(cur.presses) })].filter(Boolean).join(" · ");
  } else {
    $("s-cur").textContent = "—";
    $("s-cur-sub").textContent = t("Touche ta manette pour démarrer.");
  }
  $("s-total").textContent = fmtMin(hd.playMinutes);
  $("s-count").textContent = t("{n} session(s) enregistrée(s)", { n: hd.sessions.length });
  const best = hd.sessions.reduce((a, s) => (!a || s.minutes > a.minutes ? s : a), null);
  $("s-best").textContent = best ? fmtMin(best.minutes) : "—";
  $("s-best-sub").textContent = best ? [best.game, fmtDate(best.start)].filter(Boolean).join(" · ") : "";
  $("s-list").innerHTML = hd.sessions.length
    ? hd.sessions.map((s) => `<div class="srow">
        <span class="m">${esc(fmtDate(s.start))}</span>
        <span class="g">${esc(s.game || t("Jeu inconnu"))}</span>
        <span class="d">${fmtMin(s.minutes)}</span>
        <span class="m">${esc(sessionLine(s))}</span></div>`).join("")
    : `<div class="muted">${t("Ta première session apparaîtra ici après avoir joué.")}</div>`;

  // Accueil
  const box = $("dash-session");
  if (cur && cur.minutes >= 0.5) {
    box.innerHTML = `<div class="k">${t("En cours")}</div><div class="bigv">${fmtMin(cur.minutes)}</div>
      <div class="muted">${esc([cur.game, t("{p} appuis", { p: nf().format(cur.presses) })].filter(Boolean).join(" · "))}</div>`;
  } else if (hd.sessions.length) {
    const s = hd.sessions[0];
    box.innerHTML = `<div class="k">${t("Dernière session")}</div><div class="bigv">${fmtMin(s.minutes)}</div>
      <div class="muted">${esc([s.game, fmtDate(s.start)].filter(Boolean).join(" · "))}</div>`;
  } else {
    box.innerHTML = `<div class="muted">${t("Aucune session pour l'instant. Lance un jeu !")}</div>`;
  }
}

async function pollHealth() {
  try {
    health = await invoke("get_health");
    renderHealth(health);
  } catch (e) {
    console.error(e);
  }
}

// ---------- Assistant de premier lancement ----------
let wizStep = 0;
const WIZ_STEPS = 5;
$("wiz-dots").innerHTML = Array.from({ length: WIZ_STEPS }, () => "<span></span>").join("");
function openWizard() {
  wizStep = 0;
  $("wizard").hidden = false;
  $("wiz-brk").checked = S.breakReminder;
  showWizStep();
}
function showWizStep() {
  $$(".wiz-step").forEach((s) => s.classList.toggle("active", +s.dataset.step === wizStep));
  $$("#wiz-dots span").forEach((d, i) => d.classList.toggle("done", i <= wizStep));
  $("wiz-prev").style.visibility = wizStep ? "visible" : "hidden";
  $("wiz-next").textContent = wizStep === WIZ_STEPS - 1 ? t("Terminer") : t("Suivant");
  $$("#wiz-light button").forEach((b) => b.classList.toggle("sel", +b.dataset.mode === S.lightbar.mode));
  if (wizStep === 2) checkDrivers();
}
async function checkDrivers() {
  const set = (id, ok, optional) => {
    $(id).textContent = ok ? t("Installé") : optional ? t("Absent") : t("Manquant");
    $(id).className = "wiz-ok " + (ok ? "ok" : optional ? "" : "err");
  };
  try {
    const st = await invoke("setup_status");
    set("wiz-vigem", st.vigem, false);
    set("wiz-hidhide", st.hidhide, true);
  } catch (e) {
    console.error(e);
  }
}
function renderWizardLive(l) {
  if ($("wizard").hidden) return;
  const el = $("wiz-pad");
  el.textContent = l.connected ? `${padName(l)} ✓` : t("En attente de ta manette…");
  el.classList.toggle("ok", l.connected);
}
$("wiz-recheck").addEventListener("click", checkDrivers);
$$("#wiz-light button").forEach((b) =>
  b.addEventListener("click", () => {
    S.lightbar.mode = +b.dataset.mode;
    if (S.lightbar.mode === 1) S.lightbar.color = [1, 1, 1];
    save(); showWizStep(); renderLight();
  })
);
$("wiz-brk").addEventListener("change", (e) => { S.breakReminder = e.target.checked; $("brk-on").checked = e.target.checked; save(); });
$("wiz-prev").addEventListener("click", () => { wizStep = Math.max(0, wizStep - 1); showWizStep(); });
$("wiz-next").addEventListener("click", () => {
  if (wizStep < WIZ_STEPS - 1) { wizStep++; showWizStep(); return; }
  S.firstRunDone = true;
  save();
  $("wizard").hidden = true;
});

// ---------- Boucles ----------
async function tick() {
  try {
    live = await invoke("get_live");
    renderLive(live);
  } catch (e) {
    console.error(e);
  }
  setTimeout(tick, 33); // ~30 fois par seconde, seulement quand la fenêtre est ouverte
}
function healthLoop() {
  const visible = ["page-dashboard", "page-health", "page-sessions"].some((id) => $(id).classList.contains("active"));
  if (visible) pollHealth();
  setTimeout(healthLoop, 1000);
}

(async () => {
  S = await invoke("get_settings");
  if (S.lang !== "fr" && S.lang !== "en") {
    S.lang = (navigator.language || "fr").toLowerCase().startsWith("fr") ? "fr" : "en";
    save();
  }
  LANG = S.lang;
  translateDom();
  renderAll();
  tick();
  healthLoop();
  if (!S.firstRunDone) openWizard();
})();
