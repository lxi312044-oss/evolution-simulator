use super::{ResourceError, TransferReceipt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountId(pub usize);

#[derive(Debug, Default)]
pub struct ResourceBank {
    nutrients: Vec<f64>,
    chemical_energy: Vec<f64>,
}
impl ResourceBank {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn open(&mut self, nutrients: f64, energy: f64) -> Result<AccountId, ResourceError> {
        if !valid(nutrients) || !valid(energy) {
            return Err(ResourceError::InvalidAmount);
        }
        let id = AccountId(self.nutrients.len());
        self.nutrients.push(nutrients);
        self.chemical_energy.push(energy);
        Ok(id)
    }
    pub fn nutrient_balance(&self, id: AccountId) -> Option<f64> {
        self.nutrients.get(id.0).copied()
    }
    pub fn energy_balance(&self, id: AccountId) -> Option<f64> {
        self.chemical_energy.get(id.0).copied()
    }
    pub fn transfer_nutrients(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: f64,
    ) -> Result<TransferReceipt, ResourceError> {
        transfer(&mut self.nutrients, from, to, amount)
    }
    pub fn transfer_energy(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: f64,
    ) -> Result<TransferReceipt, ResourceError> {
        transfer(&mut self.chemical_energy, from, to, amount)
    }
    pub fn total_nutrients(&self) -> f64 {
        self.nutrients.iter().sum()
    }
    pub fn total_energy(&self) -> f64 {
        compensated_total(&self.chemical_energy)
    }
    pub(crate) fn energy_mut(&mut self, id: AccountId) -> Result<&mut f64, ResourceError> {
        self.chemical_energy
            .get_mut(id.0)
            .ok_or(ResourceError::InvalidAccount)
    }
}
fn valid(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}
fn transfer(
    balances: &mut [f64],
    from: AccountId,
    to: AccountId,
    amount: f64,
) -> Result<TransferReceipt, ResourceError> {
    if !valid(amount) {
        return Err(ResourceError::InvalidAmount);
    }
    if from == to {
        return Err(ResourceError::SameAccount);
    }
    let source = *balances.get(from.0).ok_or(ResourceError::InvalidAccount)?;
    let target = *balances.get(to.0).ok_or(ResourceError::InvalidAccount)?;
    if source < amount {
        return Err(ResourceError::InsufficientBalance);
    }
    if !target.is_finite() || !((target + amount).is_finite()) {
        return Err(ResourceError::CapacityExceeded);
    }
    balances[from.0] = source - amount;
    balances[to.0] = target + amount;
    Ok(TransferReceipt { amount })
}

// Stable aggregation of many accounts, including inactive zero-balance accounts.
fn compensated_total(values: &[f64]) -> f64 {
    let (mut sum, mut compensation) = (0.0_f64, 0.0_f64);
    for &x in values {
        let next = sum + x;
        if sum.abs() >= x.abs() {
            compensation += (sum - next) + x;
        } else {
            compensation += (x - next) + sum;
        }
        sum = next;
    }
    sum + compensation
}
