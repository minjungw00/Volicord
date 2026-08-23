pub struct Catalog {
    id: u64,
}

impl Catalog {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn render(&self) -> String {
        self.id.to_string()
    }
}
