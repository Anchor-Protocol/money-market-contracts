use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize};
use std::convert::TryFrom;
use std::fmt::{self, Write};
use std::ops;
use std::str::FromStr;

use bigint::U256;
use cosmwasm_std::{StdError, Uint128};

/// A fixed-point decimal value with 18 fractional digits, i.e. Decimal256(1_000_000_000_000_000_000) == 1.0
/// The greatest possible value that can be represented is 115792089237316195423570985008687907853269984665640564039457.584007913129639935 (which is (2^128 - 1) / 10^18)
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct Decimal256(#[schemars(with = "String")] pub U256);

impl Decimal256 {
    pub const MAX: Decimal256 = Decimal256(U256::MAX);
    pub const DECIMAL_FRACTIONAL: U256 = U256([1_000_000_000_000_000_000u64, 0, 0, 0]);

    /// Create a 1.0 Decimal256
    pub const fn one() -> Decimal256 {
        Decimal256(Decimal256::DECIMAL_FRACTIONAL)
    }

    /// Create a 0.0 Decimal256
    pub const fn zero() -> Decimal256 {
        Decimal256(U256([0, 0, 0, 0]))
    }

    /// Convert x% into Decimal256
    pub fn percent(x: u64) -> Decimal256 {
        Decimal256(U256::from(x) * U256::from(10_000_000_000_000_000u64))
    }

    /// Convert permille (x/1000) into Decimal256
    pub fn permille(x: u64) -> Decimal256 {
        Decimal256(U256::from(x) * U256::from(1_000_000_000_000_000u64))
    }

    /// Returns the ratio (nominator / denominator) as a Decimal256
    pub fn from_ratio<A: Into<U256>, B: Into<U256>>(nominator: A, denominator: B) -> Decimal256 {
        let nominator: U256 = nominator.into();
        let denominator: U256 = denominator.into();
        if denominator.is_zero() {
            panic!("Denominator must not be zero");
        }

        Decimal256(nominator * Decimal256::DECIMAL_FRACTIONAL / denominator)
    }

    pub fn from_uint256<A: Into<Uint256>>(val: A) -> Decimal256 {
        let num: Uint256 = val.into();
        Decimal256(num.0 * Decimal256::DECIMAL_FRACTIONAL)
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl FromStr for Decimal256 {
    type Err = StdError;

    /// Converts the decimal string to a Decimal256
    /// Possible inputs: "1.23", "1", "000012", "1.123000000"
    /// Disallowed: "", ".23"
    ///
    /// This never performs any kind of rounding.
    /// More than 18 fractional digits, even zeros, result in an error.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = input.split('.').collect();
        match parts.len() {
            1 => {
                let whole = U256::from_dec_str(parts[0])
                    .map_err(|_| StdError::generic_err("Error parsing whole"))?;

                let whole_as_atomics = whole * Decimal256::DECIMAL_FRACTIONAL;
                Ok(Decimal256(whole_as_atomics))
            }
            2 => {
                let whole = U256::from_dec_str(parts[0])
                    .map_err(|_| StdError::generic_err("Error parsing whole"))?;
                let fractional = U256::from_dec_str(parts[1])
                    .map_err(|_| StdError::generic_err("Error parsing fractional"))?;
                let exp = (18usize.checked_sub(parts[1].len())).ok_or_else(|| {
                    StdError::generic_err("Cannot parse more than 18 fractional digits")
                })?;
                let fractional_factor = U256::from(10).pow(exp.into());

                let whole_as_atomics = whole * Decimal256::DECIMAL_FRACTIONAL;
                let atomics = whole_as_atomics + fractional * fractional_factor;
                Ok(Decimal256(atomics))
            }
            _ => Err(StdError::generic_err("Unexpected number of dots")),
        }
    }
}

impl fmt::Display for Decimal256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let whole = (self.0) / Decimal256::DECIMAL_FRACTIONAL;
        let fractional = (self.0) % Decimal256::DECIMAL_FRACTIONAL;

        if fractional.is_zero() {
            write!(f, "{}", whole)
        } else {
            let fractional_string = fractional.to_string();
            let fractional_string = "0".repeat(18 - fractional_string.len()) + &fractional_string;

            f.write_str(&whole.to_string())?;
            f.write_char('.')?;
            f.write_str(fractional_string.trim_end_matches('0'))?;

            Ok(())
        }
    }
}

