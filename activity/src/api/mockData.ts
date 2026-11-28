import type {
  ActivitiesResponse,
  ActivityKindKey,
  ActivityRunDto,
  BrokerListingDto,
  CancelListingResponse,
  CityResponse,
  CitySnapshot,
  CityStageKey,
  ClaimPendingResponse,
  CollectActivitiesResponse,
  CraftRunDto,
  CraftingResponse,
  CrewMemberDto,
  CrewResponse,
  FulfillOrderResponse,
  InventoryItemDto,
  InventoryResponse,
  MarketPriceDto,
  MarketResponse,
  MarketSellResponse,
  MeResponse,
  MockPreset,
  ProjectKey,
  ProjectQuoteDto,
  RegisterResponse,
  RunningProjectDto,
  SetTaxResponse,
  TreasuryBalanceResponse,
} from "./types";

const STAGE_ORDER: CityStageKey[] = [
  "kampung",
  "desa",
  "kota_kecil",
  "kota_dagang",
  "metropolis",
];

const STAGE_META: Record<
  CityStageKey,
  { name: string; era: string; slots: number; cap: number }
> = {
  kampung: { name: "Hamlet", era: "Village Era", slots: 2, cap: 10 },
  desa: { name: "Village", era: "Village Era", slots: 3, cap: 20 },
  kota_kecil: { name: "Town", era: "Town Era", slots: 4, cap: 30 },
  kota_dagang: { name: "Trade City", era: "City Era", slots: 4, cap: 40 },
  metropolis: { name: "Metropolis", era: "Metropolis Era", slots: 4, cap: 50 },
};

const ITEM_META: Record<
  string,
  { name: string; rarity: string; basePrice: number; npcPrice: string }
> = {
  wheat: { name: "Wheat", rarity: "Common", basePrice: 3.0, npcPrice: "3.12" },
  potato: { name: "Potato", rarity: "Common", basePrice: 4.0, npcPrice: "4.10" },
  wood: { name: "Wood", rarity: "Common", basePrice: 3.0, npcPrice: "2.95" },
  berries: { name: "Forest Berries", rarity: "Common", basePrice: 3.5, npcPrice: "3.50" },
  wild_herb: { name: "Wild Herb", rarity: "Uncommon", basePrice: 12.0, npcPrice: "12.40" },
  flour: { name: "Wheat Flour", rarity: "Common", basePrice: 9.5, npcPrice: "9.80" },
  plank: { name: "Wood Plank", rarity: "Common", basePrice: 9.5, npcPrice: "10.10" },
  herb_extract: { name: "Herb Extract", rarity: "Uncommon", basePrice: 40.0, npcPrice: "42.00" },
  bread: { name: "Village Bread", rarity: "Common", basePrice: 28.0, npcPrice: "29.90" },
  wooden_crate: { name: "Wooden Crate", rarity: "Uncommon", basePrice: 32.0, npcPrice: "34.00" },
  healing_tonic: { name: "Herbal Tonic", rarity: "Rare", basePrice: 68.0, npcPrice: "71.50" },
  river_fish: { name: "River Fish", rarity: "Common", basePrice: 5.0, npcPrice: "5.40" },
  silver_carp: { name: "Silver Carp", rarity: "Uncommon", basePrice: 16.0, npcPrice: "17.10" },
  golden_eel: { name: "Golden Eel", rarity: "Rare", basePrice: 45.0, npcPrice: "48.20" },
  stone: { name: "Stone", rarity: "Common", basePrice: 4.5, npcPrice: "4.50" },
  iron_ore: { name: "Iron Ore", rarity: "Uncommon", basePrice: 18.0, npcPrice: "18.90" },
  amethyst: { name: "Amethyst", rarity: "Rare", basePrice: 60.0, npcPrice: "64.00" },
  star_crystal: { name: "Star Crystal", rarity: "Epic", basePrice: 150.0, npcPrice: "162.50" },
};

const RECIPE_CATALOG: Array<{
  id: string;
  name: string;
  tier: "processed" | "finished";
  tier_name: string;
  output_item_id: string;
  output_qty: number;
  duration_secs: number;
  xp_per_unit: number;
  inputs: Array<{ item_id: string; name: string; need: number }>;
}> = [
  {
    id: "flour",
    name: "Wheat Flour",
    tier: "processed",
    tier_name: "Processed",
    output_item_id: "flour",
    output_qty: 1,
    duration_secs: 12,
    xp_per_unit: 10,
    inputs: [{ item_id: "wheat", name: "Wheat", need: 2 }],
  },
  {
    id: "plank",
    name: "Wood Plank",
    tier: "processed",
    tier_name: "Processed",
    output_item_id: "plank",
    output_qty: 1,
    duration_secs: 12,
    xp_per_unit: 10,
    inputs: [{ item_id: "wood", name: "Wood", need: 2 }],
  },
  {
    id: "herb_extract",
    name: "Herb Extract",
    tier: "processed",
    tier_name: "Processed",
    output_item_id: "herb_extract",
    output_qty: 1,
    duration_secs: 15,
    xp_per_unit: 10,
    inputs: [
      { item_id: "wild_herb", name: "Wild Herb", need: 2 },
      { item_id: "berries", name: "Forest Berries", need: 1 },
    ],
  },
  {
    id: "bread",
    name: "Village Bread",
    tier: "finished",
    tier_name: "Finished",
    output_item_id: "bread",
    output_qty: 1,
    duration_secs: 18,
    xp_per_unit: 20,
    inputs: [{ item_id: "flour", name: "Wheat Flour", need: 2 }],
  },
  {
    id: "wooden_crate",
    name: "Wooden Crate",
    tier: "finished",
    tier_name: "Finished",
    output_item_id: "wooden_crate",
    output_qty: 1,
    duration_secs: 18,
    xp_per_unit: 20,
    inputs: [
      { item_id: "plank", name: "Wood Plank", need: 2 },
      { item_id: "wood", name: "Wood", need: 1 },
    ],
  },
  {
    id: "healing_tonic",
    name: "Herbal Tonic",
    tier: "finished",
    tier_name: "Finished",
    output_item_id: "healing_tonic",
    output_qty: 1,
    duration_secs: 20,
    xp_per_unit: 20,
    inputs: [
      { item_id: "herb_extract", name: "Herb Extract", need: 1 },
      { item_id: "berries", name: "Forest Berries", need: 2 },
    ],
  },
];

const ACTIVITY_DROPS: Record<ActivityKindKey, Array<{ id: string; qty: number }>> = {
  farming: [
    { id: "wheat", qty: 5 },
    { id: "potato", qty: 3 },
  ],
  gathering: [
    { id: "wood", qty: 6 },
    { id: "berries", qty: 4 },
    { id: "wild_herb", qty: 1 },
  ],
  fishing: [
    { id: "river_fish", qty: 5 },
    { id: "silver_carp", qty: 2 },
  ],
  mining: [
    { id: "stone", qty: 6 },
    { id: "iron_ore", qty: 3 },
  ],
};

function stageGte(current: CityStageKey, min: CityStageKey): boolean {
  return STAGE_ORDER.indexOf(current) >= STAGE_ORDER.indexOf(min);
}

function isoAfter(secs: number): string {
  return new Date(Date.now() + secs * 1000).toISOString();
}

