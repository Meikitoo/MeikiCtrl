// MeikiCtrl — traduction anglaise. Le texte français sert de clé :
// on écrit l'interface en français, et `t()` / `translateDom()` donnent l'anglais si besoin.
let LANG = "fr";

const EN = {
  // Navigation
  "Accueil": "Home",
  "Profils": "Profiles",
  "Ta manette": "Your controller",
  "Sticks & gâchettes": "Sticks & triggers",
  "Lumière & effets": "Light & effects",
  "Santé de la manette": "Controller health",
  "Sessions": "Sessions",
  "En jeu": "In game",
  "Émulation Xbox": "Xbox emulation",
  "Overlay manette": "Controller overlay",
  "Plus": "More",
  "Remappage": "Remapping",
  "Réglages": "Settings",
  "Soutenir le projet": "Support the project",

  // Accueil
  "Prêt à jouer ?": "Ready to play?",
  "Branche ta manette pour commencer.": "Plug in your controller to get started.",
  "La manette 3D par-dessus ton jeu": "The 3D controller on top of your game",
  "Les jeux voient une manette Xbox 360": "Games see an Xbox 360 controller",
  "Profil actif": "Active profile",
  "Aucune manette": "No controller",
  "Déconnectée": "Disconnected",
  "Connectée": "Connected",
  "Manette en direct": "Live controller",
  "Connexion": "Connection",
  "Batterie": "Battery",
  "Fréquence": "Polling rate",
  "Ta session": "Your session",
  "Aucune session en cours.": "No session in progress.",
  "Voir les sessions": "See sessions",
  "Recentrer l'overlay": "Recenter overlay",
  "Tu kiffes MeikiCtrl ?": "Enjoying MeikiCtrl?",
  "L'appli est gratuite et le restera. Un café aide à ajouter de nouvelles fonctions.": "The app is free and will stay free. A coffee helps add new features.",
  "♥ Offrir un café sur Ko-fi": "♥ Buy me a coffee on Ko-fi",
  "Manette Xbox": "Xbox controller",
  "en charge": "charging",
  "Profil": "Profile",
  "Activé par {g}": "Activated by {g}",
  "Profil hors jeu": "Default profile",
  "En cours": "In progress",
  "Dernière session": "Last session",
  "Aucune session pour l'instant. Lance un jeu !": "No sessions yet. Go play something!",

  // Profils
  "Un profil par jeu : tes réglages de sticks, de gâchettes, de lumière et de boutons. MeikiCtrl l'active tout seul quand le jeu passe au premier plan.":
    "One profile per game: your stick, trigger, light and button settings. MeikiCtrl switches to it on its own when the game comes to the front.",
  "Bascule automatique selon le jeu": "Switch automatically with the game",
  "Profil hors jeu reconnu": "Profile when no known game is running",
  "Tes profils": "Your profiles",
  "Nouveau": "New",
  "Dupliquer": "Duplicate",
  "Profil sélectionné": "Selected profile",
  "Nom": "Name",
  "Exécutable du jeu": "Game executable",
  "ex. cod.exe (vide = pas de bascule auto)": "e.g. cod.exe (empty = no auto switch)",
  "Jeu détecté": "Detected game",
  "Astuce : lance ton jeu, reviens ici et clique « Jeu détecté ». Pour COD : cod.exe.": "Tip: launch your game, come back here and click “Detected game”. For COD: cod.exe.",
  "Ce jeu bloque l'émulation": "This game blocks emulation",
  "Pour les jeux dont l'anti-triche refuse les manettes virtuelles (COD…). Tant que le jeu tourne, l'émulation est en pause, ta DualSense redevient visible et le jeu gère lui-même la lumière et les gâchettes.":
    "For games whose anti-cheat rejects virtual controllers (COD…). While the game runs, emulation pauses, your DualSense becomes visible again and the game handles the light and triggers itself.",
  "Supprimer": "Delete",
  "aucun jeu lié": "no linked game",
  "Sans nom": "Unnamed",
  "Nouveau profil": "New profile",
  "(copie)": "(copy)",

  // Sticks
  "Ton perso bouge tout seul ? C'est de la dérive. Lâche le stick et clique « Corriger la dérive ».": "Your character moves on its own? That's drift. Let go of the stick and click “Fix drift”.",
  "Stick gauche": "Left stick",
  "Stick droit": "Right stick",
  "Brut": "Raw",
  "Ce que voit le jeu": "What the game sees",
  "Zone morte": "Deadzone",
  "Corriger la dérive": "Fix drift",
  "Courbe de visée": "Aim curve",
  "Courbe": "Curve",
  "Avancé": "Advanced",
  "Anti-zone morte": "Anti-deadzone",
  "Portée max": "Max range",
  "Gâchettes L2 / R2": "L2 / R2 triggers",
  "Gâchette rapide": "Hair trigger",
  "Le moindre appui compte comme un appui complet : tu tires plus tôt.": "The slightest press counts as a full press: you shoot sooner.",
  "Anti-zone morte : compense la zone morte propre au jeu (COD en a une). Monte-la doucement jusqu'à ce que le moindre mouvement du stick fasse bouger la visée.":
    "Anti-deadzone: cancels the game's own deadzone (COD has one). Raise it slowly until the smallest stick movement moves your aim.",
  "Perso": "Custom",
  "Ne touche pas au stick… mesure en cours (2 s)": "Don't touch the stick… measuring (2 s)",
  "Zone morte réglée à {n} %. La dérive est corrigée.": "Deadzone set to {n} %. Drift fixed.",
  "Linéaire": "Linear",
  "Visée FPS (précis au centre)": "FPS aim (precise in the center)",
  "Précision max": "Max precision",
  "Réactif (rapide au centre)": "Snappy (fast in the center)",

  // Lumière
  "La barre lumineuse, les LED joueur et les gâchettes adaptatives de ta DualSense.": "Your DualSense's light bar, player LEDs and adaptive triggers.",
  "Barre lumineuse": "Light bar",
  "Couleur fixe": "Solid color",
  "Respiration": "Breathing",
  "Couleur du profil": "Profile color",
  "Couleur du joueur": "Player color",
  "Ne pas toucher": "Leave it alone",
  "Luminosité": "Brightness",
  "Flash au tir": "Flash on fire",
  "Un éclair blanc à chaque appui sur le bouton de ton choix.": "A white flash every time you press the button of your choice.",
  "Bouton qui déclenche le flash": "Button that triggers the flash",
  "Joueur": "Player",
  "Les petites LED sous le pavé tactile, comme sur PS5. En mode « Couleur du joueur », la barre prend aussi sa couleur.":
    "The small LEDs under the touchpad, like on PS5. In “Player color” mode, the light bar takes its color too.",
  "Éteintes": "Off",
  "Joueur 1": "Player 1",
  "Joueur 2": "Player 2",
  "Joueur 3": "Player 3",
  "Joueur 4": "Player 4",
  "Utilisée en mode « Couleur du profil » : une couleur par jeu.": "Used in “Profile color” mode: one color per game.",
  "Gâchettes adaptatives": "Adaptive triggers",
  "La résistance de L2 et R2, réglée par profil. Coupe l'interrupteur pour toutes les désactiver d'un coup.": "L2 and R2 resistance, set per profile. Flip the switch to turn them all off at once.",
  "Force": "Strength",
  "Couleur perso": "Custom color",
  "Aucun effet": "No effect",
  "Résistance": "Resistance",
  "Clic d'arme": "Weapon click",
  "MeikiCtrl ne touche pas à la lumière : la manette ou le jeu gère.": "MeikiCtrl leaves the light alone: the controller or the game handles it.",
  "Toujours la même couleur.": "Always the same color.",
  "Vert, jaune puis rouge selon la batterie. Elle pulse en charge et clignote quand c'est presque vide.": "Green, yellow, then red with the battery. It pulses while charging and blinks when almost empty.",
  "Chaque profil a sa couleur : la barre change toute seule avec le jeu.": "Each profile has its own color: the light bar changes with the game.",
  "La couleur de ton numéro de joueur, comme sur PS5.": "The color of your player number, like on PS5.",
  "Ta couleur, qui respire doucement.": "Your color, gently breathing.",
  "Toutes les couleurs qui défilent en boucle. Règle la vitesse juste en dessous.": "Every color, cycling in a loop. Set the speed just below.",
  "Vitesse": "Speed",
  "La lumière et les gâchettes adaptatives sont réservées à la DualSense.": "Light and adaptive triggers are DualSense-only.",
  "En pause : « {g} » gère lui-même la lumière et les gâchettes.": "Paused: “{g}” handles the light and triggers itself.",

  // Couleurs et boutons
  "Blanc": "White",
  "Rouge": "Red",
  "Violet": "Purple",
  "Vert": "Green",
  "Or": "Gold",
  "Orange": "Orange",
  "Bleu": "Blue",
  "Rose": "Pink",
  "Carré": "Square",
  "Croix": "Cross",
  "Rond": "Circle",
  "Triangle": "Triangle",
  "Pavé tactile": "Touchpad",
  "Micro": "Mute",
  "Haut": "Up",
  "Bas": "Down",
  "Gauche": "Left",
  "Droite": "Right",
  "Croix ↑": "D-pad ↑",
  "Croix ↓": "D-pad ↓",
  "Croix ←": "D-pad ←",
  "Croix →": "D-pad →",
  "— Rien —": "— Nothing —",
  "Clic stick gauche (LS)": "Left stick click (LS)",
  "Clic stick droit (RS)": "Right stick click (RS)",

  // Santé
  "MeikiCtrl surveille ta manette pendant que tu joues : dérive, boutons qui rebondissent, réactivité.": "MeikiCtrl keeps an eye on your controller while you play: drift, bouncing buttons, responsiveness.",
  "Dérive au repos": "Resting drift",
  "Boutons": "Buttons",
  "Dérive jour après jour": "Drift day by day",
  "Le décalage de tes sticks quand tu ne les touches pas. S'il grimpe avec le temps, la dérive s'aggrave : augmente la zone morte ou pense à nettoyer ou réparer le stick.":
    "How far your sticks sit off-center when you're not touching them. If it climbs over time, drift is getting worse: raise the deadzone, or consider cleaning or repairing the stick.",
  "Test de circularité": "Circularity test",
  "Clique « Lancer », puis fais 3 tours lents avec chaque stick, poussé à fond. Un stick sain dessine un cercle presque parfait.":
    "Click “Start”, then slowly rotate each stick 3 times, pushed all the way. A healthy stick draws an almost perfect circle.",
  "Lancer": "Start",
  "Terminer": "Finish",
  "Tourne…": "Rotate…",
  "Pas assez de tours": "Not enough rotations",
  "{p} % d'écart : excellent": "{p} % off: excellent",
  "{p} % d'écart : correct": "{p} % off: fine",
  "{p} % d'écart : stick fatigué": "{p} % off: worn stick",
  "Appuis par bouton": "Presses per button",
  "Rebond : un seul appui détecté deux fois en moins de 15 ms. Quelques-uns, ce n'est rien ; s'ils se multiplient sur un bouton, il commence à s'user.":
    "Bounce: a single press detected twice within 15 ms. A few are nothing; if they pile up on one button, it's starting to wear out.",
  "Remettre les compteurs à zéro": "Reset counters",
  "Clique encore pour confirmer": "Click again to confirm",
  "{a} ms en moyenne, pire {w} ms.": "{a} ms on average, worst {w} ms.",
  "Signal régulier.": "Steady signal.",
  "Quelques à-coups : rapproche la manette ou passe en USB.": "A few hiccups: move the controller closer or switch to USB.",
  "Mesure réservée à la DualSense.": "Only measured on DualSense.",
  "Branche ta DualSense pour mesurer.": "Plug in your DualSense to measure.",
  "G": "L",
  "D": "R",
  "Plus fort que ta zone morte : corrige la dérive.": "Stronger than your deadzone: fix the drift.",
  "Sous ta zone morte : aucun mouvement fantôme.": "Below your deadzone: no ghost movement.",
  "Laisse ta manette posée quelques secondes pour mesurer.": "Leave your controller still for a few seconds to measure.",
  "À surveiller": "Keep an eye",
  "Nickel": "All good",
  "Rebonds sur : {b}.": "Bounces on: {b}.",
  "{n} rebond(s) isolé(s), rien d'inquiétant.": "{n} isolated bounce(s), nothing to worry about.",
  "Aucun rebond détecté.": "No bounce detected.",
  "Pas encore d'appui enregistré.": "No presses recorded yet.",

  // Sessions
  "Une session commence quand tu touches ta manette et se termine après 5 minutes sans rien toucher. À la fin, tu reçois ton bilan.":
    "A session starts when you touch your controller and ends after 5 minutes without input. At the end, you get your recap.",
  "Temps de jeu total": "Total play time",
  "Plus longue session": "Longest session",
  "Dernières sessions": "Recent sessions",
  "Rappel de pause": "Break reminder",
  "Me rappeler de faire une pause": "Remind me to take a break",
  "Une petite bulle en haut de l'écran, par-dessus le jeu.": "A small bubble at the top of the screen, on top of the game.",
  "Toutes les": "Every",
  "1 h": "1 h",
  "1 h 30": "1 h 30",
  "2 h": "2 h",
  "3 h": "3 h",
  "Notifications à l'écran": "On-screen notifications",
  "Pause, bilan de session et batterie faible.": "Breaks, session recap and low battery.",
  "Tester une notification": "Test a notification",
  "{p} appuis": "{p} presses",
  "Touche ta manette pour démarrer.": "Touch your controller to start.",
  "{n} session(s) enregistrée(s)": "{n} session(s) recorded",
  "Jeu inconnu": "Unknown game",
  "Ta première session apparaîtra ici après avoir joué.": "Your first session will show up here after you play.",

  // Émulation
  "Ta DualSense devient une manette Xbox 360 pour tous les jeux PC, avec les réglages de ton profil.": "Your DualSense becomes an Xbox 360 controller for every PC game, with your profile settings.",
  "Manette Xbox 360 virtuelle": "Virtual Xbox 360 controller",
  "Via le pilote ViGEmBus": "Through the ViGEmBus driver",
  "Masquer la DualSense aux jeux": "Hide the DualSense from games",
  "Pendant l'émulation, les jeux ne voient plus que la manette Xbox : fini les doubles entrées.": "During emulation, games only see the Xbox controller: no more double inputs.",
  "Utilise HidHide (Nefarius). Sans le mode administrateur, Windows demande une autorisation à chaque masquage. En quittant MeikiCtrl, la DualSense redevient visible.":
    "Uses HidHide (Nefarius). Without admin mode, Windows asks for permission each time. When you quit MeikiCtrl, the DualSense becomes visible again.",
  "À savoir": "Good to know",
  "Steam : si Steam Input est activé pour ta DualSense, désactive-le pour ce jeu, sinon il y aura un doublon.": "Steam: if Steam Input is on for your DualSense, turn it off for that game, or you'll get double inputs.",
  "MeikiCtrl n'a ni macros ni turbo. Sur COD, Ricochet peut sanctionner ce genre d'outils en partie classée.": "MeikiCtrl has no macros or turbo. In COD, Ricochet may penalize that kind of tool in ranked.",
  "Ta manette est déjà une manette Xbox : pas besoin d'émulation.": "Your controller is already an Xbox controller: no emulation needed.",
  "Émulation coupée.": "Emulation off.",
  "En pause : « {g} » bloque les manettes virtuelles. Ta DualSense est utilisée en direct.": "Paused: “{g}” blocks virtual controllers. Your DualSense is used directly.",
  "Active : les jeux voient une manette Xbox 360.": "Active: games see an Xbox 360 controller.",
  "En attente de ta manette…": "Waiting for your controller…",
  "Branchement de la manette virtuelle…": "Plugging in the virtual controller…",
  "HidHide n'est pas installé. Télécharge-le sur github.com/nefarius/HidHide/releases, puis relance MeikiCtrl.": "HidHide isn't installed. Get it from github.com/nefarius/HidHide/releases, then restart MeikiCtrl.",
  "Autorisation administrateur demandée…": "Admin permission requested…",
  "DualSense masquée : les jeux ne voient que la manette Xbox.": "DualSense hidden: games only see the Xbox controller.",
  "DualSense visible : « {g} » est lancé.": "DualSense visible: “{g}” is running.",
  "S'activera avec l'émulation.": "Will kick in with emulation.",
  "DualSense visible par les jeux.": "DualSense visible to games.",

  // Overlay
  "Une manette 3D animée par-dessus ton jeu. Elle s'allume quand tu appuies, et ne consomme presque rien.": "An animated 3D controller on top of your game. It lights up when you press, and uses almost nothing.",
  "Afficher l'overlay": "Show overlay",
  "Transparent, au premier plan, les clics passent à travers. Raccourci : F8.": "Transparent, always on top, clicks go through. Shortcut: F8.",
  "Habillage": "Skin",
  "Éclairs": "Lightning",
  "Noir mat": "Matte black",
  "Couleur": "Color",
  "Couleur des éclairs et des boutons": "Lightning and button color",
  "Le bouton PS et le micro restent rouges.": "The PS and mute buttons stay red.",
  "Modèle": "Model",
  "Automatique": "Automatic",
  "Position": "Position",
  "Haut gauche": "Top left",
  "Haut droite": "Top right",
  "Bas gauche": "Bottom left",
  "Bas droite": "Bottom right",
  "Taille": "Size",
  "Opacité": "Opacity",
  "Raccourcis : F8 pour afficher ou cacher, F9 pour recentrer.": "Shortcuts: F8 to show or hide, F9 to recenter.",
  "Automatique : l'overlay affiche la manette que tu as branchée.": "Automatic: the overlay shows the controller you plugged in.",
  "Le modèle Xbox n'est pas installé : l'overlay affiche la DualSense.": "The Xbox model isn't installed: the overlay shows the DualSense.",

  // Remappage
  "Choisis quel bouton Xbox envoie chaque bouton de ta DualSense, pour ce profil (pendant l'émulation).": "Choose which Xbox button each DualSense button sends, for this profile (during emulation).",
  "Remettre par défaut": "Reset to default",

  // Réglages
  "Langue, démarrage et performances.": "Language, startup and performance.",
  "Langue": "Language",
  "Lancer MeikiCtrl avec Windows": "Start MeikiCtrl with Windows",
  "Démarre discrètement à côté de l'horloge.": "Starts quietly next to the clock.",
  "Relancer l'assistant de démarrage": "Run the setup assistant again",
  "Performances": "Performance",
  "Priorité GPU de l'overlay": "Overlay GPU priority",
  "Droits administrateur": "Admin rights",
  "Priorité du processus (comme OBS)": "Process priority (like OBS)",
  "Normale": "Normal",
  "Au-dessus de la normale (recommandé)": "Above normal (recommended)",
  "Haute": "High",
  "Mode administrateur": "Admin mode",
  "Garantit la priorité GPU « temps réel » (overlay fluide même à 500+ FPS) et supprime les demandes d'autorisation de HidHide.":
    "Guarantees “realtime” GPU priority (smooth overlay even at 500+ FPS) and removes HidHide permission prompts.",
  "À propos": "About",
  "MeikiCtrl v1.0 par Meikitoow. Gratuit, sans pub.": "MeikiCtrl v1.0 by Meikitoow. Free, no ads.",
  "Modèles 3D : « PS5 Controller » de Taohid Animation et « Xbox Controller-Black » d'ArtStudio640 (CC-BY 4.0).":
    "3D models: “PS5 Controller” by Taohid Animation and “Xbox Controller-Black” by ArtStudio640 (CC-BY 4.0).",
  "Fermer cette fenêtre la détruit : il ne reste que l'icône près de l'horloge, et l'interface ne consomme plus rien.":
    "Closing this window destroys it: only the icon next to the clock remains, and the interface uses nothing.",
  "♥ Soutenir sur Ko-fi": "♥ Support on Ko-fi",
  "MeikiCtrl se lancera avec Windows.": "MeikiCtrl will start with Windows.",
  "Démarrage automatique désactivé.": "Autostart turned off.",
  "Windows va demander l'autorisation administrateur…": "Windows will ask for admin permission…",
  "MeikiCtrl redémarre en administrateur…": "MeikiCtrl is restarting as admin…",
  "Mode administrateur actif.": "Admin mode on.",
  "Mode administrateur coupé.": "Admin mode off.",
  "Oui": "Yes",
  "Non": "No",
  "temps réel": "realtime",
  "haute": "high",
  "normale": "normal",

  // Assistant
  "On règle MeikiCtrl ensemble, ça prend une minute.": "Let's set up MeikiCtrl together, it takes a minute.",
  "Branche ta DualSense en USB ou connecte-la en Bluetooth. Une manette Xbox marche aussi.": "Plug in your DualSense over USB or connect it via Bluetooth. An Xbox controller works too.",
  "Les pilotes": "Drivers",
  "Deux petits outils gratuits de Nefarius, à installer une seule fois.": "Two small free tools by Nefarius, installed once.",
  "Nécessaire pour l'émulation Xbox.": "Required for Xbox emulation.",
  "Télécharger": "Download",
  "Optionnel : cache ta DualSense aux jeux pendant l'émulation.": "Optional: hides your DualSense from games during emulation.",
  "Vérifier à nouveau": "Check again",
  "Après une installation, redémarre MeikiCtrl pour qu'il les détecte.": "After installing, restart MeikiCtrl so it picks them up.",
  "Ton style": "Your style",
  "Toutes les 2 h de jeu.": "Every 2 hours of play.",
  "C'est parti !": "Let's go!",
  "Tout est prêt. Retrouve l'icône MeikiCtrl à côté de l'horloge : fermer la fenêtre ne l'arrête pas.": "All set. Find the MeikiCtrl icon next to the clock: closing the window doesn't stop it.",
  "MeikiCtrl est gratuit. Si tu veux soutenir le projet :": "MeikiCtrl is free. If you'd like to support the project:",
  "Retour": "Back",
  "Suivant": "Next",
  "Installé": "Installed",
  "Absent": "Not installed",
  "Manquant": "Missing",

  // Messages du moteur
  "Aucune manette connectée.": "No controller connected.",
  "Le stick a bougé pendant la mesure : lâche-le complètement et recommence.": "The stick moved during the measurement: let go of it completely and try again.",
  "Le mode administrateur sera coupé au prochain lancement de MeikiCtrl.": "Admin mode will be off next time MeikiCtrl starts.",
  "Autorisation administrateur refusée.": "Admin permission denied.",
  "Le programme n'a pas pu être lancé.": "The program couldn't be started.",
  "Impossible de retirer la tâche de démarrage (relance MeikiCtrl en administrateur).": "Couldn't remove the startup task (restart MeikiCtrl as admin).",
  "Le démarrage en mode administrateur demande que MeikiCtrl tourne déjà en administrateur.": "Starting in admin mode requires MeikiCtrl to already run as admin.",
  "Impossible de modifier le démarrage automatique.": "Couldn't change autostart.",
  "Création de la tâche planifiée refusée (droits administrateur nécessaires).": "Scheduled task creation denied (admin rights needed).",
};