impl ops::Add for Decimal256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Decimal256(self.0 + rhs.0)
    }
}

impl ops::AddAssign for Decimal256 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0;
    }
}

impl ops::Sub for Decimal256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        assert!(self.0 >= rhs.0);
        Decimal256(self.0 - rhs.0)
    }
}

impl ops::Mul for Decimal256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Decimal256(self.0 * rhs.0 / Decimal256::DECIMAL_FRACTIONAL)
    }
}

impl ops::Div for Decimal256 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        assert!(!rhs.is_zero());

        Decimal256(self.0 * Decimal256::DECIMAL_FRACTIONAL / rhs.0)
    }
}

/// Serializes as a decimal string
impl Serialize for Decimal256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Deserializes as a base64 string
impl<'de> Deserialize<'de> for Decimal256 {
    fn deserialize<D>(deserializer: D) -> Result<Decimal256, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Decimal256Visitor)
    }
}

struct Decimal256Visitor;

impl<'de> de::Visitor<'de> for Decimal256Visitor {
    type Value = Decimal256;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string-encoded decimal")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match Decimal256::from_str(v) {
            Ok(d) => Ok(d),
            Err(e) => Err(E::custom(format!("Error parsing decimal '{}': {}", v, e))),
        }
    }
}

//*** Uint256 ***/
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct Uint256(#[schemars(with = "String")] pub U256);

impl Uint256 {
    /// Creates a Uint256(0)
    pub const fn zero() -> Self {
        Uint256(U256([0, 0, 0, 0]))
    }

    /// Create a 1.0 Decimal256
    pub const fn one() -> Self {
        Uint256(U256([1, 0, 0, 0]))
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl From<U256> for Uint256 {
    fn from(val: U256) -> Self {
        Uint256(val)
    }
}

#[inline(always)]
fn split_u128(a: u128) -> (u64, u64) {
    ((a >> 64) as _, (a & 0xFFFFFFFFFFFFFFFF) as _)
}

impl From<Uint128> for Uint256 {
    fn from(val: Uint128) -> Self {
        Uint256::from(val.u128())
    }
}

impl From<u128> for Uint256 {
    fn from(val: u128) -> Self {
        let (hi, low) = split_u128(val);
        Uint256(U256([low, hi, 0, 0]))
    }
}

impl From<u64> for Uint256 {
    fn from(val: u64) -> Self {
        Uint256(val.into())
    }
}

impl TryFrom<&str> for Uint256 {
    type Error = StdError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        match U256::from_dec_str(val) {
            Ok(u) => Ok(Uint256(u)),
            Err(_e) => Err(StdError::generic_err(format!("invalid Uint256 '{}'", val))),
        }
    }
}

impl FromStr for Uint256 {
    type Err = StdError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let number =
            U256::from_dec_str(input).map_err(|_| StdError::generic_err("Error parsing number"))?;
        Ok(Uint256(number))
    }
}

impl Into<String> for Uint256 {
    fn into(self) -> String {
        self.0.to_string()
    }
}

impl Into<u128> for Uint256 {
    fn into(self) -> u128 {
        let U256(ref arr) = self.0;
        assert!(arr[2] == 0u64);
        assert!(arr[3] == 0u64);

        let (hi, low) = (arr[1], arr[0]);
        ((hi as u128) << 64) + (low as u128)
    }
}

impl Into<Uint128> for Uint256 {
    fn into(self) -> Uint128 {
        Uint128(self.into())
    }
}

impl fmt::Display for Uint256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ops::Add for Uint256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Uint256(self.0 + rhs.0)
    }
}

impl ops::AddAssign for Uint256 {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0 + other.0;
    }
}

impl ops::Sub for Uint256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        assert!(self.0 >= rhs.0);
        Uint256(self.0 - rhs.0)
    }
}