function buildQuotes(
  stage: CityStageKey,
  facilities: Record<ProjectKey, number>,
  runningProject: ProjectKey | null,
  availableBac: number,
  stock: Record<string, number>,
): ProjectQuoteDto[] {
  const defs: Array<{
    project: ProjectKey;
    name: string;
    description: string;
    max_level: number;
    min_stage: CityStageKey;
    min_stage_name: string;
    cost_bac: number;
    duration_secs: number;
    items: Array<{ item_id: string; name: string; need: number }>;
  }> = [
    {
      project: "well",
      name: "Well",
      description: "+10% Farming & Gathering yield bonus. Prerequisite for Village Hall.",
      max_level: 1,
      min_stage: "kampung",
      min_stage_name: "Hamlet",
      cost_bac: 50,
      duration_secs: 25,
      items: [{ item_id: "wood", name: "Wood", need: 10 }],
    },
    {
      project: "granary",
      name: "Granary",
      description:
        "Adds +4 bag slots & +3% yield to all activities per level (max Lv 5).",
      max_level: 5,
      min_stage: "kampung",
      min_stage_name: "Hamlet",
      cost_bac: 80,
      duration_secs: 25,
      items: [
        { item_id: "wood", name: "Wood", need: 15 },
        { item_id: "wheat", name: "Wheat", need: 10 },
      ],
    },
    {
      project: "town_hall",
      name: "Village Hall",
      description:
        "Advance city to Village (unlocks Fishing, 3 Activity Slots, Lv 20 cap; requires Well & Granary).",
      max_level: 1,
      min_stage: "kampung",
      min_stage_name: "Hamlet",
      cost_bac: 150,
      duration_secs: 25,
      items: [
        { item_id: "wood", name: "Wood", need: 25 },
        { item_id: "potato", name: "Potato", need: 10 },
      ],
    },
    {
      project: "market_hall",
      name: "Market Hall",
      description: "+5% NPC sell price bonus. Prerequisite for Town Hall.",
      max_level: 1,
      min_stage: "desa",
      min_stage_name: "Village",
      cost_bac: 200,
      duration_secs: 25,
      items: [
        { item_id: "plank", name: "Wood Plank", need: 10 },
        { item_id: "bread", name: "Village Bread", need: 5 },
      ],
    },
    {
      project: "city_hall",
      name: "Town Hall",
      description:
        "Advance city to Town (unlocks Mining, 4 Activity Slots, Lv 30 cap; requires Market Hall).",
      max_level: 1,
      min_stage: "desa",
      min_stage_name: "Village",
      cost_bac: 350,
      duration_secs: 25,
      items: [
        { item_id: "wooden_crate", name: "Wooden Crate", need: 5 },
        { item_id: "river_fish", name: "River Fish", need: 10 },
      ],
    },
    {
      project: "trade_port",
      name: "Trade Port",
      description:
        "Advance city to Trade City (Lv 40 cap, 3 Craft Slots, +15% NPC Order rewards).",
      max_level: 1,
      min_stage: "kota_kecil",
      min_stage_name: "Town",
      cost_bac: 600,
      duration_secs: 25,
      items: [
        { item_id: "plank", name: "Wood Plank", need: 20 },
        { item_id: "iron_ore", name: "Iron Ore", need: 10 },
      ],
    },
    {
      project: "metropolis_tower",
      name: "Metropolis Tower",
      description:
        "Advance city to Metropolis (Lv 50 cap, +15% Activity & Craft speed).",
      max_level: 1,
      min_stage: "kota_dagang",
      min_stage_name: "Trade City",
      cost_bac: 1200,
      duration_secs: 25,
      items: [
        { item_id: "amethyst", name: "Amethyst", need: 5 },
        { item_id: "healing_tonic", name: "Herbal Tonic", need: 5 },
      ],
    },
  ];

  return defs.map((d) => {
    const cur = facilities[d.project] ?? 0;
    if (cur >= d.max_level) {
      return {
        project: d.project,
        name: d.name,
        description: d.description,
        current_level: cur,
        next_level: null,
        max_level: d.max_level,
        min_stage: d.min_stage,
        min_stage_name: d.min_stage_name,
        cost_bac: "0.00",
        available_bac: availableBac.toFixed(2),
        duration_secs: 0,
        items: [],
        can_start: false,
        block_reason: `facility ${d.name} has already reached maximum level`,
      };
    }

    const nextLvl = cur + 1;
    const costBac = d.cost_bac * nextLvl;
    const items = d.items.map((it) => ({
      item_id: it.item_id,
      name: it.name,
      need: it.need * nextLvl,
      have: stock[it.item_id] ?? 0,
    }));

    let blockReason: string | null = null;
    if (runningProject) {
      const runningName =
        defs.find((x) => x.project === runningProject)?.name ?? runningProject;
      blockReason = `project ${runningName} is already in progress`;
    } else if (!stageGte(stage, d.min_stage)) {
      blockReason = `this project requires city stage ${d.min_stage_name}`;
    } else if (
      d.project === "town_hall" &&
      (facilities.well === 0 || facilities.granary === 0)
    ) {
      blockReason = "facility prerequisites not met (requires Well & Granary)";
    } else if (d.project === "city_hall" && facilities.market_hall === 0) {
      blockReason = "facility prerequisites not met (requires Market Hall)";
    } else if (availableBac < costBac) {
      blockReason = `insufficient Bac balance (need ${costBac.toFixed(2)}, available ${availableBac.toFixed(2)})`;
    } else {
      const missing = items.find((i) => i.have < i.need);
      if (missing) {
        blockReason = `insufficient ${missing.name} (need ${missing.need}, have ${missing.have})`;
      }
    }

    return {
      project: d.project,
      name: d.name,
      description: d.description,
      current_level: cur,
      next_level: nextLvl,
      max_level: d.max_level,
      min_stage: d.min_stage,
      min_stage_name: d.min_stage_name,
      cost_bac: costBac.toFixed(2),
      available_bac: availableBac.toFixed(2),
      duration_secs: d.duration_secs,
      items,
      can_start: blockReason === null,
      block_reason: blockReason,
    };
  });
}

function defaultMarketPrices(): MarketPriceDto[] {
  return Object.entries(ITEM_META).map(([item_id, m]) => {
    const cur = Number(m.npcPrice);
    const mult = cur / m.basePrice;
    return {
      item_id,
      name: m.name,
      rarity: m.rarity,
      base_price: m.basePrice.toFixed(2),
      current_price: cur.toFixed(2),
      multiplier: mult.toFixed(2),
    };
  });
}

interface MockState {
  me: MeResponse;
  city: CityResponse | null;
  inventory: InventoryResponse | null;
  activities: ActivitiesResponse | null;
  market: MarketResponse | null;
  crafting: CraftingResponse | null;
  crew: CrewResponse | null;
  nextId: number;
  usedKeys: Set<string>;
}

