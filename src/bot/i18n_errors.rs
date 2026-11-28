use crate::application::activities::ActivityError;
use crate::application::city::CityError;
use crate::application::crafting::CraftError;
use crate::application::crew::CrewError;
use crate::application::economy::SellError;
use crate::application::market::MarketError;
use crate::domain::city::{MAX_TAX_RATE, MIN_TAX_RATE};
use crate::domain::economy::format_bac;
use crate::domain::i18n::{item_name_by_any, project_name_by_any, stage_name, Lang};
use crate::infrastructure::repositories::wallet_repo::WalletError;

pub fn wallet_error(err: &WalletError, _lang: Lang) -> String {
    match err {
        WalletError::InsufficientFunds => "Insufficient Bac balance.".to_string(),
        WalletError::NotFound => "Wallet not found.".to_string(),
        WalletError::Db(e) => e.to_string(),
    }
}

pub fn activity_error(err: &ActivityError, lang: Lang) -> String {
    match err {
        ActivityError::NotRegistered => {
            "No profile yet. Click **Play** or open the **Urbaniac Activity**.".to_string()
        }
        ActivityError::Locked(stage) => {
            format!("This activity unlocks at city stage **{}**.", stage_name(*stage, lang))
        }
        ActivityError::NoFreeSlot(total) => format!(
            "All Activity Slots are busy ({total} slots). Collect finished rewards or wait for an activity to finish."
        ),
        ActivityError::Db(e) => e.to_string(),
    }
}

pub fn city_error(err: &CityError, lang: Lang) -> String {
    match err {
        CityError::NotRegistered => {
            "No profile yet. Click **Play** or open the **Urbaniac Activity**.".to_string()
        }
        CityError::ProjectAlreadyRunning(name) => {
            let p_name = project_name_by_any(name, lang);
            format!("Project **{p_name}** is already in progress.")
        }
        CityError::AlreadyBuilt(name) => {
            let p_name = project_name_by_any(name, lang);
            format!("Facility **{p_name}** is already at max level.")
        }
        CityError::StageTooLow(stage) => {
            format!("This project requires city stage **{}**.", stage_name(*stage, lang))
        }
        CityError::MissingPrerequisites => "Prerequisite facilities are not built yet.".to_string(),
        CityError::StageAlreadyAdvanced => "This stage-upgrade project has already been passed.".to_string(),
        CityError::InsufficientBac { need, have } => format!(
            "Insufficient Bac (need {}, have {}).",
            format_bac(*need),
            format_bac(*have)
        ),
        CityError::InsufficientTreasury { need, have } => format!(
            "Insufficient City Treasury (need {}, have {}).",
            format_bac(*need),
            format_bac(*have)
        ),
        CityError::InsufficientItem { name, need, have } => {
            let item = item_name_by_any(name, lang);
            format!("Not enough **{item}** (need {need}, have {have}).")
        }
        CityError::InvalidAmount => "Deposit amount must be greater than 0.".to_string(),
        CityError::InvalidTaxRate => format!("Tax rate must be between {MIN_TAX_RATE}%–{MAX_TAX_RATE}%."),
        CityError::Wallet(w) => wallet_error(w, lang),
        CityError::Db(e) => e.to_string(),
    }
}

pub fn craft_error(err: &CraftError, lang: Lang) -> String {
    match err {
        CraftError::NotRegistered => {
            "No profile yet. Click **Play** or open the **Urbaniac Activity**.".to_string()
        }
        CraftError::UnknownRecipe => "Unknown recipe.".to_string(),
        CraftError::InvalidQty => "Invalid crafting quantity.".to_string(),
        CraftError::TooManyCrafts(max) => format!("Maximum of {max} concurrent crafts reached."),
        CraftError::MissingIngredient { name, need, have } => {
            let item = item_name_by_any(name, lang);
            format!("Not enough **{item}** (need {need}, have {have}).")
        }
        CraftError::OrderNotFound => "Order not found or already completed.".to_string(),
        CraftError::InsufficientOrderItem { name, need, have } => {
            let item = item_name_by_any(name, lang);
            format!("Not enough **{item}** for order (need {need}, have {have}).")
        }
        CraftError::OrderRefreshCooldown(secs) => {
            format!("Order refresh on cooldown — try again in {secs}s.")
        }
        CraftError::Wallet(w) => wallet_error(w, lang),
        CraftError::Db(e) => e.to_string(),
    }
}

pub fn crew_error(err: &CrewError, lang: Lang) -> String {
    match err {
        CrewError::NotRegistered => {
            "No profile yet. Click **Play** or open the **Urbaniac Activity**.".to_string()
        }
        CrewError::CrewFull(max) => format!("Crew roster is full (max {max} crew members)."),
        CrewError::NotFound => "Crew member not found.".to_string(),
        CrewError::MaxLevel(max) => format!("Crew member is already at max level ({max})."),
        CrewError::ActivityLocked(stage) => {
            format!("This activity unlocks at city stage **{}**.", stage_name(*stage, lang))
        }
        CrewError::InsufficientBac { need, have } => format!(
            "Insufficient Bac (need {}, have {}).",
            format_bac(*need),
            format_bac(*have)
        ),
        CrewError::Wallet(w) => wallet_error(w, lang),
        CrewError::Db(e) => e.to_string(),
    }
}

pub fn market_error(err: &MarketError, lang: Lang) -> String {
    match err {
        MarketError::NotRegistered => {
            "No profile yet. Click **Play** or open the **Urbaniac Activity**.".to_string()
        }
        MarketError::UnknownItem => "Unknown item.".to_string(),
        MarketError::InvalidQty => "Invalid quantity.".to_string(),
        MarketError::InvalidPrice => "Target price must be greater than 0.".to_string(),
        MarketError::TooManyListings(max) => format!("Maximum of {max} active broker listings reached."),
        MarketError::InsufficientItem { have } => format!("Not enough items (you have {have})."),
        MarketError::InsufficientBacForFee { need, have } => format!(
            "Insufficient Bac for listing fee (need {}, have {}).",
            format_bac(*need),
            format_bac(*have)
        ),
        MarketError::ListingNotFound => "Listing not found or already closed.".to_string(),
        MarketError::Wallet(w) => wallet_error(w, lang),
        MarketError::Db(e) => e.to_string(),
    }
}

pub fn sell_error(err: &SellError, lang: Lang) -> String {
    match err {
        SellError::UnknownItem => "Unknown item.".to_string(),
        SellError::InvalidQty => "Invalid quantity.".to_string(),
        SellError::NotEnough { have } => format!("Not enough items (you have {have})."),
        SellError::Wallet(w) => wallet_error(w, lang),
        SellError::Db(e) => e.to_string(),
    }
}
