//! Система гигантских чисел для инкрементал-игры.
//!
//! Число хранится как мантисса `m ∈ [1, 10)` и показатель `e` (i64):
//! значение = m * 10^e. Это позволяет представлять числа порядка 1e1000
//! и далеко за его пределами без потери точности там, где она не нужна.
//!
//! Формат отображения — "буквенный" (краткий), как в Cookie Clicker / AdVenture Capitalist:
//!   1_000_000 -> "1M", 1_000_000_000 -> "1B", 1e33 -> "10^33" (потом можно добавитьdecillion).

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct BigNum {
    /// мантисса, всегда нормализована в [1, 10); для нуля — 0
    pub m: f64,
    /// десятичный показатель степени
    pub e: i64,
}

/// Нулевое число (отдельная константа, т.к. нормализация требует m != 0)
pub const ZERO: BigNum = BigNum { m: 0.0, e: 0 };

impl BigNum {
    /// Создать из f64 с нормализацией
    pub fn from_f64(v: f64) -> Self {
        if v == 0.0 || !v.is_finite() {
            return ZERO;
        }
        let mut m = v.abs();
        let mut e = 0i64;
        // нормализуем
        while m >= 10.0 {
            m /= 10.0;
            e += 1;
        }
        while m < 1.0 && m > 0.0 {
            m *= 10.0;
            e -= 1;
        }
        BigNum { m, e }
    }

    pub const fn raw(m: f64, e: i64) -> Self {
        BigNum { m, e }
    }

    pub fn is_zero(&self) -> bool {
        self.m == 0.0
    }

    /// Привести к f64 (для отрисовки/сравнения с небольшими значениями).
    /// При переполнении возвращает f64::INFINITY.
    pub fn to_f64(self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        // используем экспоненту, а не powi(i32): показатель может быть огромным,
        // результат при переполнении — корректный infinity
        self.m * 10f64.powf(self.e as f64)
    }

    /// Нормализовать после операции (беречься денормалов f64)
    #[inline]
    fn norm(mut self) -> Self {
        if self.m == 0.0 {
            return ZERO;
        }
        if self.m >= 10.0 || self.m < 1.0 {
            let shift = self.m.log10().floor() as i64;
            // 10^shift может не влезть в powi(i32) — дели итеративно при большом сдвиге
            if shift.abs() <= 300 {
                self.m /= 10f64.powi(shift as i32);
            } else {
                while self.m >= 10.0 {
                    self.m /= 10.0;
                }
            }
            self.e += shift;
            // на случай очень больших сдвигов
            while self.m >= 10.0 {
                self.m /= 10.0;
                self.e += 1;
            }
            while self.m < 1.0 && self.m > 0.0 {
                self.m *= 10.0;
                self.e -= 1;
            }
        }
        self
    }

    /// a - b, результат не может быть отрицательным (запирается в 0)
    pub fn sub_saturating(self, other: Self) -> Self {
        if other.is_zero() {
            return self;
        }
        if self < other {
            return ZERO;
        }
        // разность степеней мала — считаем в f64 напрямую
        let diff_e = self.e - other.e;
        if diff_e <= 15 {
            let a = self.to_f64();
            let b = other.to_f64();
            return BigNum::from_f64(a - b);
        }
        // иначе other пренебрежимо мал по сравнению с self
        self
    }

    /// Сколько целых единиц `unit` помещается в self (для покупок xN)
    pub fn div_floor_to_u32(self, unit: Self) -> u32 {
        if unit.is_zero() || self < unit {
            return 0;
        }
        let q = (self.m / unit.m) * 10f64.powf((self.e - unit.e) as f64);
        if q >= u32::MAX as f64 {
            return u32::MAX;
        }
        q.floor().max(0.0) as u32
    }

    /// Отношение self/other как f64 (для подсказок "через N секунд")
    pub fn ratio(self, other: Self) -> f64 {
        if other.is_zero() {
            return f64::INFINITY;
        }
        (self.m / other.m) * 10f64.powf((self.e - other.e) as f64)
    }

    /// Буквенное форматирование: K, M, B, T, Qa, Qi, Sx, Sp, Oc, No, Dc...
    /// После Dc (1e33) переходим на научную запись "1.23e36".
    pub fn format_short(self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        // маленькие числа печатаем как целые
        if self.e < 0 {
            return "0".to_string();
        }
        if self.e < 3 {
            return format!("{}", (self.m * 10f64.powf(self.e as f64)).round());
        }

        let suffixes: [&str; 11] = [
            "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc",
        ];
        let tier = self.e / 3; // 1 -> K, 2 -> M, 3 -> B ...
        if tier >= 1 && (tier as usize) <= suffixes.len() {
            let scale = self.e - tier * 3; // 0..2
            let mant = self.m * 10f64.powf(scale as f64);
            let s = if mant >= 100.0 {
                format!("{:.0}", mant)
            } else {
                trim_zeros(format!("{:.2}", mant))
            };
            return format!("{}{}", s, suffixes[tier as usize - 1]);
        }
        // сверх всех суффиксов — научная запись
        trim_zeros(format!("{:.2}e{}", self.m, self.e))
    }

    /// self^n для целого n (для степеней множителей x2^n, x3^n...)
    pub fn pow_u32(self, n: u32) -> BigNum {
        if self.is_zero() {
            return ZERO;
        }
        if n == 0 {
            return ONE;
        }
        let mut m = self.m.powf(n as f64);
        let mut e = self.e * n as i64;
        if !m.is_finite() {
            // мантисса ушла в inf — переносим через логарифм
            let lg = n as f64 * (self.m.log10() + self.e as f64);
            e = lg.floor() as i64;
            m = 10f64.powf(lg - e as f64);
        }
        BigNum::raw(m, e).norm()
    }