function createInitialState(preset: MockPreset): MockState {
  const nowIso = new Date().toISOString();

  if (preset === "unregistered") {
    return {
      me: {
        registered: false,
        discord_user_id: "123456789012345678",
        username: "NewMayor",
        avatar_url: null,
        profile: null,
      },
      city: null,
      inventory: null,
      activities: null,
      market: null,
      crafting: null,
      crew: null,
      nextId: 100,
      usedKeys: new Set(),
    };
  }

  const prices = defaultMarketPrices();
  const baseCalendar = {
    day: 3,
    season: "spring",
    season_name: "Spring",
    season_hint: "+10% Potato, Berries, Herb",
    active_event: "harvest_festival",
    active_event_name: "Harvest Festival",
    event_hint: "+20% Wheat, Potato, Bread",
    trend_item_id: "wheat",
    trend_item_name: "Wheat",
  };

  const baseOrders = [
    { id: 1, item_id: "flour", name: "Wheat Flour", quantity: 2, have: 0, reward_bac: "25.65", rep_xp: 30, can_fulfill: false },
    { id: 2, item_id: "plank", name: "Wood Plank", quantity: 2, have: 0, reward_bac: "25.65", rep_xp: 30, can_fulfill: false },
    { id: 3, item_id: "bread", name: "Village Bread", quantity: 1, have: 0, reward_bac: "37.80", rep_xp: 15, can_fulfill: false },
  ];

  if (preset === "desa") {
    const items: InventoryItemDto[] = [
      { item_id: "wheat", name: "Wheat", rarity: "Common", quantity: 34, npc_unit_price: "3.25" },
      { item_id: "wood", name: "Wood", rarity: "Common", quantity: 42, npc_unit_price: "3.15" },
      { item_id: "river_fish", name: "River Fish", rarity: "Common", quantity: 14, npc_unit_price: "5.40" },
      { item_id: "silver_carp", name: "Silver Carp", rarity: "Uncommon", quantity: 4, npc_unit_price: "17.10" },
      { item_id: "wooden_crate", name: "Wooden Crate", rarity: "Uncommon", quantity: 6, npc_unit_price: "34.00" },
    ];
    return {
      me: {
        registered: true,
        discord_user_id: "123456789012345678",
        username: "CitizenDev",
        avatar_url: null,
        profile: {
          level: 9,
          xp: 420,
          xp_needed: 900,
          balance: "485.50",
          city_stage: "desa",
          city_stage_name: "Village",
          lang: "en",
          slots_used: items.length,
          slot_capacity: 20,
        },
      },
      city: {
        stage: "desa",
        stage_name: "Village",
        era: "Village Era",
        treasury: "210.00",
        wallet_balance: "485.50",
        tax_rate_pct: 10,
        welfare: 94,
        effectiveness_pct: 111,
        yield_bonus_pct: 11,
        est_tax_income: "63.00",
        est_expense: "28.00",
        assigned_crews: 1,
        facilities: [
          { id: "well", name: "Well", level: 1 },
          { id: "granary", name: "Granary", level: 2 },
          { id: "town_hall", name: "Village Hall", level: 1 },
          { id: "market_hall", name: "Market Hall", level: 1 },
        ],
        running_project: null,
        server_time: nowIso,
      },
      inventory: { slots_used: items.length, slot_capacity: 20, items, pending: [] },
      activities: {
        stage: "desa",
        stage_name: "Village",
        slots_total: 3,
        runs: [
          { id: 21, kind: "fishing", name: "Fishing", status: "running", ends_at: isoAfter(25) },
          { id: 22, kind: "farming", name: "Farming", status: "ready", ends_at: isoAfter(-20) },
        ],
        progress: [
          { kind: "farming", name: "Farming", level: 12, xp: 340, xp_needed: 1200, overflow: 0, cap: 20 },
          { kind: "gathering", name: "Gathering", level: 10, xp: 810, xp_needed: 1000, overflow: 0, cap: 20 },
          { kind: "fishing", name: "Fishing", level: 4, xp: 150, xp_needed: 400, overflow: 0, cap: 20 },
        ],
        catalog: [
          { kind: "farming", name: "Farming", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 35 },
          { kind: "gathering", name: "Gathering", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 25 },
          { kind: "fishing", name: "Fishing", min_stage: "desa", min_stage_name: "Village", unlocked: true, base_duration_secs: 20, xp_reward: 40 },
          { kind: "mining", name: "Mining", min_stage: "kota_kecil", min_stage_name: "Town", unlocked: false, base_duration_secs: 20, xp_reward: 55 },
        ],
        server_time: nowIso,
      },
      market: {
        wallet_balance: "485.50",
        calendar: baseCalendar,
        prices,
        listings: [],
        max_listings: 5,
        fee_pct: 1,
        commission_pct: 3,
        server_time: nowIso,
      },
      crafting: {
        max_concurrent: 2,
        rep_level: 2,
        rep_xp: 35,
        rep_xp_needed: 100,
        rep_bonus_pct: 3,
        completed_orders: 3,
        auto_sell: false,
        running: [],
        orders: baseOrders,
        recipes: [],
        server_time: nowIso,
      },
      crew: {
        max_slots: 4,
        max_level: 10,
        recruit_cost: "100.00",
        members: [
          {
            id: 1,
            name: "Sari",
            rarity: "Uncommon",
            specialty: "fishing",
            specialty_name: "Fishing",
            level: 2,
            assigned: "fishing",
            assigned_name: "Fishing",
            auto_enabled: false,
            efficiency_pct: 13,
            yield_pct: 13,
            luck_pct: 13,
            upgrade_cost: "200.00",
          },
        ],
      },
      nextId: 100,
      usedKeys: new Set(),
    };
  }

  if (preset === "kota_kecil" || preset === "kota_dagang") {
    const isTrade = preset === "kota_dagang";
    const stage: CityStageKey = isTrade ? "kota_dagang" : "kota_kecil";
    const stageName = isTrade ? "Trade City" : "Town";
    const era = isTrade ? "City Era" : "Town Era";
    const items: InventoryItemDto[] = [
      { item_id: "wheat", name: "Wheat", rarity: "Common", quantity: 64, npc_unit_price: "3.45" },
      { item_id: "wood", name: "Wood", rarity: "Common", quantity: 52, npc_unit_price: "3.15" },
      { item_id: "stone", name: "Stone", rarity: "Common", quantity: 36, npc_unit_price: "4.50" },
      { item_id: "silver_carp", name: "Silver Carp", rarity: "Uncommon", quantity: 9, npc_unit_price: "17.10" },
      { item_id: "iron_ore", name: "Iron Ore", rarity: "Uncommon", quantity: 14, npc_unit_price: "18.90" },
    ];
    return {
      me: {
        registered: true,
        discord_user_id: "123456789012345678",
        username: isTrade ? "MerchantMayor" : "TownMayor",
        avatar_url: null,
        profile: {
          level: isTrade ? 21 : 14,
          xp: isTrade ? 1120 : 680,
          xp_needed: isTrade ? 2100 : 1400,
          balance: isTrade ? "1420.00" : "860.25",
          city_stage: stage,
          city_stage_name: stageName,
          lang: "en",
          slots_used: items.length,
          slot_capacity: isTrade ? 28 : 24,
        },
      },
      city: {
        stage,
        stage_name: stageName,
        era,
        treasury: isTrade ? "740.00" : "410.00",
        wallet_balance: isTrade ? "1420.00" : "860.25",
        tax_rate_pct: 10,
        welfare: isTrade ? 98 : 96,
        effectiveness_pct: isTrade ? 113 : 112,
        yield_bonus_pct: isTrade ? 18 : 14,
        est_tax_income: isTrade ? "185.00" : "112.00",
        est_expense: isTrade ? "48.00" : "34.00",
        assigned_crews: 2,
        facilities: [
          { id: "well", name: "Well", level: 1 },
          { id: "granary", name: "Granary", level: isTrade ? 4 : 3 },
          { id: "town_hall", name: "Village Hall", level: 1 },
          { id: "market_hall", name: "Market Hall", level: 1 },
          ...(isTrade
            ? ([
                { id: "city_hall", name: "Town Hall", level: 1 },
                { id: "trade_port", name: "Trade Port", level: 1 },
              ] as const)
            : ([] as const)),
        ],
        running_project: null,
        server_time: nowIso,
      },
      inventory: {
        slots_used: items.length,
        slot_capacity: isTrade ? 28 : 24,
        items,
        pending: [],
      },
      activities: {
        stage,
        stage_name: stageName,
        slots_total: 4,
        runs: [
          { id: 41, kind: "mining", name: "Mining", status: "running", ends_at: isoAfter(22) },
          { id: 42, kind: "fishing", name: "Fishing", status: "running", ends_at: isoAfter(18) },
        ],
        progress: [
          { kind: "farming", name: "Farming", level: isTrade ? 24 : 18, xp: 520, xp_needed: 1800, overflow: 0, cap: isTrade ? 40 : 30 },
          { kind: "gathering", name: "Gathering", level: isTrade ? 22 : 16, xp: 430, xp_needed: 1600, overflow: 0, cap: isTrade ? 40 : 30 },
          { kind: "fishing", name: "Fishing", level: isTrade ? 19 : 11, xp: 310, xp_needed: 1100, overflow: 0, cap: isTrade ? 40 : 30 },
          { kind: "mining", name: "Mining", level: isTrade ? 15 : 6, xp: 240, xp_needed: 600, overflow: 0, cap: isTrade ? 40 : 30 },
        ],
        catalog: [
          { kind: "farming", name: "Farming", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 35 },
          { kind: "gathering", name: "Gathering", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 25 },
          { kind: "fishing", name: "Fishing", min_stage: "desa", min_stage_name: "Village", unlocked: true, base_duration_secs: 20, xp_reward: 40 },
          { kind: "mining", name: "Mining", min_stage: "kota_kecil", min_stage_name: "Town", unlocked: true, base_duration_secs: 20, xp_reward: 55 },
        ],
        server_time: nowIso,
      },
      market: {
        wallet_balance: isTrade ? "1420.00" : "860.25",
        calendar: baseCalendar,
        prices,
        listings: [],
        max_listings: 5,
        fee_pct: 1,
        commission_pct: 3,
        server_time: nowIso,
      },
      crafting: {
        max_concurrent: isTrade ? 3 : 2,
        rep_level: isTrade ? 4 : 3,
        rep_xp: 60,
        rep_xp_needed: 180,
        rep_bonus_pct: isTrade ? 9 : 6,
        completed_orders: isTrade ? 9 : 6,
        auto_sell: false,
        running: [],
        orders: baseOrders,
        recipes: [],
        server_time: nowIso,
      },
      crew: {
        max_slots: 4,
        max_level: 10,
        recruit_cost: "100.00",
        members: [],
      },
      nextId: 100,
      usedKeys: new Set(),
    };
  }

  if (preset === "metropolis") {
    const items: InventoryItemDto[] = [
      { item_id: "wheat", name: "Wheat", rarity: "Common", quantity: 120, npc_unit_price: "3.45" },
      { item_id: "plank", name: "Wood Plank", rarity: "Common", quantity: 48, npc_unit_price: "10.10" },
      { item_id: "bread", name: "Village Bread", rarity: "Common", quantity: 18, npc_unit_price: "29.90" },
      { item_id: "golden_eel", name: "Golden Eel", rarity: "Rare", quantity: 5, npc_unit_price: "48.20" },
      { item_id: "amethyst", name: "Amethyst", rarity: "Rare", quantity: 9, npc_unit_price: "64.00" },
      { item_id: "star_crystal", name: "Star Crystal", rarity: "Epic", quantity: 2, npc_unit_price: "162.50" },
    ];
    return {
      me: {
        registered: true,
        discord_user_id: "123456789012345678",
        username: "MetroArchitect",
        avatar_url: null,
        voice_boost_active: true,
        voice_boost_speed_pct: 20,
        voice_boost_xp_pct: 15,
        profile: {
          level: 28,
          xp: 1650,
          xp_needed: 2800,
          balance: "2450.75",
          city_stage: "metropolis",
          city_stage_name: "Metropolis",
          lang: "en",
          slots_used: items.length,
          slot_capacity: 32,
        },
      },
      city: {
        stage: "metropolis",
        stage_name: "Metropolis",
        era: "Metropolis Era",
        treasury: "1380.00",
        wallet_balance: "2450.75",
        tax_rate_pct: 10,
        welfare: 100,
        effectiveness_pct: 115,
        yield_bonus_pct: 22,
        est_tax_income: "316.00",
        est_expense: "67.00",
        assigned_crews: 2,
        facilities: [
          { id: "well", name: "Well", level: 1 },
          { id: "granary", name: "Granary", level: 5 },
          { id: "town_hall", name: "Village Hall", level: 1 },
          { id: "market_hall", name: "Market Hall", level: 1 },
          { id: "city_hall", name: "Town Hall", level: 1 },
          { id: "trade_port", name: "Trade Port", level: 1 },
          { id: "metropolis_tower", name: "Metropolis Tower", level: 1 },
        ],
        running_project: null,
        server_time: nowIso,
      },
      inventory: { slots_used: items.length, slot_capacity: 32, items, pending: [] },
      activities: {
        stage: "metropolis",
        stage_name: "Metropolis",
        slots_total: 4,
        runs: [
          { id: 91, kind: "mining", name: "Mining", status: "running", ends_at: isoAfter(25) },
          { id: 92, kind: "fishing", name: "Fishing", status: "running", ends_at: isoAfter(20) },
          { id: 93, kind: "farming", name: "Farming", status: "ready", ends_at: isoAfter(-15) },
        ],
        progress: [
          { kind: "farming", name: "Farming", level: 34, xp: 1200, xp_needed: 3400, overflow: 0, cap: 50 },
          { kind: "gathering", name: "Gathering", level: 31, xp: 800, xp_needed: 3100, overflow: 0, cap: 50 },
          { kind: "fishing", name: "Fishing", level: 26, xp: 1900, xp_needed: 2600, overflow: 0, cap: 50 },
          { kind: "mining", name: "Mining", level: 22, xp: 450, xp_needed: 2200, overflow: 0, cap: 50 },
        ],
        catalog: [
          { kind: "farming", name: "Farming", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 35 },
          { kind: "gathering", name: "Gathering", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 25 },
          { kind: "fishing", name: "Fishing", min_stage: "desa", min_stage_name: "Village", unlocked: true, base_duration_secs: 20, xp_reward: 40 },
          { kind: "mining", name: "Mining", min_stage: "kota_kecil", min_stage_name: "Town", unlocked: true, base_duration_secs: 20, xp_reward: 55 },
        ],
        server_time: nowIso,
      },
      market: {
        wallet_balance: "2450.75",
        calendar: {
          day: 11,
          season: "summer",
          season_name: "Summer",
          season_hint: "+12% Fish",
          active_event: "mining_boom",
          active_event_name: "Mining Boom",
          event_hint: "+20% Mining Yield",
          trend_item_id: "amethyst",
          trend_item_name: "Amethyst",
        },
        prices,
        listings: [],
        max_listings: 5,
        fee_pct: 1,
        commission_pct: 3,
        server_time: nowIso,
      },
      crafting: {
        max_concurrent: 3,
        rep_level: 5,
        rep_xp: 90,
        rep_xp_needed: 250,
        rep_bonus_pct: 12,
        completed_orders: 14,
        auto_sell: false,
        running: [],
        orders: baseOrders,
        recipes: [],
        server_time: nowIso,
      },
      crew: {
        max_slots: 4,
        max_level: 10,
        recruit_cost: "100.00",
        members: [
          {
            id: 1,
            name: "Budi",
            rarity: "Rare",
            specialty: "mining",
            specialty_name: "Mining",
            level: 4,
            assigned: "mining",
            assigned_name: "Mining",
            auto_enabled: true,
            efficiency_pct: 24,
            yield_pct: 24,
            luck_pct: 24,
            upgrade_cost: "600.00",
          },
          {
            id: 2,
            name: "Laras",
            rarity: "Epic",
            specialty: "fishing",
            specialty_name: "Fishing",
            level: 3,
            assigned: "fishing",
            assigned_name: "Fishing",
            auto_enabled: false,
            efficiency_pct: 28,
            yield_pct: 28,
            luck_pct: 28,
            upgrade_cost: "600.00",
          },
        ],
      },
      nextId: 200,
      usedKeys: new Set(),
    };
  }

  // Default: "kampung"
  const items: InventoryItemDto[] = [
    { item_id: "wheat", name: "Wheat", rarity: "Common", quantity: 14, npc_unit_price: "3.12" },
    { item_id: "potato", name: "Potato", rarity: "Common", quantity: 8, npc_unit_price: "4.10" },
    { item_id: "wood", name: "Wood", rarity: "Common", quantity: 19, npc_unit_price: "2.95" },
    { item_id: "wild_herb", name: "Wild Herb", rarity: "Uncommon", quantity: 3, npc_unit_price: "12.40" },
  ];
  return {
    me: {
      registered: true,
      discord_user_id: "123456789012345678",
      username: "CitizenDev",
      avatar_url: null,
      profile: {
        level: 4,
        xp: 230,
        xp_needed: 400,
        balance: "142.50",
        city_stage: "kampung",
        city_stage_name: "Hamlet",
        lang: "en",
        slots_used: items.length,
        slot_capacity: 12,
      },
    },
    city: {
      stage: "kampung",
      stage_name: "Hamlet",
      era: "Village Era",
      treasury: "45.00",
      wallet_balance: "142.50",
      tax_rate_pct: 10,
      welfare: 88,
      effectiveness_pct: 107,
      yield_bonus_pct: 3,
      est_tax_income: "20.24",
      est_expense: "5.00",
      assigned_crews: 0,
      facilities: [{ id: "well", name: "Well", level: 1 }],
      running_project: {
        id: 7,
        project: "granary",
        name: "Granary",
        ends_at: isoAfter(25),
      },
      server_time: nowIso,
    },
    inventory: {
      slots_used: items.length,
      slot_capacity: 12,
      items,
      pending: [{ item_id: "berries", name: "Forest Berries", rarity: "Common", quantity: 6 }],
    },
    activities: {
      stage: "kampung",
      stage_name: "Hamlet",
      slots_total: 2,
      runs: [
        { id: 11, kind: "farming", name: "Farming", status: "running", ends_at: isoAfter(20) },
        { id: 12, kind: "gathering", name: "Gathering", status: "ready", ends_at: isoAfter(-10) },
      ],
      progress: [
        { kind: "farming", name: "Farming", level: 3, xp: 140, xp_needed: 300, overflow: 0, cap: 10 },
        { kind: "gathering", name: "Gathering", level: 10, xp: 0, xp_needed: 1000, overflow: 75, cap: 10 },
      ],
      catalog: [
        { kind: "farming", name: "Farming", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 35 },
        { kind: "gathering", name: "Gathering", min_stage: "kampung", min_stage_name: "Hamlet", unlocked: true, base_duration_secs: 20, xp_reward: 25 },
        { kind: "fishing", name: "Fishing", min_stage: "desa", min_stage_name: "Village", unlocked: false, base_duration_secs: 20, xp_reward: 40 },
        { kind: "mining", name: "Mining", min_stage: "kota_kecil", min_stage_name: "Town", unlocked: false, base_duration_secs: 20, xp_reward: 55 },
      ],
      server_time: nowIso,
    },
    market: {
      wallet_balance: "142.50",
      calendar: baseCalendar,
      prices,
      listings: [],
      max_listings: 5,
      fee_pct: 1,
      commission_pct: 3,
      server_time: nowIso,
    },
    crafting: {
      max_concurrent: 2,
      rep_level: 1,
      rep_xp: 0,
      rep_xp_needed: 50,
      rep_bonus_pct: 0,
      completed_orders: 0,
      auto_sell: false,
      running: [],
      orders: baseOrders,
      recipes: [],
      server_time: nowIso,
    },
    crew: {
      max_slots: 4,
      max_level: 10,
      recruit_cost: "100.00",
      members: [],
    },
    nextId: 100,
    usedKeys: new Set(),
  };
}

