use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TokenExchangeRequest {
    pub code: String,
}

#[derive(Serialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct AppConfigResponse {
    pub discord_client_id: String,
}

#[derive(Serialize)]
pub struct MeProfileDto {
    pub level: i32,
    pub xp: i64,
    pub xp_needed: i64,
    pub balance: String,
    pub city_stage: &'static str,
    pub city_stage_name: &'static str,
    pub lang: &'static str,
    pub slots_used: i64,
    pub slot_capacity: i64,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub registered: bool,
    pub discord_user_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub profile: Option<MeProfileDto>,
}

#[derive(Serialize)]
pub struct FacilityDto {
    pub id: &'static str,
    pub name: &'static str,
    pub level: i32,
}

#[derive(Serialize)]
pub struct RunningProjectDto {
    pub id: i64,
    pub project: &'static str,
    pub name: &'static str,
    pub ends_at: String,
}

#[derive(Serialize)]
pub struct CityResponse {
    pub stage: &'static str,
    pub stage_name: &'static str,
    pub era: &'static str,
    pub treasury: String,
    pub wallet_balance: String,
    pub tax_rate_pct: i32,
    pub welfare: i32,
    pub effectiveness_pct: i32,
    pub yield_bonus_pct: i64,
    pub est_tax_income: String,
    pub est_expense: String,
    pub assigned_crews: i64,
    pub facilities: Vec<FacilityDto>,
    pub running_project: Option<RunningProjectDto>,
    pub server_time: String,
}

#[derive(Serialize)]
pub struct ProjectQuoteItemDto {
    pub item_id: &'static str,
    pub name: &'static str,
    pub need: i64,
    pub have: i64,
}

#[derive(Serialize)]
pub struct ProjectQuoteDto {
    pub project: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub current_level: i32,
    pub next_level: Option<i32>,
    pub max_level: i32,
    pub min_stage: &'static str,
    pub min_stage_name: &'static str,
    pub cost_bac: String,
    pub available_bac: String,
    pub duration_secs: i64,
    pub items: Vec<ProjectQuoteItemDto>,
    pub can_start: bool,
    pub block_reason: Option<String>,
}

#[derive(Serialize)]
pub struct ProjectQuoteResponse {
    pub projects: Vec<ProjectQuoteDto>,
}

#[derive(Serialize)]
pub struct InventoryItemDto {
    pub item_id: &'static str,
    pub name: &'static str,
    pub rarity: String,
    pub quantity: i64,
    pub npc_unit_price: String,
}

#[derive(Serialize)]
pub struct PendingItemDto {
    pub item_id: &'static str,
    pub name: &'static str,
    pub rarity: String,
    pub quantity: i64,
}

#[derive(Serialize)]
pub struct InventoryResponse {
    pub slots_used: i64,
    pub slot_capacity: i64,
    pub items: Vec<InventoryItemDto>,
    pub pending: Vec<PendingItemDto>,
}

#[derive(Serialize)]
pub struct ActivityRunDto {
    pub id: i64,
    pub kind: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub ends_at: String,
}

#[derive(Serialize)]
pub struct ActivityProgressDto {
    pub kind: &'static str,
    pub name: &'static str,
    pub level: i32,
    pub xp: i64,
    pub xp_needed: i64,
    pub overflow: i64,
    pub cap: i32,
}

#[derive(Serialize)]
pub struct ActivityCatalogDto {
    pub kind: &'static str,
    pub name: &'static str,
    pub min_stage: &'static str,
    pub min_stage_name: &'static str,
    pub unlocked: bool,
    pub base_duration_secs: i64,
    pub xp_reward: i64,
}

#[derive(Serialize)]
pub struct ActivitiesResponse {
    pub stage: &'static str,
    pub stage_name: &'static str,
    pub slots_total: i64,
    pub runs: Vec<ActivityRunDto>,
    pub progress: Vec<ActivityProgressDto>,
    pub catalog: Vec<ActivityCatalogDto>,
    pub server_time: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub created: bool,
    pub discord_user_id: String,
}

#[derive(Deserialize)]
pub struct StartActivityRequest {
    pub kind: String,
}

#[derive(Serialize)]
pub struct CollectedItemDto {
    pub item_id: &'static str,
    pub name: &'static str,
    pub stored: i64,
    pub pending: i64,
}

#[derive(Serialize)]
pub struct CollectedRunDto {
    pub kind: &'static str,
    pub name: &'static str,
    pub xp_gain: i64,
    pub old_level: i32,
    pub new_level: i32,
    pub items: Vec<CollectedItemDto>,
}

#[derive(Serialize)]
pub struct CollectActivitiesResponse {
    pub collected: Vec<CollectedRunDto>,
}

#[derive(Deserialize)]
pub struct MarketSellRequest {
    pub item_id: String,
    pub quantity: Option<i64>,
    pub sell_all: Option<bool>,
    pub request_id: String,
}

#[derive(Serialize)]
pub struct MarketSellResponse {
    pub duplicate: bool,
    pub item_id: &'static str,
    pub name: &'static str,
    pub qty: i64,
    pub total_bac: String,
    pub wallet_balance: String,
}

#[derive(Serialize)]
pub struct ClaimedPendingItemDto {
    pub item_id: String,
    pub name: &'static str,
    pub claimed: i64,
    pub remaining: i64,
}

#[derive(Serialize)]
pub struct ClaimPendingResponse {
    pub claimed: Vec<ClaimedPendingItemDto>,
}

#[derive(Deserialize)]
pub struct StartProjectRequest {
    pub facility: String,
    pub request_id: String,
}

