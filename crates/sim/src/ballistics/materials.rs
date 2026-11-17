//! Surface materials (`data/materials/*.json`).

use crate::data::defs::MaterialDef;

/// Index into a [`MaterialTable`] (stable: files sorted by id in the embedded data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialId(pub u8);

/// One material, in SI-ish units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Penetration points spent per millimetre.
    pub cost_per_mm: f32,
    /// Thicker than this always stops the bullet; `0` = no cap.
    pub stop_mm: f32,
    /// Thinner layers are ignored; `0` = none.
    pub min_mm: f32,
    /// Exit direction change, +- degrees (1.5 hard, 0.5 soft).
    pub deflect_deg: f32,
    /// Hard enough to ricochet.
    pub ricochet: bool,
}

impl Material {
    pub fn from_def(d: &MaterialDef) -> Self {
        let g = &d.gameplay;
        Self {
            cost_per_mm: g.cost_per_mm as f32,
            stop_mm: g.stop_mm as f32,
            min_mm: g.min_mm as f32,
            deflect_deg: g.deflect_deg as f32,
            ricochet: g.ricochet,
        }
    }
}

/// All materials of the data set.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialTable {
    ids: Vec<String>,
    mats: Vec<Material>,
}

impl MaterialTable {
    pub fn from_defs(defs: &[MaterialDef]) -> Self {
        let mut sorted: Vec<&MaterialDef> = defs.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        Self {
            ids: sorted.iter().map(|d| d.id.clone()).collect(),
            mats: sorted.iter().map(|d| Material::from_def(d)).collect(),
        }
    }

    /// The table of the embedded game data.
    pub fn embedded() -> Self {
        Self::from_defs(crate::data::GameData::embedded().materials)
    }

    pub fn id_of(&self, name: &str) -> Option<MaterialId> {
        self.ids
            .iter()
            .position(|i| i == name)
            .map(|i| MaterialId(i as u8))
    }

    pub fn name(&self, id: MaterialId) -> Option<&str> {
        self.ids.get(usize::from(id.0)).map(String::as_str)
    }

    pub fn get(&self, id: MaterialId) -> Option<&Material> {
        self.mats.get(usize::from(id.0))
    }

    pub fn len(&self) -> usize {
        self.mats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mats.is_empty()
    }
}
