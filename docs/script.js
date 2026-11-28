const STAGES = {
  kampung: {
    tier: 1,
    scale: 0.2,
    name: "Hamlet",
    era: "Village Era",
    cap: "Activity Cap: Lv 10",
    modifier: "Modifier: 1.00x",
    summary:
      "Begin with 100 Bac and 12 inventory slots. Run Farming and Gathering across 2 offline slots, build Well, upgrade Granary, and complete Village Hall.",
    projects: "Well, Granary (Lv 1–5), Village Hall",
    activities: "Farming, Gathering"
  },
  desa: {
    tier: 2,
    scale: 0.4,
    name: "Village",
    era: "Village Era",
    cap: "Activity Cap: Lv 20",
    modifier: "Modifier: 1.02x",
    summary:
      "Complete Village Hall to enter Village, unlock Fishing and Market Hall, and automatically apply stored activity XP overflow.",
    projects: "Market Hall, Town Hall",
    activities: "Farming, Gathering, Fishing"
  },
  kota_kecil: {
    tier: 3,
    scale: 0.6,
    name: "Town",
    era: "Town Era",
    cap: "Activity Cap: Lv 30",
    modifier: "Modifier: 1.05x",
    summary:
      "Complete Town Hall to unlock Mining for stone, iron ore, amethyst, and star crystal, expanding crafting chains and NPC orders.",
    projects: "Trade Port",
    activities: "Farming, Gathering, Fishing, Mining"
  },
  kota_dagang: {
    tier: 4,
    scale: 0.8,
    name: "Trade City",
    era: "City Era",
    cap: "Activity Cap: Lv 40",
    modifier: "Modifier: 1.10x",
    summary:
      "Construct Trade Port to enter the City Era, raising local economic modifiers and scaling processed goods across seasonal cycles.",
    projects: "Metropolis Tower",
    activities: "All 4 Activities (Cap Lv 40)"
  },
  metropolis: {
    tier: 5,
    scale: 1.0,
    name: "Metropolis",
    era: "Metropolis Era",
    cap: "Activity Cap: Lv 50",
    modifier: "Modifier: 1.15x",
    summary:
      "Raise Metropolis Tower to reach the maximum Lv 50 activity cap, peak local modifier, and full urban economy capacity.",
    projects: "Full City Infrastructure Complete",
    activities: "All 4 Activities (Cap Lv 50)"
  }
};

const PREVIEW_VIEWS = {
  market: {
    accentClass: "embed-economy",
    title: "Market & Broker (NPC Market)",
    pill: "Day 14 · Autumn",
    subLabel: "World Calendar",
    subValue: "Autumn · Trending: Village Bread",
    rows: [
      { label: "Wheat", value: "3.60 Bac", badge: "▲ +20%", badgeClass: "trend-up" },
      { label: "Village Bread", value: "24.50 Bac", badge: "▲ +15%", badgeClass: "trend-up" },
      { label: "River Fish", value: "9.10 Bac", badge: "▼ -8%", badgeClass: "trend-down" }
    ],
    footLabel: "Broker Fees",
    footValue: "1% Listing Fee (min 0.50 Bac) + 3% Commission"
  },
  crafting: {
    accentClass: "embed-crafting",
    title: "Crafting & NPC Orders",
    pill: "2 / 2 Active Slots",
    subLabel: "Production Queue",
    subValue: "Raw → Processed → Finished · Auto-Sell Ready",
    rows: [
      { label: "Slot 1 · Village Bread", value: "Ready in 4m", badge: "Processed", badgeClass: "trend-neutral" },
      { label: "Slot 2 · Iron Ingot", value: "Ready in 11m", badge: "Processed", badgeClass: "trend-neutral" },
      { label: "Order · Harbor Supply", value: "84.00 Bac", badge: "+25% Bonus", badgeClass: "trend-up" }
    ],
    footLabel: "Shop Reputation",
    footValue: "Lv 04 (+12% extra order bonus · 3 active orders max)"
  },
  crew: {
    accentClass: "embed-crew",
    title: "Crew & City Treasury",
    pill: "4 / 4 Crew Active",
    subLabel: "Governance Status",
    subValue: "Citizen Tax: 10% · Welfare: 100% (Zero-Debt)",
    rows: [
      { label: "Klara · Harvester", value: "Yield +12%", badge: "Farming", badgeClass: "trend-up" },
      { label: "Bram · Scout", value: "Luck +10%", badge: "Gathering", badgeClass: "trend-up" },
      { label: "Taro · Foreman", value: "AUTO Restart", badge: "Active", badgeClass: "trend-neutral" }
    ],
    footLabel: "City Treasury Ledger",
    footValue: "1,280.00 Bac · Next upkeep covered (0 debt)"
  }
};

