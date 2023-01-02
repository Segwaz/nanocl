#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use super::cargo_config::{CargoConfig, CargoConfigPartial};

/// Structure to create a cargo used as body
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct CargoPartial {
  pub name: String,
  pub config: CargoConfigPartial,
}

/// Cargo with his current config
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct Cargo {
  pub key: String,
  pub namespace_name: String,
  pub name: String,
  pub config_key: uuid::Uuid,
  pub config: CargoConfig,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct CargoSummary {
  pub key: String,
  pub name: String,
  pub config_key: uuid::Uuid,
  pub namespace_name: String,
  pub config: CargoConfig,
  pub running_instances: i64,
}
