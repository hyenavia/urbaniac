import type { AuthSession } from "./discordAuth";
import {
  getMockSnapshot,
  mockAssignCrew,
  mockCancelListing,
  mockClaimPending,
  mockCollectActivities,
  mockCreateListing,
  mockFulfillOrder,
  mockRecruitCrew,
  mockRefreshOrders,
  mockRegister,
  mockSellToNpc,
  mockSetTaxRate,
  mockStartActivity,
  mockStartCraft,
  mockStartProject,
  mockToggleCraftAutoSell,
  mockTreasuryTransfer,
  mockUpgradeCrew,
} from "./mockData";
import type {
  ActivitiesResponse,
  ActivityKindKey,
  ActivityRunDto,
  BrokerListingDto,
  CancelListingResponse,
  CityResponse,
  CitySnapshot,
  ClaimPendingResponse,
  CollectActivitiesResponse,
  CraftRunDto,
  CraftingResponse,
  CrewMemberDto,
  CrewResponse,
  FulfillOrderResponse,
  InventoryResponse,
  MarketResponse,
  MarketSellResponse,
  MeResponse,
  MockPreset,
  ProjectKey,
  ProjectQuoteResponse,
  RegisterResponse,
  RunningProjectDto,
  SetTaxResponse,
  TreasuryBalanceResponse,
} from "./types";

