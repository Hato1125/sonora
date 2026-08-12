use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    List,
    Cards,
}

impl Mode {
    pub const ALL: [Self; 2] = [Self::List, Self::Cards];

    pub fn key(self) -> &'static str {
        match self {
            Self::List => "view-list",
            Self::Cards => "view-cards",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::List => "icons/list.svg",
            Self::Cards => "icons/layout-grid.svg",
        }
    }
}
