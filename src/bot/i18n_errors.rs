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

pub fn wallet_error(err: &WalletError, lang: Lang) -> String {
    match (err, lang) {
        (WalletError::InsufficientFunds, Lang::En) => "Insufficient Bac balance.".to_string(),
        (WalletError::InsufficientFunds, Lang::Id) => "Saldo Bac tidak cukup.".to_string(),
        (WalletError::NotFound, Lang::En) => "Wallet not found.".to_string(),
        (WalletError::NotFound, Lang::Id) => "Wallet tidak ditemukan.".to_string(),
        (WalletError::Db(e), _) => e.to_string(),
    }
}

pub fn activity_error(err: &ActivityError, lang: Lang) -> String {
    match (err, lang) {
        (ActivityError::NotRegistered, Lang::En) => "No profile yet, run `/start`.".to_string(),
        (ActivityError::NotRegistered, Lang::Id) => "Profil belum ada, jalankan `/start`.".to_string(),
        (ActivityError::Locked(stage), Lang::En) => {
            format!("This activity unlocks at city stage **{}**.", stage_name(*stage, lang))
        }
        (ActivityError::Locked(stage), Lang::Id) => {
            format!("Aktivitas ini terbuka di stage **{}**.", stage_name(*stage, lang))
        }
        (ActivityError::NoFreeSlot(total), Lang::En) => format!(
            "All Activity Slots are busy ({total} slots). Collect finished rewards or wait for an activity to finish."
        ),
        (ActivityError::NoFreeSlot(total), Lang::Id) => format!(
            "Semua Activity Slot terpakai ({total} slot). Ambil reward yang sudah selesai atau tunggu aktivitas berjalan."
        ),
        (ActivityError::Db(e), _) => e.to_string(),
    }
}

pub fn city_error(err: &CityError, lang: Lang) -> String {
    match err {
        CityError::NotRegistered => match lang {
            Lang::En => "No profile yet, run `/start`.".to_string(),
            Lang::Id => "Profil belum ada, jalankan `/start`.".to_string(),
        },
        CityError::ProjectAlreadyRunning(name) => {
            let p_name = project_name_by_any(name, lang);
            match lang {
                Lang::En => format!("Project **{p_name}** is already in progress."),
                Lang::Id => format!("Proyek **{p_name}** sudah berjalan."),
            }
        }
        CityError::AlreadyBuilt(name) => {
            let p_name = project_name_by_any(name, lang);
            match lang {
                Lang::En => format!("Facility **{p_name}** is already at max level."),
                Lang::Id => format!("Fasilitas **{p_name}** sudah mencapai level maksimal."),
            }
        }
        CityError::StageTooLow(stage) => match lang {
            Lang::En => format!("This project requires city stage **{}**.", stage_name(*stage, lang)),
            Lang::Id => format!("Proyek ini membutuhkan stage kota **{}**.", stage_name(*stage, lang)),
        },
        CityError::MissingPrerequisites => match lang {
            Lang::En => "Prerequisite facilities are not built yet.".to_string(),
            Lang::Id => "Prasyarat fasilitas belum terpenuhi.".to_string(),
        },
        CityError::StageAlreadyAdvanced => match lang {
            Lang::En => "This stage-upgrade project has already been passed.".to_string(),
            Lang::Id => "Proyek kenaikan stage ini sudah terlewati.".to_string(),
        },
        CityError::InsufficientBac { need, have } => match lang {
            Lang::En => format!(
                "Insufficient Bac (need {}, have {}).",
                format_bac(*need),
                format_bac(*have)
            ),
            Lang::Id => format!(
                "Saldo Bac tidak cukup (butuh {}, saldo {}).",
                format_bac(*need),
                format_bac(*have)
            ),
        },
        CityError::InsufficientTreasury { need, have } => match lang {
            Lang::En => format!(
                "Insufficient City Treasury (need {}, have {}).",
                format_bac(*need),
                format_bac(*have)
            ),
            Lang::Id => format!(
                "Saldo City Treasury tidak cukup (butuh {}, saldo {}).",
                format_bac(*need),
                format_bac(*have)
            ),
        },
        CityError::InsufficientItem { name, need, have } => {
            let item = item_name_by_any(name, lang);
            match lang {
                Lang::En => format!("Not enough **{item}** (need {need}, have {have})."),
                Lang::Id => format!("Resource **{item}** tidak cukup (butuh {need}, punya {have})."),
            }
        }
        CityError::InvalidAmount => match lang {
            Lang::En => "Deposit amount must be greater than 0.".to_string(),
            Lang::Id => "Jumlah setoran harus lebih dari 0.".to_string(),
        },
        CityError::InvalidTaxRate => match lang {
            Lang::En => format!("Tax rate must be between {MIN_TAX_RATE}%–{MAX_TAX_RATE}%."),
            Lang::Id => format!("Tarif pajak harus {MIN_TAX_RATE}%–{MAX_TAX_RATE}%."),
        },
        CityError::Wallet(w) => wallet_error(w, lang),
        CityError::Db(e) => e.to_string(),
    }
}

