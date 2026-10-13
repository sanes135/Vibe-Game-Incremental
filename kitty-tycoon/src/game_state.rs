//! Игровое состояние: ресурсы, улучшения, перерождение.

use crate::big_num::{bn, pow10, BigNum, ZERO};
use serde::{Deserialize, Serialize};

/// Порог перерождения: 1 миллиард лапок (1e9)
pub const PRESTIGE_THRESHOLD: BigNum = pow10(9);

#[derive(Clone, Copy, Debug)]
pub struct UpgradeDef {
    /// отображаемое имя (мемное)
    pub name: &'static str,
    /// описание-мем
    pub desc: &'static str,
    /// базовая цена в лапках
    pub base_cost: f64,
    /// множитель цены за каждую купленную единицу
    pub cost_growth: f64,
    /// что даёт улучшение
    pub effect: Effect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// +X лапок за клик
    PerClick(f64),
    /// умножает силу клика на X
    ClickMult(f64),
    /// +X лапок в секунду (пассивно)
    PerSecond(f64),
    /// умножает весь доход (клик и пассивный) на X
    AllMult(f64),
    /// автоклик: X кликов в секунду (кошачье улучшение)
    AutoClick(f64),
}

/// Все улучшения за лапки (обычный магазин). Цены растут геометрически.
pub const UPGRADES: &[UpgradeDef] = &[
    UpgradeDef {
        name: "Шершавый язык",
        desc: "+1 к силе глажения. Каждый лизун — плюс к карме.",
        base_cost: 15.0,
        cost_growth: 1.15,
        effect: Effect::PerClick(1.0),
    },
    UpgradeDef {
        name: "Митенки хозяйки",
        desc: "+3 лапки за каждое поглаживание. Мяу-стиль!",
        base_cost: 120.0,
        cost_growth: 1.17,
        effect: Effect::PerClick(3.0),
    },
    UpgradeDef {
        name: "Валенок для мурчания",
        desc: "+5 лапок/сек. Кот мурчит в тапке, пока ты AFK.",
        base_cost: 600.0,
        cost_growth: 1.18,
        effect: Effect::PerSecond(5.0),
    },
    UpgradeDef {
        name: "Пельмень дружбы",
        desc: "x2 ко всем лапкам. Сибирский кот делится пельменём.",
        base_cost: 5_000.0,
        cost_growth: 1.35,
        effect: Effect::AllMult(2.0),
    },
    UpgradeDef {
        name: "Мем «Ждун»",
        desc: "+250 лапок/сек. Он ждёт. Он всегда ждал лапки.",
        base_cost: 30_000.0,
        cost_growth: 1.2,
        effect: Effect::PerSecond(250.0),
    },
    UpgradeDef {
        name: "Картонная коробка",
        desc: "x2 ко всем лапкам. Если кот в коробке — он доволен.",
        base_cost: 250_000.0,
        cost_growth: 1.4,
        effect: Effect::AllMult(2.0),
    },
    UpgradeDef {
        name: "Энергетик «Котэ»",
        desc: "+1500 лапок за клик. Ночь — не повод спать.",
        base_cost: 1_500_000.0,
        cost_growth: 1.22,
        effect: Effect::PerClick(1_500.0),
    },
    UpgradeDef {
        name: "Фабрика шёлкового мячика",
        desc: "+50K лапок/сек. Промышленный масштаб мурчания.",
        base_cost: 20_000_000.0,
        cost_growth: 1.25,
        effect: Effect::PerSecond(50_000.0),
    },
    UpgradeDef {
        name: "Гиперлуп «Ня»",
        desc: "x3 ко всем лапкам. Мурчание на релятивистских скоростях.",
        base_cost: 400_000_000.0,
        cost_growth: 1.5,
        effect: Effect::AllMult(3.0),
    },
];

