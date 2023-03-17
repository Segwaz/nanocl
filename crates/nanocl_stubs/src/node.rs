#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub struct NodeDbModel {
  pub(crate) name: String,
  pub(crate) ip_address: String,
}