pub fn craft_error(err: &CraftError, lang: Lang) -> String {
    match err {
        CraftError::NotRegistered => match lang {
            Lang::En => "No profile yet, run `/start`.".to_string(),
            Lang::Id => "Profil belum ada, jalankan `/start`.".to_string(),
        },
        CraftError::UnknownRecipe => match lang {
            Lang::En => "Unknown recipe.".to_string(),
            Lang::Id => "Resep tidak dikenal.".to_string(),
        },
        CraftError::InvalidQty => match lang {
            Lang::En => "Invalid crafting quantity.".to_string(),
            Lang::Id => "Jumlah crafting tidak valid.".to_string(),
        },
        CraftError::TooManyCrafts(max) => match lang {
            Lang::En => format!("Maximum of {max} concurrent crafts reached."),
            Lang::Id => format!("Maksimal {max} crafting berjalan bersamaan."),
        },
        CraftError::MissingIngredient { name, need, have } => {
            let item = item_name_by_any(name, lang);
            match lang {
                Lang::En => format!("Not enough **{item}** (need {need}, have {have})."),
                Lang::Id => format!("Bahan **{item}** tidak cukup (butuh {need}, punya {have})."),
            }
        }
        CraftError::OrderNotFound => match lang {
            Lang::En => "Order not found or already completed.".to_string(),
            Lang::Id => "Pesanan tidak ditemukan atau sudah selesai.".to_string(),
        },
        CraftError::InsufficientOrderItem { name, need, have } => {
            let item = item_name_by_any(name, lang);
            match lang {
                Lang::En => format!("Not enough **{item}** for order (need {need}, have {have})."),
                Lang::Id => format!("Item **{item}** kurang untuk pesanan (butuh {need}, punya {have})."),
            }
        }
        CraftError::OrderRefreshCooldown(secs) => match lang {
            Lang::En => format!("Order refresh on cooldown — try again in {secs}s."),
            Lang::Id => format!("Ganti pesanan sedang cooldown — coba lagi dalam {secs}d."),
        },
        CraftError::Wallet(w) => wallet_error(w, lang),
        CraftError::Db(e) => e.to_string(),
    }
}

