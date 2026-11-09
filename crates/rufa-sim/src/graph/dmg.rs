//! A damage vector: the four types plus breacher damage that ignores resists.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dmg {
    pub em: f64,
    pub thermal: f64,
    pub kinetic: f64,
    pub explosive: f64,
    pub pure: f64,
}

impl Dmg {
    pub fn types(parts: [f64; 4]) -> Dmg {
        Dmg {
            em: parts[0],
            thermal: parts[1],
            kinetic: parts[2],
            explosive: parts[3],
            pure: 0.0,
        }
    }

    pub fn total(self) -> f64 {
        self.em + self.thermal + self.kinetic + self.explosive + self.pure
    }

    pub fn scale(self, f: f64) -> Dmg {
        Dmg {
            em: self.em * f,
            thermal: self.thermal * f,
            kinetic: self.kinetic * f,
            explosive: self.explosive * f,
            pure: self.pure * f,
        }
    }

    pub fn plus(self, other: Dmg) -> Dmg {
        Dmg {
            em: self.em + other.em,
            thermal: self.thermal + other.thermal,
            kinetic: self.kinetic + other.kinetic,
            explosive: self.explosive + other.explosive,
            pure: self.pure + other.pure,
        }
    }

    /// What gets through the given resists. Pure damage always does.
    pub fn applied(self, resists: [f64; 4]) -> f64 {
        self.em * (1.0 - resists[0])
            + self.thermal * (1.0 - resists[1])
            + self.kinetic * (1.0 - resists[2])
            + self.explosive * (1.0 - resists[3])
            + self.pure
    }

    /// Spreads a total over the four types in the given shares.
    pub fn split(total: f64, shares: [f64; 4]) -> Dmg {
        let sum: f64 = shares.iter().sum();
        if sum <= 0.0 {
            return Dmg::types([total / 4.0; 4]);
        }
        Dmg::types([
            total * shares[0] / sum,
            total * shares[1] / sum,
            total * shares[2] / sum,
            total * shares[3] / sum,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resists_take_their_share_but_never_the_pure_part() {
        let d = Dmg {
            em: 10.0,
            thermal: 10.0,
            kinetic: 10.0,
            explosive: 10.0,
            pure: 5.0,
        };
        assert_eq!(d.applied([0.5; 4]), 25.0);
        assert_eq!(d.applied([0.0; 4]), 45.0);
        assert_eq!(d.scale(2.0).total(), 90.0);
    }

    #[test]
    fn split_follows_the_shares() {
        let d = Dmg::split(100.0, [1.0, 3.0, 0.0, 0.0]);
        assert_eq!((d.em, d.thermal, d.kinetic), (25.0, 75.0, 0.0));
        assert_eq!(Dmg::split(100.0, [0.0; 4]).em, 25.0);
    }
}
