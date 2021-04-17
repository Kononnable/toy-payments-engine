use std::ops::{Add, AddAssign, Sub, SubAssign};

// Value multiplied by 10_000
// TODO: conversion methods
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecimalType(u32);
impl Add for DecimalType {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            0: self.0 + other.0,
        }
    }
}

impl AddAssign for DecimalType {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            0: self.0 + other.0,
        };
    }
}

impl Sub for DecimalType {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            0: self.0 - other.0,
        }
    }
}
impl SubAssign for DecimalType {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            0: self.0 - other.0,
        };
    }
}

#[derive(Debug)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
// TODO: Deserialize
#[derive(Debug)]
pub struct Transaction {
    pub ty: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<DecimalType>,
}