pub fn crew_error(err: &CrewError, lang: Lang) -> String {
    match err {
        CrewError::NotRegistered => match lang {
            Lang::En => "No profile yet, run `/start`.".to_string(),
            Lang::Id => "Profil belum ada, jalankan `/start`.".to_string(),
        },
        CrewError::CrewFull(max) => match lang {
            Lang::En => format!("Crew roster is full (max {max} crew members)."),
            Lang::Id => format!("Slot kru sudah penuh (maksimal {max} kru)."),
        },
        CrewError::NotFound => match lang {
            Lang::En => "Crew member not found.".to_string(),
            Lang::Id => "Kru tidak ditemukan.".to_string(),
        },
        CrewError::MaxLevel(max) => match lang {
            Lang::En => format!("Crew member is already at max level ({max})."),
            Lang::Id => format!("Kru sudah di level maksimal ({max})."),
        },
        CrewError::ActivityLocked(stage) => match lang {
            Lang::En => format!("This activity unlocks at city stage **{}**.", stage_name(*stage, lang)),
            Lang::Id => format!("Aktivitas ini terbuka di stage **{}**.", stage_name(*stage, lang)),
        },
        CrewError::InsufficientBac { need, have } => match lang {
            Lang::En => format!(
                "Insufficient Bac (need {}, have {}).",
                format_bac(*need),
                format_bac(*have)
            ),
            Lang::Id => format!(
                "Saldo Bac tidak cukup (butuh {}, saldo {}).",
                format_bac(*need),
                format_bac(*have)
            ),
        },
        CrewError::Wallet(w) => wallet_error(w, lang),
        CrewError::Db(e) => e.to_string(),
    }
}

pub fn market_error(err: &MarketError, lang: Lang) -> String {
    match err {
        MarketError::NotRegistered => match lang {
            Lang::En => "No profile yet, run `/start`.".to_string(),
            Lang::Id => "Profil belum ada, jalankan `/start`.".to_string(),
        },
        MarketError::UnknownItem => match lang {
            Lang::En => "Unknown item.".to_string(),
            Lang::Id => "Item tidak dikenal.".to_string(),
        },
        MarketError::InvalidQty => match lang {
            Lang::En => "Invalid quantity.".to_string(),
            Lang::Id => "Jumlah tidak valid.".to_string(),
        },
        MarketError::InvalidPrice => match lang {
            Lang::En => "Target price must be greater than 0.".to_string(),
            Lang::Id => "Harga target harus lebih dari 0.".to_string(),
        },
        MarketError::TooManyListings(max) => match lang {
            Lang::En => format!("Maximum of {max} active broker listings reached."),
            Lang::Id => format!("Maksimal {max} listing broker aktif."),
        },
        MarketError::InsufficientItem { have } => match lang {
            Lang::En => format!("Not enough items (you have {have})."),
            Lang::Id => format!("Jumlah item tidak cukup (kamu punya {have})."),
        },
        MarketError::InsufficientBacForFee { need, have } => match lang {
            Lang::En => format!(
                "Insufficient Bac for listing fee (need {}, have {}).",
                format_bac(*need),
                format_bac(*have)
            ),
            Lang::Id => format!(
                "Saldo Bac tidak cukup untuk fee listing (butuh {}, saldo {}).",
                format_bac(*need),
                format_bac(*have)
            ),
        },
        MarketError::ListingNotFound => match lang {
            Lang::En => "Listing not found or already closed.".to_string(),
            Lang::Id => "Listing tidak ditemukan atau sudah selesai.".to_string(),
        },
        MarketError::Wallet(w) => wallet_error(w, lang),
        MarketError::Db(e) => e.to_string(),
    }
}

pub fn sell_error(err: &SellError, lang: Lang) -> String {
    match err {
        SellError::UnknownItem => match lang {
            Lang::En => "Unknown item.".to_string(),
            Lang::Id => "Item tidak dikenal.".to_string(),
        },
        SellError::InvalidQty => match lang {
            Lang::En => "Invalid quantity.".to_string(),
            Lang::Id => "Jumlah tidak valid.".to_string(),
        },
        SellError::NotEnough { have } => match lang {
            Lang::En => format!("Not enough items (you have {have})."),
            Lang::Id => format!("Jumlah item tidak cukup (kamu punya {have})."),
        },
        SellError::Wallet(w) => wallet_error(w, lang),
        SellError::Db(e) => e.to_string(),
    }
}
