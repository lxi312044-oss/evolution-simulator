use super::{AccountId, ResourceBank, ResourceError};

#[derive(Debug, Default, Clone, Copy)]
pub struct ResourceLedger {
    pub sunlight_in: f64,
    pub heat_out: f64,
    sunlight_compensation: f64,
    heat_compensation: f64,
}
impl ResourceLedger {
    // Neumaier compensated sum: preserves small additions when totals grow large.
    fn add_compensated(sum: &mut f64, compensation: &mut f64, amount: f64) {
        let next = *sum + amount;
        if sum.abs() >= amount.abs() { *compensation += (*sum - next) + amount; }
        else { *compensation += (amount - next) + *sum; }
        *sum = next;
    }
    pub fn sunlight_total(&self) -> f64 { self.sunlight_in + self.sunlight_compensation }
    pub fn heat_total(&self) -> f64 { self.heat_out + self.heat_compensation }

    pub fn capture_sunlight(&mut self, bank: &mut ResourceBank, to: AccountId, amount: f64) -> Result<(), ResourceError> {
        if !amount.is_finite() || amount < 0.0 { return Err(ResourceError::InvalidAmount); }
        let balance = bank.energy_mut(to)?;
        if !(*balance + amount).is_finite() || !(self.sunlight_in + amount).is_finite() { return Err(ResourceError::CapacityExceeded); }
        *balance += amount;
        Self::add_compensated(&mut self.sunlight_in, &mut self.sunlight_compensation, amount);
        Ok(())
    }
    pub fn dissipate_heat(&mut self, bank: &mut ResourceBank, from: AccountId, amount: f64) -> Result<(), ResourceError> {
        if !amount.is_finite() || amount < 0.0 { return Err(ResourceError::InvalidAmount); }
        let balance = bank.energy_mut(from)?;
        if *balance < amount { return Err(ResourceError::InsufficientBalance); }
        if !(self.heat_out + amount).is_finite() { return Err(ResourceError::CapacityExceeded); }
        *balance -= amount;
        Self::add_compensated(&mut self.heat_out, &mut self.heat_compensation, amount);
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ConservationBaseline { pub nutrients: f64, pub energy: f64 }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConservationError { pub nutrient_delta: f64, pub energy_delta: f64 }
impl ConservationBaseline {
    pub fn capture(bank: &ResourceBank) -> Self { Self { nutrients: bank.total_nutrients(), energy: bank.total_energy() } }
    pub fn check(&self, bank: &ResourceBank, ledger: &ResourceLedger, abs_tol: f64, rel_tol: f64) -> Result<(), ConservationError> {
        let nutrient_delta = bank.total_nutrients() - self.nutrients;
        let energy_delta = (bank.total_energy() - self.energy) - (ledger.sunlight_total() - ledger.heat_total());
        let nutrient_tol = abs_tol.max(rel_tol * self.nutrients.abs());
        let energy_tol = abs_tol.max(rel_tol * (self.energy.abs() + ledger.sunlight_total().abs() + ledger.heat_total().abs()));
        if nutrient_delta.is_finite() && energy_delta.is_finite() && nutrient_delta.abs() <= nutrient_tol && energy_delta.abs() <= energy_tol { Ok(()) }
        else { Err(ConservationError { nutrient_delta, energy_delta }) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compensated_ledger_keeps_small_additions() {
        let mut ledger = ResourceLedger::default();
        let mut bank = ResourceBank::new();
        let account = bank.open(0.0, 0.0).unwrap();
        for _ in 0..100_000 { ledger.capture_sunlight(&mut bank, account, 0.001).unwrap(); }
        assert!((ledger.sunlight_total() - 100.0).abs() < 1e-10);
    }
    #[test]
    fn ledger_tracks_heat_with_compensation() {
        let mut ledger = ResourceLedger::default();
        let mut bank = ResourceBank::new();
        let account = bank.open(0.0, 100.0).unwrap();
        for _ in 0..10_000 { ledger.dissipate_heat(&mut bank, account, 0.001).unwrap(); }
        assert!((ledger.heat_total() - 10.0).abs() < 1e-11);
    }
}
