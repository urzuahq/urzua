pub struct Cfg {
    /// The names every record must carry.
    pub known: Vec<String>,
    pub spec: Option<String>,
}

impl Cfg {
    /// How many names are known.
    pub fn known(&self) -> usize {
        self.known.len()
    }
}