/// Улучшения за кошек (магазин перерождения). Покупаются один раз, стоят N кошек.
pub const CAT_UPGRADES: &[UpgradeDef] = &[
    UpgradeDef {
        name: "Окорок ветчины",
        desc: "x1000 к силе клика. Кошка бежит быстрее времени.",
        base_cost: 1.0,
        cost_growth: 1.0,
        effect: Effect::ClickMult(1000.0),
    },
    UpgradeDef {
        name: "Квантовая миска",
        desc: "x100 ко всем лапкам. Корм существует во всех вселенных.",
        base_cost: 2.0,
        cost_growth: 1.0,
        effect: Effect::AllMult(100.0),
    },
    UpgradeDef {
        name: "Автоматическая рука",
        desc: "Автогладит 10 раз/сек. Тебе больше не нужно стараться.",
        base_cost: 3.0,
        cost_growth: 1.0,
        effect: Effect::AutoClick(10.0),
    },
    UpgradeDef {
        name: "Чёрный ход в Matrix",
        desc: "x1e6 ко всем лапкам. Числа становятся вкуснее.",
        base_cost: 5.0,
        cost_growth: 1.0,
        effect: Effect::AllMult(1e6),
    },
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameState {
    /// текущие лапки
    pub paws: BigNum,
    /// всего лапок заработано за текущий забег (база для перерождения)
    pub total_paws_run: BigNum,
    /// количество покупок каждого улучшения из UPGRADES
    pub owned: Vec<u32>,
    /// кошки — валюта перерождения
    pub cats: u32,
    /// сколько раз перерождались (+10% к доходу за каждое)
    pub prestige_count: u32,
    /// какие кошачьи улучшения куплены (индексы в CAT_UPGRADES)
    pub cat_upgrades_bought: Vec<bool>,
}

impl GameState {
    /// Полностью сбросить прогресс (для кнопки hard reset)
    pub fn hard_reset(&mut self) {
        *self = GameState::default();
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            paws: ZERO,
            total_paws_run: ZERO,
            owned: vec![0; UPGRADES.len()],
            cats: 0,
            prestige_count: 0,
            cat_upgrades_bought: vec![false; CAT_UPGRADES.len()],
        }
    }
}

impl GameState {
    /// Цена i-го улучшения при текущем количестве покупок: base * growth^n
    pub fn upgrade_cost(&self, idx: usize) -> BigNum {
        let def = &UPGRADES[idx];
        bn(def.base_cost * def.cost_growth.powf(self.owned[idx] as f64))
    }

    /// Общая стоимость покупки N штук подряд (сумма геометрической прогрессии)
    pub fn bulk_cost(&self, idx: usize, n: u32) -> BigNum {
        let def = &UPGRADES[idx];
        let g = def.cost_growth;
        let first = def.base_cost * g.powf(self.owned[idx] as f64);
        let total = if (g - 1.0).abs() < 1e-9 {
            first * n as f64
        } else {
            first * (g.powf(n as f64) - 1.0) / (g - 1.0)
        };
        bn(total)
    }

    /// Сколько максимум штук можно купить (ограниченно, чтобы не ловить оверфлоу)
    pub fn max_affordable(&self, idx: usize, cap: u32) -> u32 {
        let mut n = 0u32;
        while n < cap {
            let c = self.bulk_cost(idx, n + 1);
            if self.paws >= c {
                n += 1;
            } else {
                break;
            }
        }
        n
    }

    /// Общий множитель "ко всем лапкам": перерождения + AllMult-улучшения
    fn all_mult(&self) -> BigNum {
        // +10% за каждое перерождение
        let mut mult = bn(1.0 + 0.1 * self.prestige_count as f64);
        for (i, &n) in self.owned.iter().enumerate() {
            if let Effect::AllMult(v) = UPGRADES[i].effect {
                mult *= bn(v).pow_u32(n);
            }
        }
        for (i, &bought) in self.cat_upgrades_bought.iter().enumerate() {
            if bought {
                if let Effect::AllMult(v) = CAT_UPGRADES[i].effect {
                    mult *= bn(v);
                }
            }
        }
        mult
    }

    /// Сила одного клика (лапок за поглаживание)
    pub fn click_power(&self) -> BigNum {
        let mut per_click = 1.0f64;
        let mut click_mult = 1.0f64;
        for (i, &n) in self.owned.iter().enumerate() {
            if n == 0 {
                continue;
            }
            match UPGRADES[i].effect {
                Effect::PerClick(v) => per_click += v * n as f64,
                _ => {}
            }
        }
        for (i, &bought) in self.cat_upgrades_bought.iter().enumerate() {
            if bought {
                if let Effect::ClickMult(v) = CAT_UPGRADES[i].effect {
                    click_mult *= v;
                }
            }
        }
        bn(per_click * click_mult) * self.all_mult()
    }

    /// Пассивный доход, лапок/сек (без автокликера)
    pub fn pps(&self) -> BigNum {
        let mut base = 0.0f64;
        for (i, &n) in self.owned.iter().enumerate() {
            if let Effect::PerSecond(v) = UPGRADES[i].effect {
                base += v * n as f64;
            }
        }
        bn(base) * self.all_mult()
    }

    /// Скорость автокликера (кликов/сек), если куплен
    pub fn auto_clicks_per_sec(&self) -> f64 {
        let mut cps = 0.0;
        for (i, &bought) in self.cat_upgrades_bought.iter().enumerate() {
            if bought {
                if let Effect::AutoClick(v) = CAT_UPGRADES[i].effect {
                    cps += v;
                }
            }
        }
        cps
    }

    /// Полная скорость дохода, лапок/сек (пассив + автокликер)
    pub fn total_rate(&self) -> BigNum {
        let acps = self.auto_clicks_per_sec();
        self.pps() + self.click_power() * acps
    }

