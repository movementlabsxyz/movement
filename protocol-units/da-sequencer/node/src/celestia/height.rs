use std::{
	fmt::{Display, Formatter},
	ops::{Add, AddAssign, Sub},
};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CelestiaHeight(u64);

impl From<u64> for CelestiaHeight {
	fn from(value: u64) -> Self {
		CelestiaHeight(value)
	}
}

impl From<CelestiaHeight> for u64 {
	fn from(height: CelestiaHeight) -> Self {
		height.0
	}
}

// Rust interprets (small) integer literals without a type suffix as i32
impl<T: Into<i64>> Add<T> for CelestiaHeight {
	type Output = Self;

	fn add(self, rhs: T) -> Self::Output {
		let value = <i64 as TryInto<u64>>::try_into(rhs.into()).expect("Added a negative value");
		CelestiaHeight(self.0 + value)
	}
}

impl<T: Into<i64>> Sub<T> for CelestiaHeight {
	type Output = Self;

	fn sub(self, rhs: T) -> Self::Output {
		let value = rhs.into().abs() as u64;
		CelestiaHeight(self.0.saturating_sub(value))
	}
}

// Rust interprets (small) integer literals without a type suffix as i32
impl<T: Into<i64>> AddAssign<T> for CelestiaHeight {
	fn add_assign(&mut self, rhs: T) {
		let value = <i64 as TryInto<u64>>::try_into(rhs.into()).expect("Added a negative value");
		self.0 += value;
	}
}

impl Display for CelestiaHeight {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}
