use crate::data::bom;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Build {
    pub bom: bom::Bom,
    pub unmet: Vec<String>,
}
