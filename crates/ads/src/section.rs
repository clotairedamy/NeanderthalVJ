//! A section of a page, as the rail, tab bar and palette list it.

#[derive(Clone, PartialEq)]
pub struct Section {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub short: Option<String>,
    pub tab: bool,
}

impl Section {
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            short: None,
            tab: false,
        }
    }
    #[must_use]
    pub fn short(mut self, short: impl Into<String>) -> Self {
        self.short = Some(short.into());
        self
    }
    #[must_use]
    pub fn tab(mut self) -> Self {
        self.tab = true;
        self
    }
    #[must_use]
    pub fn display(&self) -> &str {
        self.short.as_deref().unwrap_or(&self.label)
    }
}
