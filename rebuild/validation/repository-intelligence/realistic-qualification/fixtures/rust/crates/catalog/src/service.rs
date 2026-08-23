macro_rules! normalize {
    ($value:expr) => {
        $value.trim()
    };
}

pub trait Named {
    fn name(&self) -> &str;
}

pub struct Catalog {
    name: String,
}

impl Catalog {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn render(&self) -> String {
        normalize!(self.name()).to_owned()
    }
}

impl Named for Catalog {
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(feature = "trace")]
pub fn trace(catalog: &Catalog) -> &str {
    catalog.name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_name() {
        assert_eq!(Catalog::new(" Ada ".to_owned()).render(), "Ada");
    }
}
