const STAGES = {
  kampung: {
    progress: "20%",
    name: "Kampung",
    era: "Village Era",
    cap: { en: "Activity Cap: Lv 10", id: "Batas Level Aktivitas: Lv 10" },
    modifier: { en: "Local Modifier: 1.00x", id: "Modifier Lokal: 1.00x" },
    summary: {
      en: "Start with 100 Bac and 12 inventory slots. Run Farming and Gathering across 2 offline slots, build Sumur (Well), upgrade Lumbung (+4 slots per level), and complete Balai Kampung.",
      id: "Mulai dengan 100 Bac dan 12 slot tas. Jalankan Farming dan Gathering di 2 slot offline, bangun Sumur, upgrade Lumbung (+4 slot per level), dan selesaikan Balai Kampung."
    },
    projects: {
      en: "Sumur, Lumbung (Lv 1–5), Balai Kampung",
      id: "Sumur, Lumbung (Lv 1–5), Balai Kampung"
    },
    activities: {
      en: "Farming, Gathering",
      id: "Farming, Gathering"
    }
  },
  desa: {
    progress: "40%",
    name: "Desa",
    era: "Village Era",
    cap: { en: "Activity Cap: Lv 20", id: "Batas Level Aktivitas: Lv 20" },
    modifier: { en: "Local Modifier: 1.02x", id: "Modifier Lokal: 1.02x" },
    summary: {
      en: "Completing Balai Kampung raises your city to Desa, opens Fishing and Pasar Desa, and automatically applies any stored activity XP overflow from Kampung.",
      id: "Menyelesaikan Balai Kampung menaikkan kotamu ke Desa, membuka Fishing dan Pasar Desa, serta langsung menerapkan XP overflow aktivitas dari Kampung."
    },
    projects: {
      en: "Pasar Desa, Balai Kota Kecil",
      id: "Pasar Desa, Balai Kota Kecil"
    },
    activities: {
      en: "Farming, Gathering, Fishing",
      id: "Farming, Gathering, Fishing"
    }
  },
  kota_kecil: {
    progress: "60%",
    name: "Kota Kecil",
    era: "Town Era",
    cap: { en: "Activity Cap: Lv 30", id: "Batas Level Aktivitas: Lv 30" },
    modifier: { en: "Local Modifier: 1.05x", id: "Modifier Lokal: 1.05x" },
    summary: {
      en: "Complete Balai Kota Kecil to open Mining for stone, iron ore, amethyst, and star crystal, expanding mid-game crafting chains and higher-tier NPC orders.",
      id: "Selesaikan Balai Kota Kecil untuk membuka Mining (batu, bijih besi, kecubung, kristal bintang), memperluas rantai crafting dan pesanan NPC bernilai tinggi."
    },
    projects: {
      en: "Pelabuhan Dagang",
      id: "Pelabuhan Dagang"
    },
    activities: {
      en: "Farming, Gathering, Fishing, Mining",
      id: "Farming, Gathering, Fishing, Mining"
    }
  },
  kota_dagang: {
    progress: "80%",
    name: "Kota Dagang",
    era: "City Era",
    cap: { en: "Activity Cap: Lv 40", id: "Batas Level Aktivitas: Lv 40" },
    modifier: { en: "Local Modifier: 1.10x", id: "Modifier Lokal: 1.10x" },
    summary: {
      en: "Construct Pelabuhan Dagang to enter the City Era, boosting local economic modifiers and scaling production of processed and finished goods across seasonal demand cycles.",
      id: "Bangun Pelabuhan Dagang untuk memasuki City Era, meningkatkan modifier ekonomi lokal, dan mempercepat produksi barang olahan di siklus pasar musiman."
    },
    projects: {
      en: "Menara Metropolis",
      id: "Menara Metropolis"
    },
    activities: {
      en: "All 4 Activities (Cap Lv 40)",
      id: "Semua 4 Aktivitas (Cap Lv 40)"
    }
  },
  metropolis: {
    progress: "100%",
    name: "Metropolis",
    era: "Metropolis Era",
    cap: { en: "Activity Cap: Lv 50", id: "Batas Level Aktivitas: Lv 50" },
    modifier: { en: "Local Modifier: 1.15x", id: "Modifier Lokal: 1.15x" },
    summary: {
      en: "Raise Menara Metropolis to reach the highest activity level cap, maximum local modifier, and full urban economy capacity.",
      id: "Dirikan Menara Metropolis untuk mencapai batas level aktivitas tertinggi, modifier lokal maksimal, dan kapasitas ekonomi kota penuh."
    },
    projects: {
      en: "Full City Infrastructure Complete",
      id: "Seluruh Infrastruktur Kota Selesai"
    },
    activities: {
      en: "All 4 Activities (Cap Lv 50)",
      id: "Semua 4 Aktivitas (Cap Lv 50)"
    }
  }
};

const META_TEXT = {
  en: {
    title: "Urbaniac — A Discord-Based Urban Life RPG",
    description:
      "Build your city from Kampung to Metropolis on Discord. Manage offline activities, dynamic NPC markets, crafting, crew automation, and city economy.",
    menuOpen: "Close navigation",
    menuClosed: "Open navigation",
    langButton: "ID",
    langAria: "Switch to Indonesian"
  },
  id: {
    title: "Urbaniac — RPG Ekonomi & Pembangunan Kota di Discord",
    description:
      "Bangun kotamu dari Kampung hingga Metropolis di Discord. Kelola aktivitas offline, pasar NPC dinamis, crafting, otomasi kru, dan ekonomi kota.",
    menuOpen: "Tutup navigasi",
    menuClosed: "Buka navigasi",
    langButton: "EN",
    langAria: "Switch to English"
  }
};

