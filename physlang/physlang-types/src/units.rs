use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SI base dimensions: L, M, T, I, Θ, N, J
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Dimensions {
    pub l: i32, // length
    pub m: i32, // mass
    pub t: i32, // time
    pub i: i32, // current
    pub th: i32, // temperature (Θ)
    pub n: i32, // amount
    pub j: i32, // luminous intensity
}

impl Dimensions {
    pub const DIMENSIONLESS: Dimensions = Dimensions {
        l: 0,
        m: 0,
        t: 0,
        i: 0,
        th: 0,
        n: 0,
        j: 0,
    };

    pub fn mul(self, other: Dimensions) -> Dimensions {
        Dimensions {
            l: self.l + other.l,
            m: self.m + other.m,
            t: self.t + other.t,
            i: self.i + other.i,
            th: self.th + other.th,
            n: self.n + other.n,
            j: self.j + other.j,
        }
    }

    pub fn div(self, other: Dimensions) -> Dimensions {
        Dimensions {
            l: self.l - other.l,
            m: self.m - other.m,
            t: self.t - other.t,
            i: self.i - other.i,
            th: self.th - other.th,
            n: self.n - other.n,
            j: self.j - other.j,
        }
    }

    pub fn pow(self, exp: i32) -> Dimensions {
        Dimensions {
            l: self.l * exp,
            m: self.m * exp,
            t: self.t * exp,
            i: self.i * exp,
            th: self.th * exp,
            n: self.n * exp,
            j: self.j * exp,
        }
    }

