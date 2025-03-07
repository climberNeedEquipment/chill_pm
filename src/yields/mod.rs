mod aave;
mod eigen_layer;
mod lido;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub use aave::*;
pub use eigen_layer::*;
pub use lido::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct APR {
    pub symbol: String,
    pub deposit_apr: f64,
    pub borrow_apr: Option<f64>,
}

#[async_trait]
pub trait Yield {
    fn get_symbol() -> String;
    async fn get_apr(&self) -> Result<Vec<APR>, Box<dyn Error>>;
}

pub struct CombinedYieldFetcher {
    pub aave: Aave,
    pub lido: Lido,
    pub eigen: Eigen,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombinedYields {
    pub aave: Vec<APR>,
    pub lido: Vec<APR>,
    pub eigen: Vec<APR>,
}

impl CombinedYieldFetcher {
    pub fn new() -> Self {
        Self {
            aave: Aave {},
            lido: Lido {},
            eigen: Eigen {},
        }
    }

    pub async fn get_apr(&self) -> Result<CombinedYields, Box<dyn Error>> {
        let aave_apr = self.aave.get_apr().await?;
        let lido_apr = self.lido.get_apr().await?;
        let eigen_apr = self.eigen.get_apr().await?;

        Ok(CombinedYields {
            aave: aave_apr,
            lido: lido_apr,
            eigen: eigen_apr,
        })
    }
}

impl std::fmt::Display for APR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: Deposit APR: {:.2}%", self.symbol, self.deposit_apr)?;
        if let Some(borrow_apr) = self.borrow_apr {
            write!(f, ", Borrow APR: {:.2}%", borrow_apr)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for CombinedYields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Aave Yields:")?;
        for apr in &self.aave {
            writeln!(f, "  {}", apr)?;
        }

        writeln!(f, "\nLido Yields:")?;
        for apr in &self.lido {
            writeln!(f, "  {}", apr)?;
        }

        writeln!(f, "\nEigen Yields:")?;
        for apr in &self.eigen {
            writeln!(f, "  {}", apr)?;
        }

        Ok(())
    }
}