impl ops::Mul<Uint256> for Uint256 {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: Uint256) -> Self::Output {
        // 0*a and b*0 is always 0
        if self.is_zero() || rhs.is_zero() {
            return Uint256::zero();
        }

        Uint256(self.0 * rhs.0)
    }
}

/// Both d*u and u*d with d: Decimal256 and u: Uint256 returns an Uint256. There is no
/// specific reason for this decision other than the initial use cases we have. If you
/// need a Decimal256 result for the same calculation, use Decimal256(d*u) or Decimal256(u*d).
impl ops::Mul<Decimal256> for Uint256 {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: Decimal256) -> Self::Output {
        // 0*a and b*0 is always 0
        if self.is_zero() || rhs.is_zero() {
            return Uint256::zero();
        }

        self.multiply_ratio(rhs.0, Decimal256::DECIMAL_FRACTIONAL)
    }
}

impl ops::Div<Decimal256> for Uint256 {
    type Output = Self;

    fn div(self, rhs: Decimal256) -> Self::Output {
        assert!(!rhs.is_zero());

        if self.is_zero() {
            return Uint256::zero();
        }

        self.multiply_ratio(Decimal256::DECIMAL_FRACTIONAL, rhs.0)
    }
}

impl ops::Mul<Uint256> for Decimal256 {
    type Output = Uint256;

    fn mul(self, rhs: Uint256) -> Self::Output {
        rhs * self
    }
}

impl Uint256 {
    /// returns self * nom / denom
    pub fn multiply_ratio<A: Into<U256>, B: Into<U256>>(&self, nom: A, denom: B) -> Uint256 {
        let nominator: U256 = nom.into();
        let denominator: U256 = denom.into();
        if denominator.is_zero() {
            panic!("Denominator must not be zero");
        }

        // TODO: minimize rounding that takes place (using gcd algorithm)
        let val = self.0 * nominator / denominator;
        Uint256::from(val)
    }
}

/// Serializes as a base64 string
impl Serialize for Uint256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Deserializes as a base64 string
impl<'de> Deserialize<'de> for Uint256 {
    fn deserialize<D>(deserializer: D) -> Result<Uint256, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Uint256Visitor)
    }
}

struct Uint256Visitor;