const mockStore: Partial<Record<MockPreset, MockState>> = {};

function getState(preset: MockPreset): MockState {
  if (!mockStore[preset]) {
    mockStore[preset] = createInitialState(preset);
  }
  const st = mockStore[preset]!;
  reconcileState(st);
  return st;
}

function facilityMap(city: CityResponse): Record<ProjectKey, number> {
  const map: Record<ProjectKey, number> = {
    well: 0,
    granary: 0,
    town_hall: 0,
    market_hall: 0,
    city_hall: 0,
    trade_port: 0,
    metropolis_tower: 0,
  };
  for (const f of city.facilities) {
    map[f.id] = f.level;
  }
  return map;
}

function grantItem(st: MockState, itemId: string, qty: number): { stored: number; pending: number } {
  const meta = ITEM_META[itemId] ?? {
    name: itemId,
    rarity: "Common",
    basePrice: 3,
    npcPrice: "3.00",
  };
  const inv = st.inventory!;
  const existing = inv.items.find((it) => it.item_id === itemId);
  if (existing) {
    existing.quantity += qty;
    return { stored: qty, pending: 0 };
  }
  if (inv.items.length < inv.slot_capacity) {
    inv.items.push({
      item_id: itemId,
      name: meta.name,
      rarity: meta.rarity,
      quantity: qty,
      npc_unit_price: meta.npcPrice,
    });
    return { stored: qty, pending: 0 };
  }
  const pend = inv.pending.find((p) => p.item_id === itemId);
  if (pend) pend.quantity += qty;
  else inv.pending.push({ item_id: itemId, name: meta.name, rarity: meta.rarity, quantity: qty });
  return { stored: 0, pending: qty };
}