    /// Квадратный корень (для формулы перерождения)
    pub fn sqrt(self) -> BigNum {
        if self.is_zero() {
            return ZERO;
        }
        // чтобы показатель стал чётным, при нечётном e забираем один десяток в мантиссу
        let (m, e) = if self.e % 2 != 0 {
            (self.m * 10.0, self.e - 1)
        } else {
            (self.m, self.e)
        };
        BigNum::raw(m.sqrt(), e / 2).norm()
    }

    /// То же, но всегда с дробной частью полезно для "в секунду"
    pub fn format_rate(self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        if self.e < 3 {
            let v = self.to_f64();
            if v.fract() == 0.0 {
                return format!("{}", v as i64);
            }
            return trim_zeros(format!("{:.1}", v));
        }
        self.format_short()
    }
}

fn trim_zeros(s: String) -> String {
    if s.contains('.') {
        let t = s.trim_end_matches('0').trim_end_matches('.');
        t.to_string()
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Арифметика и сравнения
// ---------------------------------------------------------------------------

impl Add for BigNum {
    type Output = BigNum;
    fn add(self, rhs: Self) -> BigNum {
        if rhs.is_zero() {
            return self;
        }
        if self.is_zero() {
            return rhs;
        }
        let (big, small) = if self.e >= rhs.e { (self, rhs) } else { (rhs, self) };
        let diff = big.e - small.e;
        if diff > 15 {
            return big; // малое слагаемое теряется в точности f64 — ок для кликера
        }
        // diff может быть i64, а powi принимает i32 — конвертируем безопасно
        BigNum::raw(big.m + small.m / 10f64.powf(diff as f64), big.e).norm()
    }
}

impl Div for BigNum {
    type Output = BigNum;
    fn div(self, rhs: Self) -> BigNum {
        if rhs.is_zero() {
            return ZERO;
        }
        if self.is_zero() {
            return ZERO;
        }
        BigNum::raw(self.m / rhs.m, self.e - rhs.e).norm()
    }
}

impl AddAssign for BigNum {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Mul for BigNum {
    type Output = BigNum;
    fn mul(self, rhs: Self) -> BigNum {
        if self.is_zero() || rhs.is_zero() {
            return ZERO;
        }
        BigNum::raw(self.m * rhs.m, self.e + rhs.e).norm()
    }
}

impl Mul<f64> for BigNum {
    type Output = BigNum;
    fn mul(self, rhs: f64) -> BigNum {
        if self.is_zero() || rhs == 0.0 {
            return ZERO;
        }
        BigNum::raw(self.m * rhs, self.e).norm()
    }
}

impl MulAssign<f64> for BigNum {
    fn mul_assign(&mut self, rhs: f64) {
        *self = *self * rhs;
    }
}

impl PartialOrd for BigNum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigNum {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let z1 = self.is_zero();
        let z2 = other.is_zero();
        if z1 && z2 {
            return std::cmp::Ordering::Equal;
        }
        if z1 {
            return std::cmp::Ordering::Less;
        }
        if z2 {
            return std::cmp::Ordering::Greater;
        }
        self.e.cmp(&other.e).then(self.m.partial_cmp(&other.m).unwrap())
    }
}

impl PartialEq for BigNum {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}
impl Eq for BigNum {}

impl Default for BigNum {
    fn default() -> Self {
        ZERO
    }
}

pub const ONE: BigNum = BigNum { m: 1.0, e: 0 };

/// Удобные литералы
pub fn bn(v: f64) -> BigNum {
    BigNum::from_f64(v)
}

/// 10^n
pub fn pow10(n: i64) -> BigNum {
    BigNum { m: 1.0, e: n }
}

// ---------------------------------------------------------------------------
// Тесты — самая важная подсистема игры, проверяем тщательно
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_basic() {
        assert_eq!(bn(0.0).format_short(), "0");
        assert_eq!(bn(7.0).format_short(), "7");
        assert_eq!(bn(999.0).format_short(), "999");
        assert_eq!(bn(1_000.0).format_short(), "1K");
        assert_eq!(bn(1_234_567.0).format_short(), "1.23M");
        assert_eq!(bn(1_000_000_000.0).format_short(), "1B"); // миллиард лапок!
        assert_eq!(bn(12_500_000_000.0).format_short(), "12.5B");
        assert_eq!(pow10(33).format_short(), "1Dc");
        assert_eq!(pow10(36).format_short(), "1e36");
        assert_eq!(pow10(1000).format_short(), "1e1000");
    }

    #[test]
    fn arithmetic() {
        assert_eq!(bn(1e9) + bn(1e9), bn(2e9));
        assert_eq!(bn(1e300) + bn(1.0), bn(1e300)); // мелочь теряется — это нормально
        assert_eq!(bn(1e9) * bn(1e9), bn(1e18));
        assert_eq!(pow10(500) * pow10(500), pow10(1000));
        assert_eq!(bn(5.0) * 3.0, bn(15.0));
        assert_eq!(bn(1e10).sub_saturating(bn(1e9)), bn(9e9));
        assert_eq!(bn(1.0).sub_saturating(bn(5.0)), ZERO);
    }

    #[test]
    fn ordering_and_div() {
        assert!(bn(1e9) > bn(999_999_999.0));
        assert!(pow10(1000) > pow10(999));
        assert_eq!(bn(1e9).div_floor_to_u32(bn(1e6)), 1000);
        assert_eq!(ZERO.div_floor_to_u32(bn(10.0)), 0);
    }

    #[test]
    fn huge_numbers_survive() {
        let mut x = pow10(1000);
        x += pow10(999); // +10%
        assert_eq!(x.e, 1000);
        assert!((x.m - 1.1).abs() < 1e-10);
        assert_eq!(x.format_short(), "1.1e1000");
    }
}
