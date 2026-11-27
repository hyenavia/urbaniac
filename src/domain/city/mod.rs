#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CityStage {
    Kampung,
    Desa,
    KotaKecil,
    KotaDagang,
    Metropolis,
}

impl CityStage {
    pub fn name(&self) -> &'static str {
        match self {
            CityStage::Kampung => "Kampung",
            CityStage::Desa => "Desa",
            CityStage::KotaKecil => "Kota Kecil",
            CityStage::KotaDagang => "Kota Dagang",
            CityStage::Metropolis => "Metropolis",
        }
    }
}