const menuButton = document.getElementById("menuToggle");
const nav = document.getElementById("nav");
const header = document.querySelector(".site-header");
const langToggle = document.getElementById("langToggle");
const stageTabs = Array.from(document.querySelectorAll(".stage-tab"));
const stagePanel = document.getElementById("stage-panel");
const stageRailFill = document.getElementById("stageRailFill");
const stageEra = document.getElementById("stageEra");
const stageCap = document.getElementById("stageCap");
const stageModifier = document.getElementById("stageModifier");
const stageTitle = document.getElementById("stageTitle");
const stageSummary = document.getElementById("stageSummary");
const stageProjects = document.getElementById("stageProjects");
const stageActivities = document.getElementById("stageActivities");

let currentLanguage = "en";
let activeStageKey = "kampung";

function updateMenuLabel(isOpen) {
  if (!menuButton) return;
  const copy = META_TEXT[currentLanguage];
  menuButton.setAttribute("aria-label", isOpen ? copy.menuOpen : copy.menuClosed);
}

function setMenuOpen(isOpen) {
  if (!menuButton || !nav) return;
  nav.classList.toggle("open", isOpen);
  menuButton.setAttribute("aria-expanded", String(isOpen));
  updateMenuLabel(isOpen);
}

function renderStage(stageKey, animate) {
  const stage = STAGES[stageKey];
  if (!stage || !stagePanel) return;

  activeStageKey = stageKey;
  const activeTab = document.getElementById(`tab-${stageKey}`);

  stageTabs.forEach(tab => {
    const selected = tab.dataset.stage === stageKey;
    tab.classList.toggle("is-active", selected);
    tab.setAttribute("aria-selected", String(selected));
    tab.setAttribute("tabindex", selected ? "0" : "-1");
  });

  if (activeTab) {
    stagePanel.setAttribute("aria-labelledby", activeTab.id);
  }
  if (stageRailFill) {
    stageRailFill.style.width = stage.progress;
  }

  stageEra.textContent = stage.era;
  stageCap.textContent = stage.cap[currentLanguage];
  stageModifier.textContent = stage.modifier[currentLanguage];
  stageTitle.textContent = stage.name;
  stageSummary.textContent = stage.summary[currentLanguage];
  stageProjects.textContent = stage.projects[currentLanguage];
  stageActivities.textContent = stage.activities[currentLanguage];

  if (animate) {
    stagePanel.classList.remove("is-animating");
    void stagePanel.offsetWidth;
    stagePanel.classList.add("is-animating");
  }
}

function setLanguage(language) {
  currentLanguage = language === "id" ? "id" : "en";
  const meta = META_TEXT[currentLanguage];

  document.documentElement.lang = currentLanguage;
  document.title = meta.title;

  const descMeta = document.querySelector('meta[name="description"]');
  if (descMeta) {
    descMeta.setAttribute("content", meta.description);
  }

  document.querySelectorAll("[data-en][data-id]").forEach(element => {
    element.textContent = element.dataset[currentLanguage];
  });

  if (langToggle) {
    langToggle.textContent = meta.langButton;
    langToggle.setAttribute("aria-label", meta.langAria);
  }

  updateMenuLabel(nav ? nav.classList.contains("open") : false);
  renderStage(activeStageKey, false);

  try {
    localStorage.setItem("urbaniac_lang", currentLanguage);
  } catch {
    // Ignore storage restriction errors in private contexts
  }
}

if (menuButton && nav) {
  menuButton.addEventListener("click", () => {
    setMenuOpen(!nav.classList.contains("open"));
  });

  nav.querySelectorAll("a").forEach(link => {
    link.addEventListener("click", () => setMenuOpen(false));
  });

  document.addEventListener("keydown", event => {
    if (event.key === "Escape" && nav.classList.contains("open")) {
      setMenuOpen(false);
      menuButton.focus();
    }
  });

  document.addEventListener("click", event => {
    if (
      nav.classList.contains("open") &&
      header &&
      !header.contains(event.target)
    ) {
      setMenuOpen(false);
    }
  });
}

stageTabs.forEach((tab, index) => {
  tab.addEventListener("click", () => {
    renderStage(tab.dataset.stage, true);
  });

  tab.addEventListener("keydown", event => {
    let nextIndex = null;
    if (event.key === "ArrowRight") {
      nextIndex = (index + 1) % stageTabs.length;
    } else if (event.key === "ArrowLeft") {
      nextIndex = (index - 1 + stageTabs.length) % stageTabs.length;
    } else if (event.key === "Home") {
      nextIndex = 0;
    } else if (event.key === "End") {
      nextIndex = stageTabs.length - 1;
    }

    if (nextIndex !== null) {
      event.preventDefault();
      const targetTab = stageTabs[nextIndex];
      targetTab.focus();
      renderStage(targetTab.dataset.stage, true);
    }
  });
});

if (langToggle) {
  langToggle.addEventListener("click", () => {
    setLanguage(currentLanguage === "en" ? "id" : "en");
  });
}

const yearEl = document.getElementById("year");
if (yearEl) {
  yearEl.textContent = String(new Date().getFullYear());
}

let savedLanguage = "en";
try {
  const stored = localStorage.getItem("urbaniac_lang");
  if (stored === "id" || stored === "en") {
    savedLanguage = stored;
  }
} catch {
  savedLanguage = "en";
}
setLanguage(savedLanguage);