export function newRequestId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `req_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
}

export class ApiError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function parseError(res: Response, path: string): Promise<ApiError> {
  let msg = `Request ${path} failed (HTTP ${res.status})`;
  try {
    const body = (await res.json()) as { error?: { message?: string } };
    if (body.error?.message) msg = body.error.message;
  } catch {
    // Ignore non-JSON body
  }
  return new ApiError(msg, res.status);
}

async function apiGet<T>(path: string, accessToken: string): Promise<T> {
  const res = await fetch(path, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });
  if (!res.ok) throw await parseError(res, path);
  return (await res.json()) as T;
}

async function apiPost<T>(
  path: string,
  accessToken: string,
  body?: unknown,
): Promise<T> {
  const res = await fetch(path, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) throw await parseError(res, path);
  return (await res.json()) as T;
}

export async function fetchSnapshot(
  session: AuthSession,
  mockPreset: MockPreset,
): Promise<CitySnapshot> {
  if (session.mode === "mock") {
    return getMockSnapshot(mockPreset);
  }

  const me = await apiGet<MeResponse>("/api/me", session.accessToken);
  if (!me.registered) {
    return {
      me,
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

  const [city, quoteResp, inventory, activities, market, crafting, crew] =
    await Promise.all([
      apiGet<CityResponse>("/api/city", session.accessToken),
      apiGet<ProjectQuoteResponse>(
        "/api/city/projects/quote",
        session.accessToken,
      ),
      apiGet<InventoryResponse>("/api/inventory", session.accessToken),
      apiGet<ActivitiesResponse>("/api/activities", session.accessToken),
      apiGet<MarketResponse>("/api/market", session.accessToken),
      apiGet<CraftingResponse>("/api/crafting", session.accessToken),
      apiGet<CrewResponse>("/api/crew", session.accessToken),
    ]);

  const parsedServer = Date.parse(city.server_time);
  const clockOffsetMs = Number.isNaN(parsedServer) ? 0 : parsedServer - Date.now();

  return {
    me,
    city,
    quotes: quoteResp.projects,
    inventory,
    activities,
    market,
    crafting,
    crew,
    clockOffsetMs,
  };
}

export async function registerPlayer(
  session: AuthSession,
  mockPreset: MockPreset,
): Promise<RegisterResponse> {
  if (session.mode === "mock") return mockRegister(mockPreset);
  return apiPost<RegisterResponse>("/api/player/register", session.accessToken);
}

export async function startActivity(
  session: AuthSession,
  mockPreset: MockPreset,
  kind: ActivityKindKey,
): Promise<ActivityRunDto> {
  if (session.mode === "mock") return mockStartActivity(mockPreset, kind);
  return apiPost<ActivityRunDto>("/api/activities/start", session.accessToken, { kind });
}

export async function collectActivities(
  session: AuthSession,
  mockPreset: MockPreset,
): Promise<CollectActivitiesResponse> {
  if (session.mode === "mock") return mockCollectActivities(mockPreset);
  return apiPost<CollectActivitiesResponse>("/api/activities/collect", session.accessToken);
}

export async function sellToNpc(
  session: AuthSession,
  mockPreset: MockPreset,
  params: {
    itemId: string;
    quantity?: number;
    sellAll?: boolean;
    requestId: string;
  },
): Promise<MarketSellResponse> {
  if (session.mode === "mock") {
    return mockSellToNpc(
      mockPreset,
      params.itemId,
      params.quantity,
      params.sellAll,
      params.requestId,
    );
  }
  return apiPost<MarketSellResponse>("/api/market/sell", session.accessToken, {
    item_id: params.itemId,
    quantity: params.quantity,
    sell_all: params.sellAll,
    request_id: params.requestId,
  });
}

export async function claimPending(
  session: AuthSession,
  mockPreset: MockPreset,
): Promise<ClaimPendingResponse> {
  if (session.mode === "mock") return mockClaimPending(mockPreset);
  return apiPost<ClaimPendingResponse>("/api/inventory/claim", session.accessToken);
}

export async function startCityProject(
  session: AuthSession,
  mockPreset: MockPreset,
  facility: ProjectKey,
  requestId: string,
): Promise<RunningProjectDto> {
  if (session.mode === "mock") return mockStartProject(mockPreset, facility, requestId);
  return apiPost<RunningProjectDto>("/api/city/projects/start", session.accessToken, {
    facility,
    request_id: requestId,
  });
}

export async function depositTreasury(
  session: AuthSession,
  mockPreset: MockPreset,
  amount: string,
  requestId: string,
): Promise<TreasuryBalanceResponse> {
  if (session.mode === "mock") {
    return mockTreasuryTransfer(mockPreset, "deposit", amount, requestId);
  }
  return apiPost<TreasuryBalanceResponse>(
    "/api/city/treasury/deposit",
    session.accessToken,
    { amount, request_id: requestId },
  );
}

export async function withdrawTreasury(
  session: AuthSession,
  mockPreset: MockPreset,
  amount: string,
  requestId: string,
): Promise<TreasuryBalanceResponse> {
  if (session.mode === "mock") {
    return mockTreasuryTransfer(mockPreset, "withdraw", amount, requestId);
  }
  return apiPost<TreasuryBalanceResponse>(
    "/api/city/treasury/withdraw",
    session.accessToken,
    { amount, request_id: requestId },
  );
}

export async function setCityTaxRate(
  session: AuthSession,
  mockPreset: MockPreset,
  taxRatePct: number,
): Promise<SetTaxResponse> {
  if (session.mode === "mock") return mockSetTaxRate(mockPreset, taxRatePct);
  return apiPost<SetTaxResponse>("/api/city/tax", session.accessToken, {
    tax_rate_pct: taxRatePct,
  });
}

export async function createMarketListing(
  session: AuthSession,
  mockPreset: MockPreset,
  params: {
    itemId: string;
    quantity: number;
    targetUnitPrice: string;
    duration: "2h" | "6h" | "24h";
    requestId: string;
  },
): Promise<BrokerListingDto> {
  if (session.mode === "mock") return mockCreateListing(mockPreset, params);
  return apiPost<BrokerListingDto>("/api/market/listings", session.accessToken, {
    item_id: params.itemId,
    quantity: params.quantity,
    target_unit_price: params.targetUnitPrice,
    duration: params.duration,
    request_id: params.requestId,
  });
}

export async function cancelMarketListing(
  session: AuthSession,
  mockPreset: MockPreset,
  listingId: number,
): Promise<CancelListingResponse> {
  if (session.mode === "mock") return mockCancelListing(mockPreset, listingId);
  return apiPost<CancelListingResponse>(
    "/api/market/listings/cancel",
    session.accessToken,
    { listing_id: listingId },
  );
}

export async function startCraft(
  session: AuthSession,
  mockPreset: MockPreset,
  recipeId: string,
  batchQty: number,
): Promise<CraftRunDto> {
  if (session.mode === "mock") return mockStartCraft(mockPreset, recipeId, batchQty);
  return apiPost<CraftRunDto>("/api/crafting/start", session.accessToken, {
    recipe_id: recipeId,
    batch_qty: batchQty,
  });
}

export async function toggleCraftAutoSell(
  session: AuthSession,
  mockPreset: MockPreset,
): Promise<{ auto_sell: boolean }> {
  if (session.mode === "mock") return mockToggleCraftAutoSell(mockPreset);
  return apiPost<{ auto_sell: boolean }>(
    "/api/crafting/auto-sell",
    session.accessToken,
  );
}

export async function fulfillCraftOrder(
  session: AuthSession,
  mockPreset: MockPreset,
  orderId: number,
): Promise<FulfillOrderResponse> {
  if (session.mode === "mock") return mockFulfillOrder(mockPreset, orderId);
  return apiPost<FulfillOrderResponse>(
    "/api/crafting/orders/fulfill",
    session.accessToken,
    { order_id: orderId },
  );
}

export async function refreshCraftOrders(
  session: AuthSession,
  mockPreset: MockPreset,
): Promise<{ ok: boolean }> {
  if (session.mode === "mock") return mockRefreshOrders(mockPreset);
  return apiPost<{ ok: boolean }>(
    "/api/crafting/orders/refresh",
    session.accessToken,
  );
}

export async function recruitCrew(
  session: AuthSession,
  mockPreset: MockPreset,
  requestId: string,
): Promise<CrewMemberDto> {
  if (session.mode === "mock") return mockRecruitCrew(mockPreset, requestId);
  return apiPost<CrewMemberDto>("/api/crew/recruit", session.accessToken, {
    request_id: requestId,
  });
}

export async function upgradeCrew(
  session: AuthSession,
  mockPreset: MockPreset,
  crewId: number,
  requestId: string,
): Promise<CrewMemberDto> {
  if (session.mode === "mock") return mockUpgradeCrew(mockPreset, crewId, requestId);
  return apiPost<CrewMemberDto>("/api/crew/upgrade", session.accessToken, {
    crew_id: crewId,
    request_id: requestId,
  });
}

export async function assignCrew(
  session: AuthSession,
  mockPreset: MockPreset,
  crewId: number,
  activity: ActivityKindKey | null,
  autoEnabled: boolean,
): Promise<CrewMemberDto> {
  if (session.mode === "mock") {
    return mockAssignCrew(mockPreset, crewId, activity, autoEnabled);
  }
  return apiPost<CrewMemberDto>("/api/crew/assign", session.accessToken, {
    crew_id: crewId,
    activity,
    auto_enabled: autoEnabled,
  });
}