function reconcileState(st: MockState): void {
  const now = Date.now();
  const nowIso = new Date(now).toISOString();

  if (st.activities) {
    st.activities.server_time = nowIso;
    for (const r of st.activities.runs) {
      if (r.status === "running" && Date.parse(r.ends_at) <= now) {
        r.status = "ready";
      }
    }
  }

  if (st.crafting && st.inventory && st.city) {
    st.crafting.server_time = nowIso;
    const stillRunning = [];
    for (const cr of st.crafting.running) {
      if (Date.parse(cr.ends_at) <= now) {
        const recipe = RECIPE_CATALOG.find((x) => x.id === cr.recipe_id);
        if (recipe) {
          if (cr.auto_sell) {
            const unit = Number(ITEM_META[recipe.output_item_id]?.npcPrice ?? "5.00");
            st.city.wallet_balance = (
              Number(st.city.wallet_balance) + unit * cr.quantity
            ).toFixed(2);
          } else {
            grantItem(st, recipe.output_item_id, cr.quantity);
          }
        }
      } else {
        stillRunning.push(cr);
      }
    }
    st.crafting.running = stillRunning;
  }

  if (st.city) {
    st.city.server_time = nowIso;
    if (st.city.running_project && Date.parse(st.city.running_project.ends_at) <= now) {
      const done = st.city.running_project;
      const existing = st.city.facilities.find((f) => f.id === done.project);
      if (existing) existing.level += 1;
      else st.city.facilities.push({ id: done.project, name: done.name, level: 1 });
      st.city.running_project = null;

      const stageAdvance: Partial<Record<ProjectKey, CityStageKey>> = {
        town_hall: "desa",
        city_hall: "kota_kecil",
        trade_port: "kota_dagang",
        metropolis_tower: "metropolis",
      };
      const nextStage = stageAdvance[done.project];
      if (nextStage) {
        st.city.stage = nextStage;
        const meta = STAGE_META[nextStage];
        st.city.stage_name = meta.name;
        st.city.era = meta.era;
        if (st.me.profile) {
          st.me.profile.city_stage = nextStage;
          st.me.profile.city_stage_name = meta.name;
        }
        if (st.activities) {
          st.activities.stage = nextStage;
          st.activities.stage_name = meta.name;
          st.activities.slots_total = meta.slots;
          for (const c of st.activities.catalog) {
            c.unlocked = stageGte(nextStage, c.min_stage);
            if (c.unlocked && !st.activities.progress.some((p) => p.kind === c.kind)) {
              st.activities.progress.push({
                kind: c.kind,
                name: c.name,
                level: 1,
                xp: 0,
                xp_needed: 100,
                overflow: 0,
                cap: meta.cap,
              });
            }
          }
          for (const p of st.activities.progress) p.cap = meta.cap;
        }
      }
    }
  }

  if (st.inventory && st.city) {
    const granaryLvl = st.city.facilities.find((f) => f.id === "granary")?.level ?? 0;
    const cap = 12 + granaryLvl * 4;
    st.inventory.slot_capacity = cap;
    st.inventory.slots_used = st.inventory.items.length;
    if (st.me.profile) {
      st.me.profile.slot_capacity = cap;
      st.me.profile.slots_used = st.inventory.items.length;
      st.me.profile.balance = st.city.wallet_balance;
    }
    if (st.market) {
      st.market.wallet_balance = st.city.wallet_balance;
      st.market.server_time = nowIso;
    }
    if (st.crafting) {
      const stock = Object.fromEntries(st.inventory.items.map((i) => [i.item_id, i.quantity]));
      st.crafting.recipes = RECIPE_CATALOG.map((r) => {
        const inputs = r.inputs.map((inp) => ({
          ...inp,
          have: stock[inp.item_id] ?? 0,
        }));
        const max_batches = Math.min(
          1000,
          ...inputs.map((i) => Math.floor(i.have / Math.max(1, i.need))),
        );
        return { ...r, inputs, max_batches };
      });
      for (const ord of st.crafting.orders) {
        ord.have = stock[ord.item_id] ?? 0;
        ord.can_fulfill = ord.have >= ord.quantity;
      }
      const hasPort = st.city.facilities.some((f) => f.id === "trade_port");
      st.crafting.max_concurrent = hasPort ? 3 : 2;
    }
    if (st.crew) {
      st.city.assigned_crews = st.crew.members.filter((m) => m.assigned !== null).length;
    }
  }
}