#[derive(Deserialize)]
pub struct TreasuryTransferRequest {
    pub amount: String,
    pub request_id: String,
}

#[derive(Serialize)]
pub struct TreasuryBalanceResponse {
    pub wallet_balance: String,
    pub treasury: String,
}

#[derive(Deserialize)]
pub struct SetTaxRequest {
    pub tax_rate_pct: i32,
}

#[derive(Serialize)]
pub struct SetTaxResponse {
    pub tax_rate_pct: i32,
}

#[derive(Deserialize)]
pub struct SetLangRequest {
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct SetLangResponse {
    pub lang: &'static str,
    pub label: &'static str,
}

#[derive(Serialize)]
pub struct CalendarDto {
    pub day: i32,
    pub season: &'static str,
    pub season_name: &'static str,
    pub season_hint: &'static str,
    pub active_event: Option<&'static str>,
    pub active_event_name: Option<&'static str>,
    pub event_hint: Option<&'static str>,
    pub trend_item_id: Option<&'static str>,
    pub trend_item_name: Option<&'static str>,
}

#[derive(Serialize)]
pub struct MarketPriceDto {
    pub item_id: &'static str,
    pub name: &'static str,
    pub rarity: String,
    pub base_price: String,
    pub current_price: String,
    pub multiplier: String,
}

#[derive(Serialize)]
pub struct BrokerListingDto {
    pub id: i64,
    pub item_id: &'static str,
    pub name: &'static str,
    pub quantity: i64,
    pub target_unit_price: String,
    pub listing_fee: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub struct MarketResponse {
    pub wallet_balance: String,
    pub calendar: CalendarDto,
    pub prices: Vec<MarketPriceDto>,
    pub listings: Vec<BrokerListingDto>,
    pub max_listings: i64,
    pub fee_pct: i64,
    pub commission_pct: i64,
    pub server_time: String,
}

#[derive(Deserialize)]
pub struct CreateListingRequest {
    pub item_id: String,
    pub quantity: i64,
    pub target_unit_price: String,
    pub duration: String,
    pub request_id: String,
}

#[derive(Deserialize)]
pub struct CancelListingRequest {
    pub listing_id: i64,
}

#[derive(Serialize)]
pub struct CancelListingResponse {
    pub item_id: &'static str,
    pub name: &'static str,
    pub stored: i64,
    pub pending: i64,
}

#[derive(Serialize)]
pub struct RecipeInputDto {
    pub item_id: &'static str,
    pub name: &'static str,
    pub need: i64,
    pub have: i64,
}

#[derive(Serialize)]
pub struct RecipeDto {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: &'static str,
    pub tier_name: &'static str,
    pub output_item_id: &'static str,
    pub output_qty: i64,
    pub duration_secs: i64,
    pub xp_per_unit: i64,
    pub inputs: Vec<RecipeInputDto>,
    pub max_batches: i64,
}

#[derive(Serialize)]
pub struct CraftRunDto {
    pub id: i64,
    pub recipe_id: &'static str,
    pub name: &'static str,
    pub quantity: i64,
    pub auto_sell: bool,
    pub ends_at: String,
}

#[derive(Serialize)]
pub struct NpcOrderDto {
    pub id: i64,
    pub item_id: &'static str,
    pub name: &'static str,
    pub quantity: i64,
    pub have: i64,
    pub reward_bac: String,
    pub rep_xp: i64,
    pub can_fulfill: bool,
}

#[derive(Serialize)]
pub struct CraftingResponse {
    pub max_concurrent: i64,
    pub rep_level: i32,
    pub rep_xp: i64,
    pub rep_xp_needed: i64,
    pub rep_bonus_pct: i64,
    pub completed_orders: i64,
    pub auto_sell: bool,
    pub running: Vec<CraftRunDto>,
    pub orders: Vec<NpcOrderDto>,
    pub recipes: Vec<RecipeDto>,
    pub server_time: String,
}

#[derive(Deserialize)]
pub struct StartCraftRequest {
    pub recipe_id: String,
    pub batch_qty: Option<i64>,
}

#[derive(Serialize)]
pub struct ToggleAutoSellResponse {
    pub auto_sell: bool,
}

#[derive(Deserialize)]
pub struct FulfillOrderRequest {
    pub order_id: i64,
}

#[derive(Serialize)]
pub struct FulfillOrderResponse {
    pub item_id: &'static str,
    pub name: &'static str,
    pub quantity: i64,
    pub reward_bac: String,
    pub rep_xp_gain: i64,
    pub old_rep_level: i32,
    pub new_rep_level: i32,
}

#[derive(Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

#[derive(Serialize)]
pub struct CrewMemberDto {
    pub id: i64,
    pub name: String,
    pub rarity: String,
    pub specialty: &'static str,
    pub specialty_name: &'static str,
    pub level: i32,
    pub assigned: Option<&'static str>,
    pub assigned_name: Option<&'static str>,
    pub auto_enabled: bool,
    pub efficiency_pct: i64,
    pub yield_pct: i64,
    pub luck_pct: i64,
    pub upgrade_cost: Option<String>,
}

#[derive(Serialize)]
pub struct CrewResponse {
    pub max_slots: i64,
    pub max_level: i32,
    pub recruit_cost: String,
    pub members: Vec<CrewMemberDto>,
}

#[derive(Deserialize)]
pub struct RecruitCrewRequest {
    pub request_id: String,
}

#[derive(Deserialize)]
pub struct UpgradeCrewRequest {
    pub crew_id: i64,
    pub request_id: String,
}

#[derive(Deserialize)]
pub struct AssignCrewRequest {
    pub crew_id: i64,
    pub activity: Option<String>,
    pub auto_enabled: bool,
}