const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

const menuButton = document.getElementById("menuToggle");
const nav = document.getElementById("nav");
const header = document.querySelector(".site-header");
const navLinks = Array.from(document.querySelectorAll('.nav a[href^="#"]'));

const stageTablist = document.querySelector(".stage-tablist");
const stageTabs = Array.from(document.querySelectorAll(".stage-tab"));
const stageTabIndicator = document.getElementById("stageTabIndicator");
const stagePanel = document.getElementById("stage-panel");
const stagePanelMain = document.getElementById("stagePanelMain");
const stageRailFill = document.getElementById("stageRailFill");
const stageSkyline = document.getElementById("stageSkyline");
const stageLayers = stageSkyline ? Array.from(stageSkyline.querySelectorAll("[data-era-layer]")) : [];

const stageEra = document.getElementById("stageEra");
const stageCap = document.getElementById("stageCap");
const stageModifier = document.getElementById("stageModifier");
const stageTitle = document.getElementById("stageTitle");
const stageSummary = document.getElementById("stageSummary");
const stageProjects = document.getElementById("stageProjects");
const stageActivities = document.getElementById("stageActivities");

const previewButtons = Array.from(document.querySelectorAll(".preview-action-btn"));
const previewDynamicEmbed = document.getElementById("previewDynamicEmbed");
const previewDynamicBody = document.getElementById("previewDynamicBody");
const previewTitle = document.getElementById("previewTitle");
const previewPill = document.getElementById("previewPill");
const previewSubLabel = document.getElementById("previewSubLabel");
const previewSubValue = document.getElementById("previewSubValue");
const previewRows = document.getElementById("previewRows");
const previewFootLabel = document.getElementById("previewFootLabel");
const previewFootValue = document.getElementById("previewFootValue");

let activeStageKey = "kampung";
let activePreviewKey = "market";
let stageSwapAnim = null;
let previewSwapAnim = null;

function runInterruptibleSwap(el, prevAnim) {
  if (!el || prefersReducedMotion.matches || typeof el.animate !== "function") {
    return null;
  }
  if (prevAnim) {
    prevAnim.cancel();
  }
  return el.animate(
    [
      { opacity: 0.4, transform: "translateY(4px)" },
      { opacity: 1, transform: "translateY(0)" }
    ],
    { duration: 180, easing: "cubic-bezier(0.22, 1, 0.36, 1)" }
  );
}

function updateMenuLabel(isOpen) {
  if (!menuButton) return;
  menuButton.setAttribute("aria-label", isOpen ? "Close navigation" : "Open navigation");
}

function setMenuOpen(isOpen, options = {}) {
  if (!menuButton || !nav) return;
  nav.classList.toggle("open", isOpen);
  menuButton.setAttribute("aria-expanded", String(isOpen));
  updateMenuLabel(isOpen);

  if (isOpen && options.focusFirstLink) {
    const firstLink = nav.querySelector("a");
    if (firstLink) firstLink.focus();
  } else if (!isOpen && options.returnFocus) {
    menuButton.focus();
  }
}

function updateTabIndicator(activeTab) {
  if (!stageTabIndicator || !stageTablist || !activeTab) return;
  const trackWidth = stageTablist.scrollWidth || stageTablist.clientWidth;
  if (!trackWidth) return;
  const x = activeTab.offsetLeft;
  const scaleX = activeTab.offsetWidth / trackWidth;
  stageTabIndicator.style.transform = `translateX(${x}px) scaleX(${scaleX})`;
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
    updateTabIndicator(activeTab);
    if (animate && window.innerWidth <= 720) {
      activeTab.scrollIntoView({
        behavior: prefersReducedMotion.matches ? "auto" : "smooth",
        inline: "nearest",
        block: "nearest"
      });
    }
  }

  if (stageRailFill) {
    stageRailFill.style.transform = `scaleX(${stage.scale})`;
  }

  if (stageSkyline) {
    stageSkyline.setAttribute("data-active-era", String(stage.tier));
    stageLayers.forEach(layer => {
      const layerTier = Number(layer.dataset.eraLayer || "1");
      layer.classList.toggle("is-visible", layerTier <= stage.tier);
    });
  }

  stageEra.textContent = stage.era;
  stageCap.textContent = stage.cap;
  stageModifier.textContent = stage.modifier;
  stageTitle.textContent = stage.name;
  stageSummary.textContent = stage.summary;
  stageProjects.textContent = stage.projects;
  stageActivities.textContent = stage.activities;

  if (animate) {
    stageSwapAnim = runInterruptibleSwap(stagePanelMain, stageSwapAnim);
  }
}