export function resetMockPreset(preset: MockPreset): void {
  mockStore[preset] = createInitialState(preset);
}

export function getMockSnapshot(preset: MockPreset): CitySnapshot {
  const st = getState(preset);
  if (!st.me.registered || !st.city || !st.inventory || !st.activities) {
    return {
      me: structuredClone(st.me),
      city: null,
      quotes: null,
      inventory: null,
      activities: null,
      market: null,
      crafting: null,
      crew: null,
      clockOffsetMs: 0,
    };
  }

  const stock = Object.fromEntries(st.inventory.items.map((i) => [i.item_id, i.quantity]));
  const availBac = Number(st.city.treasury) + Number(st.city.wallet_balance);
  const quotes = buildQuotes(
    st.city.stage,
    facilityMap(st.city),
    st.city.running_project?.project ?? null,
    availBac,
    stock,
  );

  return {
    me: structuredClone(st.me),
    city: structuredClone(st.city),
    quotes,
    inventory: structuredClone(st.inventory),
    activities: structuredClone(st.activities),
    market: structuredClone(st.market),
    crafting: structuredClone(st.crafting),
    crew: structuredClone(st.crew),
    clockOffsetMs: 0,
  };
}

export function mockRegister(preset: MockPreset): RegisterResponse {
  const st = getState(preset);
  if (st.me.registered) {
    return { created: false, discord_user_id: st.me.discord_user_id };
  }
  const fresh = createInitialState("kampung");
  fresh.me.username = st.me.username;
  fresh.me.profile!.balance = "100.00";
  fresh.city!.treasury = "0.00";
  fresh.city!.wallet_balance = "100.00";
  fresh.city!.facilities = [];
  fresh.city!.running_project = null;
  fresh.activities!.runs = [];
  fresh.inventory!.items = [];
  fresh.inventory!.pending = [];
  reconcileState(fresh);
  mockStore[preset] = fresh;
  return { created: true, discord_user_id: fresh.me.discord_user_id };
}

export function mockStartActivity(preset: MockPreset, kind: ActivityKindKey): ActivityRunDto {
  const st = getState(preset);
  if (!st.activities) throw new Error("Profile not found.");
  const cat = st.activities.catalog.find((c) => c.kind === kind);
  if (!cat || !cat.unlocked) {
    throw new Error(`This activity unlocks at ${cat?.min_stage_name ?? "-"} stage.`);
  }
  if (st.activities.runs.length >= st.activities.slots_total) {
    throw new Error(`All Activity Slots are in use (${st.activities.slots_total} slots).`);
  }
  const run: ActivityRunDto = {
    id: st.nextId++,
    kind,
    name: cat.name,
    status: "running",
    ends_at: isoAfter(20),
  };
  st.activities.runs.push(run);
  return structuredClone(run);
}

export function mockCollectActivities(preset: MockPreset): CollectActivitiesResponse {
  const st = getState(preset);
  if (!st.activities || !st.inventory || !st.me.profile) {
    throw new Error("Profile not found.");
  }
  const ready = st.activities.runs.filter((r) => r.status === "ready");
  st.activities.runs = st.activities.runs.filter((r) => r.status !== "ready");

  const collected = ready.map((r) => {
    const drops = ACTIVITY_DROPS[r.kind];
    const items = drops.map((d) => {
      const meta = ITEM_META[d.id] ?? { name: d.id, rarity: "Common", basePrice: 3, npcPrice: "3.00" };
      const { stored, pending } = grantItem(st, d.id, d.qty);
      return { item_id: d.id, name: meta.name, stored, pending };
    });

    const cat = st.activities!.catalog.find((c) => c.kind === r.kind);
    const xpGain = cat?.xp_reward ?? 25;
    let prog = st.activities!.progress.find((p) => p.kind === r.kind);
    if (!prog) {
      prog = {
        kind: r.kind,
        name: r.name,
        level: 1,
        xp: 0,
        xp_needed: 100,
        overflow: 0,
        cap: STAGE_META[st.activities!.stage].cap,
      };
      st.activities!.progress.push(prog);
    }
    const oldLevel = prog.level;
    if (prog.level >= prog.cap) {
      prog.overflow += xpGain;
    } else {
      prog.xp += xpGain;
      while (prog.level < prog.cap && prog.xp >= prog.xp_needed) {
        prog.xp -= prog.xp_needed;
        prog.level += 1;
        prog.xp_needed = 100 * prog.level;
      }
    }

    st.me.profile!.xp += xpGain;
    while (st.me.profile!.xp >= st.me.profile!.xp_needed) {
      st.me.profile!.xp -= st.me.profile!.xp_needed;
      st.me.profile!.level += 1;
      st.me.profile!.xp_needed = 100 * st.me.profile!.level;
    }

    return {
      kind: r.kind,
      name: r.name,
      xp_gain: xpGain,
      old_level: oldLevel,
      new_level: prog.level,
      items,
    };
  });

  reconcileState(st);
  return { collected };
}

export function mockSellToNpc(
  preset: MockPreset,
  itemId: string,
  quantity: number | undefined,
  sellAll: boolean | undefined,
  requestId: string,
): MarketSellResponse {
  const st = getState(preset);
  if (!st.inventory || !st.city) throw new Error("Profile not found.");
  const key = `sell:${requestId}`;
  const item = st.inventory.items.find((i) => i.item_id === itemId);
  if (st.usedKeys.has(key)) {
    return {
      duplicate: true,
      item_id: itemId,
      name: item?.name ?? ITEM_META[itemId]?.name ?? itemId,
      qty: 0,
      total_bac: "0.00",
      wallet_balance: st.city.wallet_balance,
    };
  }
  if (!item) throw new Error("Insufficient item quantity (you have 0).");
  const qty = sellAll ? item.quantity : (quantity ?? 0);
  if (qty <= 0 || qty > item.quantity) {
    throw new Error(`Insufficient item quantity (you have ${item.quantity}).`);
  }

  st.usedKeys.add(key);
  const total = Number(item.npc_unit_price) * qty;
  item.quantity -= qty;
  if (item.quantity <= 0) {
    st.inventory.items = st.inventory.items.filter((i) => i.item_id !== itemId);
  }
  st.city.wallet_balance = (Number(st.city.wallet_balance) + total).toFixed(2);
  reconcileState(st);

  return {
    duplicate: false,
    item_id: item.item_id,
    name: item.name,
    qty,
    total_bac: total.toFixed(2),
    wallet_balance: st.city.wallet_balance,
  };
}