    pub fn is_compatible_with(self, other: Dimensions) -> bool {
        self == other
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        let dims = [
            (self.l, "m"),
            (self.m, "kg"),
            (self.t, "s"),
            (self.i, "A"),
            (self.th, "K"),
            (self.n, "mol"),
            (self.j, "cd"),
        ];
        for (exp, sym) in dims {
            if exp == 0 {
                continue;
            }
            if exp == 1 {
                parts.push(sym.to_string());
            } else {
                parts.push(format!("{sym}^{exp}"));
            }
        }
        if parts.is_empty() {
            "dimensionless".to_string()
        } else {
            parts.join("*")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitSpec {
    pub name: String,
    pub dims: Dimensions,
    pub scale: f64, // multiplier to SI base
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantityType {
    Scalar,
    Named(String),
    Quantity {
        value_type: String,
        dims: Dimensions,
    },
    Array {
        elem: Box<QuantityType>,
        len: u32,
    },
    QReg { size: u32 },
    Gate,
    Circuit,
    Hamiltonian,
    Observable,
    Energy,
    Angle,
    Result,
    Void,
    Bool,
    Int,
    Float,
    String,
}

impl QuantityType {
    pub fn dims(&self) -> Option<Dimensions> {
        match self {
            QuantityType::Quantity { dims, .. } => Some(*dims),
            QuantityType::Energy => Some(Dimensions {
                l: 2,
                m: 1,
                t: -2,
                ..Default::default()
            }),
            QuantityType::Angle => Some(Dimensions::DIMENSIONLESS),
            _ => None,
        }
    }
}

pub struct UnitRegistry {
    units: HashMap<String, UnitSpec>,
    named_types: HashMap<String, QuantityType>,
}

impl Default for UnitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            units: HashMap::new(),
            named_types: HashMap::new(),
        };
        reg.register_si_units();
        reg.register_named_types();
        reg
    }

    fn register_si_units(&mut self) {
        let mut add = |name: &str, dims: Dimensions, scale: f64| {
            self.units.insert(
                name.to_string(),
                UnitSpec {
                    name: name.to_string(),
                    dims,
                    scale,
                },
            );
        };
        add("m", Dimensions { l: 1, ..Default::default() }, 1.0);
        add("km", Dimensions { l: 1, ..Default::default() }, 1000.0);
        add("kg", Dimensions { m: 1, ..Default::default() }, 1.0);
        add("g", Dimensions { m: 1, ..Default::default() }, 0.001);
        add("s", Dimensions { t: 1, ..Default::default() }, 1.0);
        add("A", Dimensions { i: 1, ..Default::default() }, 1.0);
        add("K", Dimensions { th: 1, ..Default::default() }, 1.0);
        add("mol", Dimensions { n: 1, ..Default::default() }, 1.0);
        add("cd", Dimensions { j: 1, ..Default::default() }, 1.0);
        add("N", Dimensions { l: 1, m: 1, t: -2, ..Default::default() }, 1.0);
        add("J", Dimensions { l: 2, m: 1, t: -2, ..Default::default() }, 1.0);
        add("Pa", Dimensions { l: -1, m: 1, t: -2, ..Default::default() }, 1.0);
        add("W", Dimensions { l: 2, m: 1, t: -3, ..Default::default() }, 1.0);
    }

    fn register_named_types(&mut self) {
        let types = [
            ("Int", QuantityType::Int),
            ("Float", QuantityType::Float),
            ("Bool", QuantityType::Bool),
            ("String", QuantityType::String),
            ("Void", QuantityType::Void),
            ("Velocity", QuantityType::Named("Velocity".into())),
            ("Force", QuantityType::Named("Force".into())),
            ("Mass", QuantityType::Named("Mass".into())),
            ("Energy", QuantityType::Energy),
            ("Action", QuantityType::Quantity {
                value_type: "Action".into(),
                dims: Dimensions { l: 2, m: 1, t: -1, ..Default::default() },
            }),
            ("Angle", QuantityType::Angle),
            ("Qubit", QuantityType::Named("Qubit".into())),
            ("Gate", QuantityType::Gate),
            ("Circuit", QuantityType::Circuit),
            ("Hamiltonian", QuantityType::Hamiltonian),
            ("Observable", QuantityType::Observable),
            ("Result", QuantityType::Result),
        ];
        for (name, ty) in types {
            self.named_types.insert(name.to_string(), ty);
        }
    }

    pub fn resolve_unit_factors(
        &self,
        factors: &[physlang_parser::UnitFactor],
    ) -> Option<Dimensions> {
        let mut dims = Dimensions::DIMENSIONLESS;
        for f in factors {
            let spec = self.units.get(&f.ident)?;
            dims = dims.mul(spec.dims.pow(f.power));
        }
        Some(dims)
    }

    pub fn named_type(&self, name: &str) -> Option<&QuantityType> {
        self.named_types.get(name)
    }

    pub fn type_for_name(&self, name: &str) -> QuantityType {
        self.named_types
            .get(name)
            .cloned()
            .unwrap_or(QuantityType::Named(name.to_string()))
    }

    pub fn dims_for_named(&self, name: &str) -> Option<Dimensions> {
        match name {
            "Velocity" => Some(Dimensions { l: 1, t: -1, ..Default::default() }),
            "Force" => Some(Dimensions { l: 1, m: 1, t: -2, ..Default::default() }),
            "Mass" => Some(Dimensions { m: 1, ..Default::default() }),
            "Pressure" => Some(Dimensions { l: -1, m: 1, t: -2, ..Default::default() }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use physlang_parser::UnitFactor;

    #[test]
    fn unit_algebra_m_s() {
        let reg = UnitRegistry::new();
        let dims = reg
            .resolve_unit_factors(&[
                UnitFactor { ident: "m".into(), power: 1 },
                UnitFactor { ident: "s".into(), power: -1 },
            ])
            .unwrap();
        assert_eq!(reg.dims_for_named("Velocity").unwrap(), dims);
    }

    #[test]
    fn force_ne_mass_plus_velocity() {
        let reg = UnitRegistry::new();
        let force = reg.dims_for_named("Force").unwrap();
        let mass = reg.dims_for_named("Mass").unwrap();
        let velocity = reg.dims_for_named("Velocity").unwrap();
        assert_ne!(force, mass);
        assert_ne!(force, velocity);
        assert_ne!(mass, velocity);
    }
}