function renderPreview(previewKey, animate) {
  const view = PREVIEW_VIEWS[previewKey];
  if (!view || !previewDynamicEmbed) return;

  activePreviewKey = previewKey;

  previewButtons.forEach(btn => {
    const selected = btn.dataset.preview === previewKey;
    btn.classList.toggle("is-active", selected);
    btn.setAttribute("aria-pressed", String(selected));
  });

  previewDynamicEmbed.classList.remove("embed-economy", "embed-crafting", "embed-crew");
  previewDynamicEmbed.classList.add(view.accentClass);

  previewTitle.textContent = view.title;
  previewPill.textContent = view.pill;
  previewSubLabel.textContent = view.subLabel;
  previewSubValue.textContent = view.subValue;
  previewFootLabel.textContent = view.footLabel;
  previewFootValue.textContent = view.footValue;

  if (previewRows) {
    previewRows.replaceChildren(
      ...view.rows.map(row => {
        const rowEl = document.createElement("div");
        rowEl.className = "market-row";

        const labelSpan = document.createElement("span");
        labelSpan.textContent = row.label;

        const rightSpan = document.createElement("span");
        rightSpan.className = "mono";
        rightSpan.textContent = `${row.value} `;

        const badgeEl = document.createElement("em");
        badgeEl.className = row.badgeClass;
        badgeEl.textContent = row.badge;

        rightSpan.appendChild(badgeEl);
        rowEl.append(labelSpan, rightSpan);
        return rowEl;
      })
    );
  }

  if (animate) {
    previewSwapAnim = runInterruptibleSwap(previewDynamicBody, previewSwapAnim);
  }
}

if (menuButton && nav) {
  menuButton.addEventListener("click", () => {
    const nextOpen = !nav.classList.contains("open");
    setMenuOpen(nextOpen, { focusFirstLink: nextOpen });
  });

  nav.querySelectorAll("a").forEach(link => {
    link.addEventListener("click", () => setMenuOpen(false));
  });

  document.addEventListener("keydown", event => {
    if (event.key === "Escape" && nav.classList.contains("open")) {
      setMenuOpen(false, { returnFocus: true });
    }
  });

  document.addEventListener("click", event => {
    if (nav.classList.contains("open") && header && !header.contains(event.target)) {
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
      renderStage(targetTab.dataset.stage, false);
    }
  });
});

previewButtons.forEach(btn => {
  btn.addEventListener("click", event => {
    const isKeyboard = event.detail === 0;
    renderPreview(btn.dataset.preview, !isKeyboard);
  });
});

let resizeRaf = 0;
window.addEventListener("resize", () => {
  if (window.innerWidth > 900 && nav && nav.classList.contains("open")) {
    setMenuOpen(false);
  }
  cancelAnimationFrame(resizeRaf);
  resizeRaf = requestAnimationFrame(() => {
    updateTabIndicator(document.getElementById(`tab-${activeStageKey}`));
  });
});

// Scroll-spy for sticky navigation links
if ("IntersectionObserver" in window && navLinks.length > 0) {
  const sections = navLinks
    .map(link => document.querySelector(link.getAttribute("href")))
    .filter(Boolean);

  const setActiveNav = id => {
    navLinks.forEach(link => {
      const match = link.getAttribute("href") === `#${id}`;
      link.classList.toggle("is-current", match);
      if (match) {
        link.setAttribute("aria-current", "location");
      } else {
        link.removeAttribute("aria-current");
      }
    });
  };

  const observer = new IntersectionObserver(
    entries => {
      const visible = entries
        .filter(entry => entry.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
      if (visible) {
        setActiveNav(visible.target.id);
      }
    },
    { rootMargin: "-25% 0px -55% 0px", threshold: [0.1, 0.35, 0.6] }
  );

  sections.forEach(section => observer.observe(section));
}

const yearEl = document.getElementById("year");
if (yearEl) {
  yearEl.textContent = String(new Date().getFullYear());
}

updateMenuLabel(false);
renderStage(activeStageKey, false);
renderPreview(activePreviewKey, false);
