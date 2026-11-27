export type CityStageKey =
  | "kampung"
  | "desa"
  | "kota_kecil"
  | "kota_dagang"
  | "metropolis";

export type ActivityKindKey = "farming" | "gathering" | "fishing" | "mining";

export type ProjectKey =
  | "well"
  | "granary"
  | "town_hall"
  | "market_hall"
  | "city_hall"
  | "trade_port"
  | "metropolis_tower";

export interface MeProfileDto {
  level: number;
  xp: number;
  xp_needed: number;
  balance: string;
  city_stage: CityStageKey;
  city_stage_name: string;
  lang: "en" | "id";
  slots_used: number;
  slot_capacity: number;
}

export interface MeResponse {
  registered: boolean;
  discord_user_id: string;
  username: string;
  avatar_url: string | null;
  profile: MeProfileDto | null;
}

export interface FacilityDto {
  id: ProjectKey;
  name: string;
  level: number;
}

export interface RunningProjectDto {
  id: number;
  project: ProjectKey;
  name: string;
  ends_at: string;
}

export interface CityResponse {
  stage: CityStageKey;
  stage_name: string;
  era: string;
  treasury: string;
  wallet_balance: string;
  tax_rate_pct: number;
  welfare: number;
  effectiveness_pct: number;
  yield_bonus_pct: number;
  est_tax_income: string;
  est_expense: string;
  assigned_crews: number;
  facilities: FacilityDto[];
  running_project: RunningProjectDto | null;
  server_time: string;
}

export interface ProjectQuoteItemDto {
  item_id: string;
  name: string;
  need: number;
  have: number;
}

export interface ProjectQuoteDto {
  project: ProjectKey;
  name: string;
  description: string;
  current_level: number;
  next_level: number | null;
  max_level: number;
  min_stage: CityStageKey;
  min_stage_name: string;
  cost_bac: string;
  available_bac: string;
  duration_secs: number;
  items: ProjectQuoteItemDto[];
  can_start: boolean;
  block_reason: string | null;
}

export interface ProjectQuoteResponse {
  projects: ProjectQuoteDto[];
}

export interface InventoryItemDto {
  item_id: string;
  name: string;
  rarity: string;
  quantity: number;
  npc_unit_price: string;
}

export interface PendingItemDto {
  item_id: string;
  name: string;
  rarity: string;
  quantity: number;
}

export interface InventoryResponse {
  slots_used: number;
  slot_capacity: number;
  items: InventoryItemDto[];
  pending: PendingItemDto[];
}

export interface ActivityRunDto {
  id: number;
  kind: ActivityKindKey;
  name: string;
  status: "running" | "ready";
  ends_at: string;
}

export interface ActivityProgressDto {
  kind: ActivityKindKey;
  name: string;
  level: number;
  xp: number;
  xp_needed: number;
  overflow: number;
  cap: number;
}

export interface ActivityCatalogDto {
  kind: ActivityKindKey;
  name: string;
  min_stage: CityStageKey;
  min_stage_name: string;
  unlocked: boolean;
  base_duration_secs: number;
  xp_reward: number;
}

export interface ActivitiesResponse {
  stage: CityStageKey;
  stage_name: string;
  slots_total: number;
  runs: ActivityRunDto[];
  progress: ActivityProgressDto[];
  catalog: ActivityCatalogDto[];
  server_time: string;
}

export interface CitySnapshot {
  me: MeResponse;
  city: CityResponse | null;
  quotes: ProjectQuoteDto[] | null;
  inventory: InventoryResponse | null;
  activities: ActivitiesResponse | null;
  market: MarketResponse | null;
  crafting: CraftingResponse | null;
  crew: CrewResponse | null;
  clockOffsetMs: number;
}

export type MockPreset = "kampung" | "desa" | "metropolis" | "unregistered";

export interface RegisterResponse {
  created: boolean;
  discord_user_id: string;
}

