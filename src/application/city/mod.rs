use crate::domain::city::CityStage;

/// Stage kota pemain. Semua pemain masih Kampung; kota per pemain dibuat di tahap City Development.
pub fn stage_for(_player_id: i64) -> CityStage {
    CityStage::Kampung
}
