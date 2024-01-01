/// The version of the engine (not the crate version)
pub struct EngineVersion {
    pub version: usize,
    pub name: &'static str,
    pub author: &'static str,
}

impl EngineVersion {
    pub const CURRENT: EngineVersion = EngineVersion {
        version: 2,
        name: "daisy",
        author: env!("CARGO_PKG_AUTHORS"),
    };
}

impl std::fmt::Display for EngineVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "weechess.{:03}-{}", self.version, self.name)
    }
}