// Messages du moteur avec un détail variable : on traduit le début.
const EN_PREFIX = [
  ["La manette virtuelle a été perdue", "The virtual controller was lost"],
  ["Pilote ViGEmBus introuvable", "ViGEmBus driver not found"],
  ["Installe « ViGEm Bus Driver ».", "Install “ViGEm Bus Driver”."],
  ["Branchement de la manette virtuelle refusé", "Virtual controller plug-in refused"],
  ["La manette virtuelle ne répond pas", "The virtual controller isn't responding"],
  ["Le programme a renvoyé une erreur", "The program returned an error"],
];

/** Traduit un texte français (avec {variables} éventuelles). */
function t(fr, vars) {
  let s = (LANG === "en" && EN[fr]) || fr;
  if (vars) for (const k in vars) s = s.split("{" + k + "}").join(vars[k]);
  return s;
}

/** Message d'erreur venant du moteur. */
function tErr(msg) {
  msg = String(msg);
  if (LANG !== "en") return msg;
  if (EN[msg]) return EN[msg];
  for (const [fr, en] of EN_PREFIX) msg = msg.split(fr).join(en);
  return msg;
}

/** Traduit tous les textes de la page (en gardant le français d'origine pour revenir en arrière). */
function translateDom(root = document.body) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let n;
  while ((n = walker.nextNode())) {
    if (n.parentNode && (n.parentNode.nodeName === "SCRIPT" || n.parentNode.nodeName === "STYLE")) continue;
    if (n.__fr === undefined) {
      const key = n.nodeValue.trim();
      if (!key || !(key in EN)) continue;
      n.__fr = n.nodeValue;
    }
    const key = n.__fr.trim();
    n.nodeValue = n.__fr.replace(key, t(key));
  }
  root.querySelectorAll("[placeholder],[title],[aria-label]").forEach((el) => {
    for (const a of ["placeholder", "title", "aria-label"]) {
      if (!el.hasAttribute(a)) continue;
      const store = "__fr_" + a;
      if (el[store] === undefined) el[store] = el.getAttribute(a);
      el.setAttribute(a, t(el[store]));
    }
  });
  document.documentElement.lang = LANG;
}