export function mockClaimPending(preset: MockPreset): ClaimPendingResponse {
  const st = getState(preset);
  if (!st.inventory) throw new Error("Profile not found.");
  const claimed = [];
  const nextPending = [];
  for (const p of st.inventory.pending) {
    const res = grantItem(st, p.item_id, p.quantity);
    if (res.pending > 0) {
      nextPending.push({ ...p, quantity: res.pending });
    }
    claimed.push({
      item_id: p.item_id,
      name: p.name,
      claimed: res.stored,
      remaining: res.pending,
    });
  }
  st.inventory.pending = nextPending;
  reconcileState(st);
  return { claimed };
}

export function mockStartProject(
  preset: MockPreset,
  facility: ProjectKey,
  requestId: string,
): RunningProjectDto {
  const st = getState(preset);
  if (!st.city || !st.inventory) throw new Error("Profile not found.");
  const key = `proj:${requestId}`;
  if (st.usedKeys.has(key) && st.city.running_project) {
    return structuredClone(st.city.running_project);
  }
  const stock = Object.fromEntries(st.inventory.items.map((i) => [i.item_id, i.quantity]));
  const availBac = Number(st.city.treasury) + Number(st.city.wallet_balance);
  const quote = buildQuotes(
    st.city.stage,
    facilityMap(st.city),
    st.city.running_project?.project ?? null,
    availBac,
    stock,
  ).find((q) => q.project === facility);

  if (!quote) throw new Error("Unknown facility.");
  if (!quote.can_start) throw new Error(quote.block_reason ?? "Project cannot be started.");

  st.usedKeys.add(key);
  const cost = Number(quote.cost_bac);
  const treasury = Number(st.city.treasury);
  const fromTreasury = Math.min(treasury, cost);
  const fromWallet = cost - fromTreasury;
  st.city.treasury = (treasury - fromTreasury).toFixed(2);
  st.city.wallet_balance = (Number(st.city.wallet_balance) - fromWallet).toFixed(2);

  for (const reqItem of quote.items) {
    const line = st.inventory.items.find((i) => i.item_id === reqItem.item_id);
    if (line) line.quantity -= reqItem.need;
  }
  st.inventory.items = st.inventory.items.filter((i) => i.quantity > 0);

  const rp: RunningProjectDto = {
    id: st.nextId++,
    project: facility,
    name: quote.name,
    ends_at: isoAfter(quote.duration_secs),
  };
  st.city.running_project = rp;
  reconcileState(st);
  return structuredClone(rp);
}

export function mockTreasuryTransfer(
  preset: MockPreset,
  dir: "deposit" | "withdraw",
  amountStr: string,
  requestId: string,
): TreasuryBalanceResponse {
  const st = getState(preset);
  if (!st.city) throw new Error("Profile not found.");
  const amount = Number(amountStr);
  if (!Number.isFinite(amount) || amount <= 0) {
    throw new Error("Amount must be greater than 0.");
  }
  const key = `${dir}:${requestId}`;
  if (st.usedKeys.has(key)) {
    return { wallet_balance: st.city.wallet_balance, treasury: st.city.treasury };
  }
  const w = Number(st.city.wallet_balance);
  const t = Number(st.city.treasury);
  if (dir === "deposit") {
    if (w < amount) throw new Error("Insufficient Wallet Bac balance.");
    st.city.wallet_balance = (w - amount).toFixed(2);
    st.city.treasury = (t + amount).toFixed(2);
  } else {
    if (t < amount) throw new Error("Insufficient City Treasury balance.");
    st.city.treasury = (t - amount).toFixed(2);
    st.city.wallet_balance = (w + amount).toFixed(2);
  }
  st.usedKeys.add(key);
  reconcileState(st);
  return { wallet_balance: st.city.wallet_balance, treasury: st.city.treasury };
}

export function mockSetTaxRate(preset: MockPreset, taxRatePct: number): SetTaxResponse {
  const st = getState(preset);
  if (!st.city) throw new Error("Profile not found.");
  if (taxRatePct < 0 || taxRatePct > 25) throw new Error("Tax rate must be 0%–25%.");
  st.city.tax_rate_pct = taxRatePct;
  return { tax_rate_pct: taxRatePct };
}

export function mockCreateListing(
  preset: MockPreset,
  params: {
    itemId: string;
    quantity: number;
    targetUnitPrice: string;
    duration: "2h" | "6h" | "24h";
    requestId: string;
  },
): BrokerListingDto {
  const st = getState(preset);
  if (!st.market || !st.inventory || !st.city) throw new Error("Profile not found.");
  if (st.market.listings.length >= st.market.max_listings) {
    throw new Error(`Maximum of ${st.market.max_listings} active broker listings reached.`);
  }
  const price = Number(params.targetUnitPrice);
  if (!Number.isFinite(price) || price <= 0) throw new Error("Target price must be > 0.");
  const item = st.inventory.items.find((i) => i.item_id === params.itemId);
  if (!item || item.quantity < params.quantity || params.quantity <= 0) {
    throw new Error("Insufficient item quantity.");
  }
  const fee = Math.max(0.5, Number((price * params.quantity * 0.01).toFixed(2)));
  const bal = Number(st.city.wallet_balance);
  if (bal < fee) throw new Error(`Insufficient Bac balance for listing fee (${fee.toFixed(2)} Bac).`);

  st.city.wallet_balance = (bal - fee).toFixed(2);
  item.quantity -= params.quantity;
  if (item.quantity <= 0) {
    st.inventory.items = st.inventory.items.filter((i) => i.item_id !== params.itemId);
  }
  const secs = params.duration === "2h" ? 7200 : params.duration === "6h" ? 21600 : 86400;
  const listing: BrokerListingDto = {
    id: st.nextId++,
    item_id: item.item_id,
    name: item.name,
    quantity: params.quantity,
    target_unit_price: price.toFixed(2),
    listing_fee: fee.toFixed(2),
    expires_at: isoAfter(secs),
  };
  st.market.listings.push(listing);
  reconcileState(st);
  return structuredClone(listing);
}

export function mockCancelListing(
  preset: MockPreset,
  listingId: number,
): CancelListingResponse {
  const st = getState(preset);
  if (!st.market || !st.inventory) throw new Error("Profile not found.");
  const idx = st.market.listings.findIndex((l) => l.id === listingId);
  if (idx === -1) throw new Error("Listing not found.");
  const [removed] = st.market.listings.splice(idx, 1);
  const { stored, pending } = grantItem(st, removed.item_id, removed.quantity);
  reconcileState(st);
  return {
    item_id: removed.item_id,
    name: removed.name,
    stored,
    pending,
  };
}

export function mockStartCraft(
  preset: MockPreset,
  recipeId: string,
  batchQty: number,
): CraftRunDto {
  const st = getState(preset);
  if (!st.crafting || !st.inventory) throw new Error("Profile not found.");
  const recipe = RECIPE_CATALOG.find((r) => r.id === recipeId);
  if (!recipe) throw new Error("Unknown recipe.");
  if (st.crafting.running.length >= st.crafting.max_concurrent) {
    throw new Error(`Maximum of ${st.crafting.max_concurrent} concurrent crafts reached.`);
  }
  const qty = Math.max(1, Math.floor(batchQty));
  for (const inp of recipe.inputs) {
    const have = st.inventory.items.find((i) => i.item_id === inp.item_id)?.quantity ?? 0;
    if (have < inp.need * qty) {
      throw new Error(`Insufficient ${inp.name} (need ${inp.need * qty}, have ${have}).`);
    }
  }
  for (const inp of recipe.inputs) {
    const line = st.inventory.items.find((i) => i.item_id === inp.item_id)!;
    line.quantity -= inp.need * qty;
  }
  st.inventory.items = st.inventory.items.filter((i) => i.quantity > 0);
  const run: CraftRunDto = {
    id: st.nextId++,
    recipe_id: recipe.id,
    name: recipe.name,
    quantity: recipe.output_qty * qty,
    auto_sell: st.crafting.auto_sell,
    ends_at: isoAfter(recipe.duration_secs),
  };
  st.crafting.running.push(run);
  reconcileState(st);
  return structuredClone(run);
}

