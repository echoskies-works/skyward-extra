//! The calculator the desktop opens: the sum it holds and the keys that
//! write it. The page that shows it is in `pages`.

/// The four sums the calculator works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl CalcOp {
    /// The sum worked, or nothing where it cannot be: a division by zero,
    /// or one whose answer runs past what a number can hold.
    fn apply(self, a: f64, b: f64) -> Option<f64> {
        let r = match self {
            CalcOp::Add => a + b,
            CalcOp::Sub => a - b,
            CalcOp::Mul => a * b,
            CalcOp::Div if b == 0.0 => return None,
            CalcOp::Div => a / b,
        };
        r.is_finite().then_some(r)
    }
}

/// A key of the calculator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcKey {
    Digit(u8),
    Dot,
    /// Flip the sign of the number typed.
    Negate,
    /// Take a hundredth of the number typed.
    Percent,
    Op(CalcOp),
    Eq,
    /// Start the sum over.
    Clear,
    /// Rub out the last figure typed.
    Erase,
}

/// How many figures the display takes before the keys stop adding to them.
const CALC_FIGURES: usize = 12;

/// A number as the calculator's display reads it: no trailing zeros, and
/// in an exponent once it is too long for the line.
fn calc_number(v: f64) -> String {
    if !v.is_finite() {
        return "Error".to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    if v.abs() >= 1e12 || v.abs() < 1e-9 {
        return format!("{v:e}");
    }
    let s = format!("{v:.8}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "0".to_string() } else { s.to_string() }
}

/// The calculator's sum so far: the number being typed, the number the
/// operator waiting works on, and the answer last reached.
#[derive(Clone, Debug, PartialEq)]
pub struct Calc {
    /// The number being typed, as the keys wrote it; empty before the
    /// first key is pressed.
    pub entry: String,
    /// The number the operator waiting works on.
    pub acc: Option<f64>,
    /// The operator waiting for the number typed next.
    pub op: Option<CalcOp>,
    /// The keys start a new number, as they do after `=` and after an
    /// operator.
    pub fresh: bool,
    /// The sum stopped, as it was divided by zero; the next key starts a
    /// new one.
    pub broken: bool,
}

impl Default for Calc {
    fn default() -> Self {
        Calc { entry: String::new(), acc: None, op: None, fresh: true, broken: false }
    }
}

impl Calc {
    /// What the display reads.
    pub fn shown(&self) -> String {
        if self.broken {
            return "Error".to_string();
        }
        if self.entry.is_empty() {
            return "0".to_string();
        }
        self.entry.clone()
    }

    /// The number typed so far.
    fn value(&self) -> f64 {
        if self.entry.is_empty() { 0.0 } else { self.entry.parse().unwrap_or(0.0) }
    }

    /// Carry out a tapped key.
    pub fn press(&mut self, key: CalcKey) {
        if self.broken {
            *self = Calc::default();
            if matches!(key, CalcKey::Clear | CalcKey::Erase) {
                return;
            }
        }
        match key {
            CalcKey::Clear => *self = Calc::default(),
            CalcKey::Erase => {
                if self.fresh {
                    self.entry.clear();
                    self.fresh = false;
                } else {
                    self.entry.pop();
                }
            }
            CalcKey::Digit(d) => {
                if self.fresh {
                    self.entry.clear();
                    self.fresh = false;
                }
                // A leading zero is the figure itself, not the one before
                // it.
                if self.entry == "0" {
                    self.entry.clear();
                }
                if self.entry.len() < CALC_FIGURES {
                    self.entry.push(char::from(b'0' + d));
                }
                if self.entry.is_empty() {
                    self.entry.push('0');
                }
            }
            CalcKey::Dot => {
                if self.fresh {
                    self.entry = "0.".to_string();
                    self.fresh = false;
                } else if !self.entry.contains('.') {
                    if self.entry.is_empty() {
                        self.entry.push('0');
                    }
                    self.entry.push('.');
                }
            }
            CalcKey::Negate => {
                if self.fresh {
                    // The number shown is the one last reached: flip it
                    // and go on from there.
                    let v = -self.value();
                    self.entry = calc_number(v);
                    self.fresh = false;
                } else if let Some(rest) = self.entry.strip_prefix('-') {
                    // Take the sign back off the figures as they stand,
                    // its dot and all.
                    self.entry = rest.to_string();
                } else if !self.entry.is_empty() && self.entry != "0" {
                    self.entry.insert(0, '-');
                }
            }
            CalcKey::Percent => {
                let v = self.value() / 100.0;
                self.entry = calc_number(v);
                self.fresh = false;
            }
            CalcKey::Op(op) => {
                let v = self.value();
                let next = match (self.acc, self.op) {
                    // An operator pressed after a number works out the sum
                    // waiting first, as a calculator's chained sums do.
                    (Some(a), Some(waiting)) if !self.fresh => match waiting.apply(a, v) {
                        Some(r) => r,
                        None => {
                            self.broken = true;
                            self.acc = None;
                            self.op = None;
                            return;
                        }
                    },
                    // An operator pressed after another only changes it.
                    (Some(a), Some(_)) => a,
                    _ => v,
                };
                self.acc = Some(next);
                self.op = Some(op);
                self.fresh = true;
                self.entry = calc_number(next);
            }
            CalcKey::Eq => {
                let v = self.value();
                if let (Some(a), Some(op)) = (self.acc, self.op) {
                    match op.apply(a, v) {
                        Some(r) => {
                            self.entry = calc_number(r);
                            self.acc = Some(r);
                        }
                        None => {
                            self.broken = true;
                            self.acc = None;
                            self.op = None;
                            return;
                        }
                    }
                }
                self.op = None;
                self.fresh = true;
            }
        }
    }
}