    /// Попытка купить N штук улучшения за лапки
    pub fn try_buy(&mut self, idx: usize, n: u32) -> bool {
        if n == 0 {
            return false;
        }
        let cost = self.bulk_cost(idx, n);
        if self.paws >= cost {
            self.paws = self.paws.sub_saturating(cost);
            self.owned[idx] += n;
            true
        } else {
            false
        }
    }

    /// Цена кошачьего улучшения (в кошках)
    pub fn cat_upgrade_price(&self, idx: usize) -> u32 {
        CAT_UPGRADES[idx].base_cost as u32
    }

    /// Попытка купить кошачье улучшение
    pub fn try_buy_cat_upgrade(&mut self, idx: usize) -> bool {
        let price = self.cat_upgrade_price(idx);
        if !self.cat_upgrades_bought[idx] && self.cats >= price {
            self.cats -= price;
            self.cat_upgrades_bought[idx] = true;
            true
        } else {
            false
        }
    }

    /// Сколько кошек даст перерождение: floor(sqrt(total_за_забег / 1e9))
    pub fn prestige_gain(&self) -> u32 {
        let r = (self.total_paws_run / PRESTIGE_THRESHOLD).sqrt().to_f64();
        if r >= u32::MAX as f64 {
            u32::MAX
        } else {
            r.floor().max(0.0) as u32
        }
    }

    pub fn can_prestige(&self) -> bool {
        self.prestige_gain() >= 1
    }

    /// Перерождение: сбег забега, +N кошек, +10% к доходу навечно
    pub fn do_prestige(&mut self) -> bool {
        let gain = self.prestige_gain();
        if gain < 1 {
            return false;
        }
        self.cats += gain;
        self.prestige_count += 1;
        self.paws = ZERO;
        self.total_paws_run = ZERO;
        self.owned.iter_mut().for_each(|n| *n = 0);
        true
    }

    /// Начислить лапки (текущие и счётчик забега)
    pub fn earn(&mut self, amount: BigNum) {
        self.paws += amount;
        self.total_paws_run += amount;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_grow_geometrically() {
        let mut s = GameState::default();
        let c0 = s.upgrade_cost(0); // 15
        assert_eq!(c0.format_short(), "15");
        s.owned[0] = 10;
        let c10 = s.upgrade_cost(0); // 15 * 1.15^10 ≈ 60.7
        assert!(c10 > bn(60.0) && c10 < bn(61.0));
    }

    #[test]
    fn buy_and_income() {
        let mut s = GameState::default();
        s.paws = bn(15.0);
        assert!(s.try_buy(0, 1));
        assert_eq!(s.paws.to_f64(), 0.0);
        assert_eq!(s.click_power().format_short(), "2"); // 1+1
        // x5 покупка с накоплением
        s.paws = bn(1e9);
        let n = s.max_affordable(0, 100);
        assert!(n >= 40); // много должно влезть
        assert!(s.try_buy(0, n));
    }

    #[test]
    fn prestige_math() {
        let mut s = GameState::default();
        s.total_paws_run = bn(1e9);
        assert_eq!(s.prestige_gain(), 1);
        s.total_paws_run = bn(1e11); // sqrt(100) = 10
        assert_eq!(s.prestige_gain(), 10);
        assert!(s.do_prestige());
        assert_eq!(s.cats, 10);
        assert_eq!(s.paws, ZERO);
        assert_eq!(s.prestige_count, 1);
        // бонус +10% к клику
        assert_eq!(s.click_power().format_short(), "1.1");
    }

    #[test]
    fn cat_upgrades_work() {
        let mut s = GameState::default();
        s.cats = 10;
        assert!(s.try_buy_cat_upgrade(0)); // окорок: x1000 к клику
        // prestige_count = 0, множитель x1.0, базовый клик 1 -> 1000
        assert_eq!(s.click_power().format_short(), "1K");
        assert_eq!(s.auto_clicks_per_sec(), 0.0);
        assert!(s.try_buy_cat_upgrade(2)); // автокликер
        assert_eq!(s.auto_clicks_per_sec(), 10.0);
        // полная скорость = 10 кликов * 1000 лапок = 10K/сек
        assert_eq!(s.total_rate().format_short(), "10K");
    }

    #[test]
    fn huge_numbers_in_game() {
        // имитируем позднюю стадию: множество множителей x2/x3
        let mut s = GameState::default();
        s.owned[3] = 50; // x2^50
        s.owned[8] = 30; // x3^30
        let rate = s.pps();
        // сила клика ~ 2^50 * 3^30 ≈ 1.13e22
        let cp = s.click_power();
        assert!(cp.e >= 21 && cp.e <= 23, "click_power e={}", cp.e);
        assert!(cp.format_short().contains('e'), "{}", cp.format_short());
        let _ = rate;
    }
}