export interface SetLangResponse {
  lang: "en" | "id";
  label: string;
}

export interface CollectedItemDto {
  item_id: string;
  name: string;
  stored: number;
  pending: number;
}

export interface CollectedRunDto {
  kind: ActivityKindKey;
  name: string;
  xp_gain: number;
  old_level: number;
  new_level: number;
  items: CollectedItemDto[];
}

export interface CollectActivitiesResponse {
  collected: CollectedRunDto[];
}

export interface MarketSellResponse {
  duplicate: boolean;
  item_id: string;
  name: string;
  qty: number;
  total_bac: string;
  wallet_balance: string;
}

export interface ClaimedPendingItemDto {
  item_id: string;
  name: string;
  claimed: number;
  remaining: number;
}

export interface ClaimPendingResponse {
  claimed: ClaimedPendingItemDto[];
}

export interface TreasuryBalanceResponse {
  wallet_balance: string;
  treasury: string;
}

export interface SetTaxResponse {
  tax_rate_pct: number;
}

export interface CalendarDto {
  day: number;
  season: string;
  season_name: string;
  season_hint: string;
  active_event: string | null;
  active_event_name: string | null;
  event_hint: string | null;
  trend_item_id: string | null;
  trend_item_name: string | null;
}

export interface MarketPriceDto {
  item_id: string;
  name: string;
  rarity: string;
  base_price: string;
  current_price: string;
  multiplier: string;
}

export interface BrokerListingDto {
  id: number;
  item_id: string;
  name: string;
  quantity: number;
  target_unit_price: string;
  listing_fee: string;
  expires_at: string;
}

export interface MarketResponse {
  wallet_balance: string;
  calendar: CalendarDto;
  prices: MarketPriceDto[];
  listings: BrokerListingDto[];
  max_listings: number;
  fee_pct: number;
  commission_pct: number;
  server_time: string;
}

export interface CancelListingResponse {
  item_id: string;
  name: string;
  stored: number;
  pending: number;
}

export interface RecipeInputDto {
  item_id: string;
  name: string;
  need: number;
  have: number;
}

export interface RecipeDto {
  id: string;
  name: string;
  tier: "processed" | "finished";
  tier_name: string;
  output_item_id: string;
  output_qty: number;
  duration_secs: number;
  xp_per_unit: number;
  inputs: RecipeInputDto[];
  max_batches: number;
}

export interface CraftRunDto {
  id: number;
  recipe_id: string;
  name: string;
  quantity: number;
  auto_sell: boolean;
  ends_at: string;
}

export interface NpcOrderDto {
  id: number;
  item_id: string;
  name: string;
  quantity: number;
  have: number;
  reward_bac: string;
  rep_xp: number;
  can_fulfill: boolean;
}

export interface CraftingResponse {
  max_concurrent: number;
  rep_level: number;
  rep_xp: number;
  rep_xp_needed: number;
  rep_bonus_pct: number;
  completed_orders: number;
  auto_sell: boolean;
  running: CraftRunDto[];
  orders: NpcOrderDto[];
  recipes: RecipeDto[];
  server_time: string;
}

export interface FulfillOrderResponse {
  item_id: string;
  name: string;
  quantity: number;
  reward_bac: string;
  rep_xp_gain: number;
  old_rep_level: number;
  new_rep_level: number;
}

export interface CrewMemberDto {
  id: number;
  name: string;
  rarity: string;
  specialty: ActivityKindKey;
  specialty_name: string;
  level: number;
  assigned: ActivityKindKey | null;
  assigned_name: string | null;
  auto_enabled: boolean;
  efficiency_pct: number;
  yield_pct: number;
  luck_pct: number;
  upgrade_cost: string | null;
}

export interface CrewResponse {
  max_slots: number;
  max_level: number;
  recruit_cost: string;
  members: CrewMemberDto[];
}

export interface RealtimeEvent {
  kind: string;
  message: string | null;
}
