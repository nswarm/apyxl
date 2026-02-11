#[derive(Debug, Copy, Clone)]
pub enum Feature {
    Namespace,
}

impl Feature {
    pub fn to_pyx_str(self) -> &'static str {
        match self {
            Feature::Namespace => "namespace ns {}",
        }
    }
}
