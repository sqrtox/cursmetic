use std::fmt::Display;

use strum::EnumIter;

use crate::error::Result;
use crate::windows::{ResourceId, load_string, main_cpl};

#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIter, Ord, PartialOrd)]
pub enum Cursor {
    NormalSelect,
    HelpSelect,
    WorkingInBackground,
    Busy,
    PrecisionSelect,
    TextSelect,
    Handwriting,
    Unavailable,
    VerticalResize,
    HorizontalResize,
    DiagonalResize1,
    DiagonalResize2,
    Move,
    AlternateSelect,
    LinkSelect,
    LocationSelect,
    PersonSelect,
    Other(String),
    Unknown,
}

impl Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Cursor::*;

        let s = match self {
            Other(s) => &format!("Other {s}"),
            Unknown => "Unknown",
            // TODO
            _ => self.english_display_name().unwrap(),
        };

        write!(f, "{s}")
    }
}

impl Cursor {
    pub fn is_standard(&self) -> bool {
        use Cursor::*;

        match self {
            NormalSelect | Busy | WorkingInBackground | Unavailable | TextSelect
            | PrecisionSelect | VerticalResize | HorizontalResize | DiagonalResize1
            | DiagonalResize2 | Move | HelpSelect | Handwriting | AlternateSelect | LinkSelect
            | LocationSelect | PersonSelect => true,
            _ => false,
        }
    }

    pub fn english_display_name(&self) -> Option<&str> {
        use Cursor::*;

        Some(match self {
            NormalSelect => "Normal Select",
            Busy => "Busy",
            WorkingInBackground => "Working In Background",
            Unavailable => "Unavailable",
            TextSelect => "Text Select",
            PrecisionSelect => "Precision Select",
            VerticalResize => "Vertical Resize",
            HorizontalResize => "Horizontal Resize",
            DiagonalResize1 => "Diagonal Resize 1",
            DiagonalResize2 => "Diagonal Resize 2",
            Move => "Move",
            HelpSelect => "Help Select",
            Handwriting => "Handwriting",
            AlternateSelect => "Alternate Select",
            LinkSelect => "Link Select",
            LocationSelect => "Location Select",
            PersonSelect => "Person Select",
            _ => return None,
        })
    }

    pub fn display_name(&self) -> Result<Option<String>> {
        let resource_id = ResourceId::from(self);
        let main_cpl = main_cpl()?;

        Ok(load_string(main_cpl, &resource_id))
    }

    pub const fn registry_name(&self) -> Option<&str> {
        use Cursor::*;

        Some(match self {
            NormalSelect => "Arrow",
            Busy => "Wait",
            WorkingInBackground => "AppStarting",
            Unavailable => "No",
            TextSelect => "IBeam",
            PrecisionSelect => "Crosshair",
            VerticalResize => "SizeNS",
            HorizontalResize => "SizeWE",
            DiagonalResize1 => "SizeNWSE",
            DiagonalResize2 => "SizeNESW",
            Move => "SizeAll",
            HelpSelect => "Help",
            Handwriting => "NWPen",
            AlternateSelect => "UpArrow",
            LinkSelect => "Hand",
            LocationSelect => "Pin",
            PersonSelect => "Person",
            _ => return None,
        })
    }
}

impl From<&Cursor> for ResourceId {
    fn from(value: &Cursor) -> Self {
        use Cursor::*;

        Self(Some(match value {
            NormalSelect => 207,
            Busy => 208,
            WorkingInBackground => 209,
            Unavailable => 210,
            TextSelect => 211,
            PrecisionSelect => 212,
            VerticalResize => 213,
            HorizontalResize => 214,
            DiagonalResize1 => 215,
            DiagonalResize2 => 216,
            Move => 217,
            HelpSelect => 218,
            Handwriting => 219,
            AlternateSelect => 220,
            LinkSelect => 225,
            LocationSelect => 226,
            PersonSelect => 227,
            _ => return ResourceId(None),
        }))
    }
}
