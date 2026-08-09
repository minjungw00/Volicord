pub mod format {
    pub fn normalize(value: &str) -> String {
        value.trim().to_owned()
    }
}

pub trait Named {
    fn name(&self) -> &str;
}

pub struct Greeter {
    prefix: String,
}

pub enum Mode {
    Loud,
    Quiet,
}

pub type Identifier = String;

impl Greeter {
    pub fn new(prefix: &str) -> Self {
        Self { prefix: prefix.to_owned() }
    }

    pub fn greet(&self, person: &Identifier) -> String {
        format::normalize(&format!("{}, {person}", self.name()))
    }
}

impl Named for Greeter {
    fn name(&self) -> &str {
        &self.prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_person() {
        assert!(Greeter::new("hello").greet(&"Ada".to_owned()).contains("Ada"));
    }
}