export function mockToggleCraftAutoSell(preset: MockPreset): { auto_sell: boolean } {
  const st = getState(preset);
  if (!st.crafting) throw new Error("Profile not found.");
  st.crafting.auto_sell = !st.crafting.auto_sell;
  return { auto_sell: st.crafting.auto_sell };
}

export function mockFulfillOrder(
  preset: MockPreset,
  orderId: number,
): FulfillOrderResponse {
  const st = getState(preset);
  if (!st.crafting || !st.inventory || !st.city) throw new Error("Profile not found.");
  const idx = st.crafting.orders.findIndex((o) => o.id === orderId);
  if (idx === -1) throw new Error("Order not found.");
  const order = st.crafting.orders[idx];
  const line = st.inventory.items.find((i) => i.item_id === order.item_id);
  if (!line || line.quantity < order.quantity) {
    throw new Error(`Not enough ${order.name} to fulfill order.`);
  }
  line.quantity -= order.quantity;
  st.inventory.items = st.inventory.items.filter((i) => i.quantity > 0);
  st.city.wallet_balance = (
    Number(st.city.wallet_balance) + Number(order.reward_bac)
  ).toFixed(2);
  const oldLvl = st.crafting.rep_level;
  st.crafting.rep_xp += order.rep_xp;
  while (st.crafting.rep_xp >= st.crafting.rep_xp_needed) {
    st.crafting.rep_xp -= st.crafting.rep_xp_needed;
    st.crafting.rep_level += 1;
    st.crafting.rep_xp_needed = 50 * st.crafting.rep_level;
    st.crafting.rep_bonus_pct = (st.crafting.rep_level - 1) * 3;
  }
  st.crafting.completed_orders += 1;
  // Replace completed order
  const nextRec = RECIPE_CATALOG[(st.nextId++) % RECIPE_CATALOG.length];
  st.crafting.orders[idx] = {
    id: st.nextId++,
    item_id: nextRec.output_item_id,
    name: nextRec.name,
    quantity: 1 + (st.nextId % 2),
    have: 0,
    reward_bac: (Number(ITEM_META[nextRec.output_item_id]?.basePrice ?? 10) * 1.35).toFixed(2),
    rep_xp: 20,
    can_fulfill: false,
  };
  reconcileState(st);
  return {
    item_id: order.item_id,
    name: order.name,
    quantity: order.quantity,
    reward_bac: order.reward_bac,
    rep_xp_gain: order.rep_xp,
    old_rep_level: oldLvl,
    new_rep_level: st.crafting.rep_level,
  };
}

export function mockRefreshOrders(preset: MockPreset): { ok: boolean } {
  const st = getState(preset);
  if (!st.crafting) throw new Error("Profile not found.");
  st.crafting.orders = [0, 1, 2].map((i) => {
    const r = RECIPE_CATALOG[(st.nextId + i) % RECIPE_CATALOG.length];
    const qty = 1 + ((st.nextId + i) % 3);
    const base = ITEM_META[r.output_item_id]?.basePrice ?? 10;
    return {
      id: st.nextId++,
      item_id: r.output_item_id,
      name: r.name,
      quantity: qty,
      have: 0,
      reward_bac: (base * qty * 1.35).toFixed(2),
      rep_xp: 15 * qty,
      can_fulfill: false,
    };
  });
  reconcileState(st);
  return { ok: true };
}

export function mockRecruitCrew(preset: MockPreset, requestId: string): CrewMemberDto {
  const st = getState(preset);
  if (!st.crew || !st.city) throw new Error("Profile not found.");
  if (st.crew.members.length >= st.crew.max_slots) {
    throw new Error(`Crew slots full (maximum ${st.crew.max_slots} crew).`);
  }
  const cost = Number(st.crew.recruit_cost);
  const bal = Number(st.city.wallet_balance);
  if (bal < cost) throw new Error("Insufficient Bac balance.");
  const key = `crec:${requestId}`;
  if (st.usedKeys.has(key) && st.crew.members.length > 0) {
    return structuredClone(st.crew.members[st.crew.members.length - 1]);
  }
  st.usedKeys.add(key);
  st.city.wallet_balance = (bal - cost).toFixed(2);
  const names = ["Budi", "Sari", "Raka", "Dewi", "Bagas", "Laras"];
  const unlocked = st.activities?.catalog.filter((c) => c.unlocked) ?? [];
  const spec = unlocked[st.nextId % Math.max(1, unlocked.length)]?.kind ?? "farming";
  const specName = unlocked.find((c) => c.kind === spec)?.name ?? "Farming";
  const member: CrewMemberDto = {
    id: st.nextId++,
    name: names[st.nextId % names.length],
    rarity: st.nextId % 4 === 0 ? "Rare" : "Common",
    specialty: spec,
    specialty_name: specName,
    level: 1,
    assigned: null,
    assigned_name: null,
    auto_enabled: false,
    efficiency_pct: 5,
    yield_pct: 5,
    luck_pct: 5,
    upgrade_cost: "50.00",
  };
  st.crew.members.push(member);
  reconcileState(st);
  return structuredClone(member);
}

export function mockUpgradeCrew(
  preset: MockPreset,
  crewId: number,
  requestId: string,
): CrewMemberDto {
  const st = getState(preset);
  if (!st.crew || !st.city) throw new Error("Profile not found.");
  const member = st.crew.members.find((m) => m.id === crewId);
  if (!member) throw new Error("Crew member not found.");
  if (member.level >= st.crew.max_level || !member.upgrade_cost) {
    throw new Error("Crew member is already at max level.");
  }
  const key = `cupg:${requestId}`;
  if (st.usedKeys.has(key)) return structuredClone(member);
  const cost = Number(member.upgrade_cost);
  const bal = Number(st.city.wallet_balance);
  if (bal < cost) throw new Error("Insufficient Bac balance.");
  st.usedKeys.add(key);
  st.city.wallet_balance = (bal - cost).toFixed(2);
  member.level += 1;
  member.efficiency_pct += 3;
  member.yield_pct += 3;
  member.luck_pct += 3;
  member.upgrade_cost =
    member.level >= st.crew.max_level ? null : (member.level * 50).toFixed(2);
  reconcileState(st);
  return structuredClone(member);
}

export function mockAssignCrew(
  preset: MockPreset,
  crewId: number,
  activity: ActivityKindKey | null,
  autoEnabled: boolean,
): CrewMemberDto {
  const st = getState(preset);
  if (!st.crew || !st.activities) throw new Error("Profile not found.");
  const member = st.crew.members.find((m) => m.id === crewId);
  if (!member) throw new Error("Crew member not found.");
  if (activity) {
    const cat = st.activities.catalog.find((c) => c.kind === activity);
    if (!cat || !cat.unlocked) throw new Error("Activity is not unlocked yet.");
    for (const m of st.crew.members) {
      if (m.assigned === activity) {
        m.assigned = null;
        m.assigned_name = null;
        m.auto_enabled = false;
      }
    }
    member.assigned = activity;
    member.assigned_name = cat.name;
    member.auto_enabled = autoEnabled;
  } else {
    member.assigned = null;
    member.assigned_name = null;
    member.auto_enabled = false;
  }
  reconcileState(st);
  return structuredClone(member);
}