impl<'de> de::Visitor<'de> for Uint256Visitor {
    type Value = Uint256;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string-encoded integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match U256::from_dec_str(v) {
            Ok(u) => Ok(Uint256(u)),
            Err(_e) => Err(E::custom(format!("invalid Uint256 '{}'", v))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{from_slice, to_vec, StdResult};
    use std::convert::TryInto;

    #[test]
    fn decimal_one() {
        let value = Decimal256::one();
        assert_eq!(value.0, Decimal256::DECIMAL_FRACTIONAL);
    }

    #[test]
    fn decimal_zero() {
        let value = Decimal256::zero();
        assert_eq!(value.0, U256::zero());
    }

    #[test]
    fn decimal_percent() {
        let value = Decimal256::percent(50);
        assert_eq!(value.0, Decimal256::DECIMAL_FRACTIONAL / 2.into());
    }

    #[test]
    fn decimal_permille() {
        let value = Decimal256::permille(125);
        assert_eq!(value.0, Decimal256::DECIMAL_FRACTIONAL / 8.into());
    }

    #[test]
    fn decimal_from_ratio_works() {
        // 1.0
        assert_eq!(Decimal256::from_ratio(1, 1), Decimal256::one());
        assert_eq!(Decimal256::from_ratio(53, 53), Decimal256::one());
        assert_eq!(Decimal256::from_ratio(125, 125), Decimal256::one());

        // 1.5
        assert_eq!(Decimal256::from_ratio(3, 2), Decimal256::percent(150));
        assert_eq!(Decimal256::from_ratio(150, 100), Decimal256::percent(150));
        assert_eq!(Decimal256::from_ratio(333, 222), Decimal256::percent(150));

        // 0.125
        assert_eq!(Decimal256::from_ratio(1, 8), Decimal256::permille(125));
        assert_eq!(Decimal256::from_ratio(125, 1000), Decimal256::permille(125));

        // 1/3 (result floored)
        assert_eq!(
            Decimal256::from_ratio(1, 3),
            Decimal256(0_333_333_333_333_333_333u64.into())
        );

        // 2/3 (result floored)
        assert_eq!(
            Decimal256::from_ratio(2, 3),
            Decimal256(0_666_666_666_666_666_666u64.into())
        );
    }

    #[test]
    #[should_panic(expected = "Denominator must not be zero")]
    fn decimal_from_ratio_panics_for_zero_denominator() {
        Decimal256::from_ratio(1, 0);
    }

    #[test]
    fn decimal_from_str_works() {
        // Integers
        assert_eq!(Decimal256::from_str("").unwrap(), Decimal256::percent(0));
        assert_eq!(Decimal256::from_str("0").unwrap(), Decimal256::percent(0));
        assert_eq!(Decimal256::from_str("1").unwrap(), Decimal256::percent(100));
        assert_eq!(Decimal256::from_str("5").unwrap(), Decimal256::percent(500));
        assert_eq!(
            Decimal256::from_str("42").unwrap(),
            Decimal256::percent(4200)
        );
        assert_eq!(Decimal256::from_str("000").unwrap(), Decimal256::percent(0));
        assert_eq!(
            Decimal256::from_str("001").unwrap(),
            Decimal256::percent(100)
        );
        assert_eq!(
            Decimal256::from_str("005").unwrap(),
            Decimal256::percent(500)
        );
        assert_eq!(
            Decimal256::from_str("0042").unwrap(),
            Decimal256::percent(4200)
        );

        // Decimal256s
        assert_eq!(
            Decimal256::from_str("1.").unwrap(),
            Decimal256::percent(100)
        );
        assert_eq!(
            Decimal256::from_str("1.0").unwrap(),
            Decimal256::percent(100)
        );
        assert_eq!(
            Decimal256::from_str("1.5").unwrap(),
            Decimal256::percent(150)
        );
        assert_eq!(
            Decimal256::from_str("0.5").unwrap(),
            Decimal256::percent(50)
        );
        assert_eq!(
            Decimal256::from_str("0.123").unwrap(),
            Decimal256::permille(123)
        );

        assert_eq!(
            Decimal256::from_str("40.00").unwrap(),
            Decimal256::percent(4000)
        );
        assert_eq!(
            Decimal256::from_str("04.00").unwrap(),
            Decimal256::percent(0400)
        );
        assert_eq!(
            Decimal256::from_str("00.40").unwrap(),
            Decimal256::percent(0040)
        );
        assert_eq!(
            Decimal256::from_str("00.04").unwrap(),
            Decimal256::percent(0004)
        );

        // Can handle 18 fractional digits
        assert_eq!(
            Decimal256::from_str("7.123456789012345678").unwrap(),
            Decimal256(7123456789012345678u64.into())
        );
        assert_eq!(
            Decimal256::from_str("7.999999999999999999").unwrap(),
            Decimal256(7999999999999999999u64.into())
        );

        // Works for documented max value
        assert_eq!(
            Decimal256::from_str(
                "115792089237316195423570985008687907853269984665640564039457.584007913129639935"
            )
            .unwrap(),
            Decimal256::MAX
        );
    }

    #[test]
    fn decimal_from_str_errors_for_broken_whole_part() {
        match Decimal256::from_str(" ").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Error parsing whole"),
            e => panic!("Unexpected error: {:?}", e),
        }

        match Decimal256::from_str("-1").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Error parsing whole"),
            e => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn decimal_from_str_errors_for_broken_fractinal_part() {
        match Decimal256::from_str("1. ").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Error parsing fractional"),
            e => panic!("Unexpected error: {:?}", e),
        }

        match Decimal256::from_str("1.e").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Error parsing fractional"),
            e => panic!("Unexpected error: {:?}", e),
        }

        match Decimal256::from_str("1.2e3").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Error parsing fractional"),
            e => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn decimal_from_str_errors_for_more_than_18_fractional_digits() {
        match Decimal256::from_str("7.1234567890123456789").unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Cannot parse more than 18 fractional digits")
            }
            e => panic!("Unexpected error: {:?}", e),
        }

        // No special rules for trailing zeros. This could be changed but adds gas cost for the happy path.
        match Decimal256::from_str("7.1230000000000000000").unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Cannot parse more than 18 fractional digits")
            }
            e => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn decimal_from_str_errors_for_invalid_number_of_dots() {
        match Decimal256::from_str("1.2.3").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Unexpected number of dots"),
            e => panic!("Unexpected error: {:?}", e),
        }

        match Decimal256::from_str("1.2.3.4").unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!(msg, "Unexpected number of dots"),
            e => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    #[should_panic(expected = "arithmetic operation overflow")]
    fn decimal_from_str_errors_for_more_than_max_value_integer_part() {
        let _ =
            Decimal256::from_str("115792089237316195423570985008687907853269984665640564039458");
    }

    #[test]
    #[should_panic(expected = "arithmetic operation overflow")]
    fn decimal_from_str_errors_for_more_than_max_value_integer_part_with_decimal() {
        let _ =
            Decimal256::from_str("115792089237316195423570985008687907853269984665640564039458.0");
    }
    #[test]
    #[should_panic(expected = "arithmetic operation overflow")]
    fn decimal_from_str_errors_for_more_than_max_value_decimal_part() {
        let _ = Decimal256::from_str(
            "115792089237316195423570985008687907853269984665640564039457.584007913129639936",
        );
    }

    #[test]
    fn decimal_is_zero_works() {
        assert_eq!(Decimal256::zero().is_zero(), true);
        assert_eq!(Decimal256::percent(0).is_zero(), true);
        assert_eq!(Decimal256::permille(0).is_zero(), true);

        assert_eq!(Decimal256::one().is_zero(), false);
        assert_eq!(Decimal256::percent(123).is_zero(), false);
        assert_eq!(Decimal256::permille(1234).is_zero(), false);
    }

    #[test]
    fn decimal_add() {
        let value = Decimal256::one() + Decimal256::percent(50); // 1.5
        assert_eq!(
            value.0,
            Decimal256::DECIMAL_FRACTIONAL * U256::from(3) / U256::from(2)
        );
    }

    #[test]
    fn decimal_sub() {
        assert_eq!(
            Decimal256::percent(50),
            Decimal256::one() - Decimal256::percent(50)
        );
    }

    #[test]
    fn decimal_mul() {
        assert_eq!(
            Decimal256::percent(25),
            Decimal256::percent(50) * Decimal256::percent(50)
        );
    }

    #[test]
    fn decimal_div() {
        assert_eq!(
            Decimal256::one() + Decimal256::one(),
            Decimal256::percent(50) / Decimal256::percent(25)
        );
    }

    #[test]
    fn decimal_to_string() {
        // Integers
        assert_eq!(Decimal256::zero().to_string(), "0");
        assert_eq!(Decimal256::one().to_string(), "1");
        assert_eq!(Decimal256::percent(500).to_string(), "5");

        // Decimal256s
        assert_eq!(Decimal256::percent(125).to_string(), "1.25");
        assert_eq!(Decimal256::percent(42638).to_string(), "426.38");
        assert_eq!(Decimal256::percent(1).to_string(), "0.01");
        assert_eq!(Decimal256::permille(987).to_string(), "0.987");

        assert_eq!(Decimal256(1u64.into()).to_string(), "0.000000000000000001");
        assert_eq!(Decimal256(10u64.into()).to_string(), "0.00000000000000001");
        assert_eq!(Decimal256(100u64.into()).to_string(), "0.0000000000000001");
        assert_eq!(Decimal256(1000u64.into()).to_string(), "0.000000000000001");
        assert_eq!(Decimal256(10000u64.into()).to_string(), "0.00000000000001");
        assert_eq!(Decimal256(100000u64.into()).to_string(), "0.0000000000001");
        assert_eq!(Decimal256(1000000u64.into()).to_string(), "0.000000000001");
        assert_eq!(Decimal256(10000000u64.into()).to_string(), "0.00000000001");
        assert_eq!(Decimal256(100000000u64.into()).to_string(), "0.0000000001");
        assert_eq!(Decimal256(1000000000u64.into()).to_string(), "0.000000001");
        assert_eq!(Decimal256(10000000000u64.into()).to_string(), "0.00000001");
        assert_eq!(Decimal256(100000000000u64.into()).to_string(), "0.0000001");
        assert_eq!(Decimal256(10000000000000u64.into()).to_string(), "0.00001");
        assert_eq!(Decimal256(100000000000000u64.into()).to_string(), "0.0001");
        assert_eq!(Decimal256(1000000000000000u64.into()).to_string(), "0.001");
        assert_eq!(Decimal256(10000000000000000u64.into()).to_string(), "0.01");
        assert_eq!(Decimal256(100000000000000000u64.into()).to_string(), "0.1");
    }

    #[test]
    fn decimal_serialize() {
        assert_eq!(to_vec(&Decimal256::zero()).unwrap(), br#""0""#);
        assert_eq!(to_vec(&Decimal256::one()).unwrap(), br#""1""#);
        assert_eq!(to_vec(&Decimal256::percent(8)).unwrap(), br#""0.08""#);
        assert_eq!(to_vec(&Decimal256::percent(87)).unwrap(), br#""0.87""#);
        assert_eq!(to_vec(&Decimal256::percent(876)).unwrap(), br#""8.76""#);
        assert_eq!(to_vec(&Decimal256::percent(8765)).unwrap(), br#""87.65""#);
    }

    #[test]
    fn decimal_deserialize() {
        assert_eq!(
            from_slice::<Decimal256>(br#""0""#).unwrap(),
            Decimal256::zero()
        );
        assert_eq!(
            from_slice::<Decimal256>(br#""1""#).unwrap(),
            Decimal256::one()
        );
        assert_eq!(
            from_slice::<Decimal256>(br#""000""#).unwrap(),
            Decimal256::zero()
        );
        assert_eq!(
            from_slice::<Decimal256>(br#""001""#).unwrap(),
            Decimal256::one()
        );

        assert_eq!(
            from_slice::<Decimal256>(br#""0.08""#).unwrap(),
            Decimal256::percent(8)
        );
        assert_eq!(
            from_slice::<Decimal256>(br#""0.87""#).unwrap(),
            Decimal256::percent(87)
        );
        assert_eq!(
            from_slice::<Decimal256>(br#""8.76""#).unwrap(),
            Decimal256::percent(876)
        );
        assert_eq!(
            from_slice::<Decimal256>(br#""87.65""#).unwrap(),
            Decimal256::percent(8765)
        );
    }

    #[test]
    fn to_and_from_uint256() {
        let a: Uint256 = 12345u64.into();
        assert_eq!(U256::from(12345), a.0);
        assert_eq!("12345", a.to_string());

        let a: Uint256 = "34567".try_into().unwrap();
        assert_eq!(U256::from(34567), a.0);
        assert_eq!("34567", a.to_string());

        let a: StdResult<Uint256> = "1.23".try_into();
        assert!(a.is_err());
    }

    #[test]
    fn uint256_is_zero_works() {
        assert_eq!(Uint256::zero().is_zero(), true);
        assert_eq!(Uint256::from(0u64).is_zero(), true);

        assert_eq!(Uint256::from(1u64).is_zero(), false);
        assert_eq!(Uint256::from(123u64).is_zero(), false);
    }

    #[test]
    fn uint256_json() {
        let orig = Uint256::from(1234567890987654321u64);
        let serialized = to_vec(&orig).unwrap();
        assert_eq!(serialized.as_slice(), b"\"1234567890987654321\"");
        let parsed: Uint256 = from_slice(&serialized).unwrap();
        assert_eq!(parsed, orig);
    }

    #[test]
    fn uint256_compare() {
        let a = Uint256::from(12345u64);
        let b = Uint256::from(23456u64);

        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, Uint256::from(12345u64));
    }

    #[test]
    fn uint256_math() {
        let a = Uint256::from(12345u64);
        let b = Uint256::from(23456u64);

        // test + and - for valid values
        assert_eq!(a + b, Uint256::from(35801u64));
        assert_eq!(b - a, Uint256::from(11111u64));

        // test +=
        let mut c = Uint256::from(300000u64);
        c += b;
        assert_eq!(c, Uint256::from(323456u64));
    }
    #[test]
    #[should_panic]
    fn uint256_math_sub_underflow() {
        let _ = Uint256::from(12345u64) - Uint256::from(23456u64);
    }

    #[test]
    #[should_panic]
    fn uint256_math_overflow_panics() {
        // almost_max is 2^256 - 10
        let almost_max = Uint256::from(U256([
            18446744073709551615,
            18446744073709551615,
            18446744073709551615,
            18446744073709551615,
        ]));
        let _ = almost_max + Uint256::from(12u64);
    }

    #[test]
    // in this test the Decimal256 is on the right
    fn uint256_decimal_multiply() {
        // a*b
        let left = Uint256::from(300u64);
        let right = Decimal256::one() + Decimal256::percent(50); // 1.5
        assert_eq!(left * right, Uint256::from(450u64));

        // a*0
        let left = Uint256::from(300u64);
        let right = Decimal256::zero();
        assert_eq!(left * right, Uint256::from(0u64));

        // 0*a
        let left = Uint256::zero();
        let right = Decimal256::one() + Decimal256::percent(50); // 1.5
        assert_eq!(left * right, Uint256::zero());
    }

    #[test]
    fn u256_multiply_ratio_works() {
        let base = Uint256::from(500u64);

        // factor 1/1
        assert_eq!(base.multiply_ratio(1, 1), Uint256::from(500u64));
        assert_eq!(base.multiply_ratio(3, 3), Uint256::from(500u64));
        assert_eq!(base.multiply_ratio(654321, 654321), Uint256::from(500u64));

        // factor 3/2
        assert_eq!(base.multiply_ratio(3, 2), Uint256::from(750u64));
        assert_eq!(base.multiply_ratio(333333, 222222), Uint256::from(750u64));

        // factor 2/3 (integer devision always floors the result)
        assert_eq!(base.multiply_ratio(2, 3), Uint256::from(333u64));
        assert_eq!(base.multiply_ratio(222222, 333333), Uint256::from(333u64));

        // factor 5/6 (integer devision always floors the result)
        assert_eq!(base.multiply_ratio(5, 6), Uint256::from(416u64));
        assert_eq!(base.multiply_ratio(100, 120), Uint256::from(416u64));
    }

    #[test]
    fn u256_from_u128() {
        assert_eq!(Uint256::from(100u64), Uint256::from(100u128));
        let num = Uint256::from(1_000_000_000_000_000_000_000_000u128);
        assert_eq!(num.to_string(), "1000000000000000000000000");
    }

    #[test]
    #[should_panic(expected = "Denominator must not be zero")]
    fn u256_multiply_ratio_panics_for_zero_denominator() {
        Uint256::from(500u64).multiply_ratio(1, 0);
    }

    #[test]
    fn u256_zero_one() {
        assert_eq!(Uint256::zero().0, U256::zero());
        assert_eq!(Uint256::one().0, U256::one());
    }

    #[test]
    fn u256_into_u128() {
        let val: u128 = Uint256::from(1234556700000000000999u128).into();
        assert_eq!(val, 1234556700000000000999u128);
    }

    #[test]
    #[should_panic]
    fn u256_into_u128_panics_for_overflow() {
        let _: u128 = Uint256::from_str("2134982317498312749832174923184732198471983247")
            .unwrap()
            .into();
    }

    #[test]
    // in this test the Decimal256 is on the left
    fn decimal_uint256_multiply() {
        // a*b
        let left = Decimal256::one() + Decimal256::percent(50); // 1.5
        let right = Uint256::from(300u64);
        assert_eq!(left * right, Uint256::from(450u64));

        // 0*a
        let left = Decimal256::zero();
        let right = Uint256::from(300u64);
        assert_eq!(left * right, Uint256::from(0u64));

        // a*0
        let left = Decimal256::one() + Decimal256::percent(50); // 1.5
        let right = Uint256::from(0u64);
        assert_eq!(left * right, Uint256::from(0u64));
    }
}
