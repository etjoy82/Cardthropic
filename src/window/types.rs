use crate::engine::game_mode::VariantRuntime;
use crate::game::{Card, DrawMode, GameMode};

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub(super) mode: GameMode,
    pub(super) runtime: VariantRuntime,
    pub(super) draw_mode: DrawMode,
    pub(super) selected_run: Option<SelectedRun>,
    pub(super) selected_waste: bool,
    pub(super) move_count: u32,
    pub(super) elapsed_seconds: u32,
    pub(super) timer_started: bool,
    pub(super) apm_samples: Vec<ApmSample>,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(super) struct ApmSample {
    pub(super) elapsed_seconds: u32,
    pub(super) apm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedRun {
    pub(crate) col: usize,
    pub(crate) start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartMoveMode {
    Disabled,
    SingleClick,
    DoubleClick,
}

impl SmartMoveMode {
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::SingleClick => "single-click",
            Self::DoubleClick => "double-click",
        }
    }

    pub fn from_setting(value: &str) -> Self {
        match value {
            "disabled" => Self::Disabled,
            "single-click" => Self::SingleClick,
            "double-click" => Self::DoubleClick,
            _ => Self::DoubleClick,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotStrategy {
    Fast,
    Balanced,
    Deep,
}

impl RobotStrategy {
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Deep => "deep",
        }
    }

    pub fn from_setting(value: &str) -> Self {
        match value {
            "fast" => Self::Fast,
            "balanced" => Self::Balanced,
            "deep" => Self::Deep,
            _ => Self::Balanced,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fast => "Fast",
            Self::Balanced => "Balanced",
            Self::Deep => "Deep",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DragOrigin {
    Waste,
    Tableau { col: usize, start: usize },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum WorkspacePreset {
    Compact600,
    Hd720,
    Fhd1080,
    Qhd1440,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WorkspaceLayoutProfile {
    pub(super) side_padding: i32,
    pub(super) tableau_vertical_padding: i32,
    pub(super) gap: i32,
    pub(super) assumed_depth: i32,
    pub(super) min_card_width: i32,
    pub(super) max_card_width: i32,
    pub(super) min_card_height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TableauPictureRenderState {
    pub(super) card: Card,
    pub(super) display_face_up: bool,
    pub(super) selected: bool,
    pub(super) y: i32,
    pub(super) card_width: i32,
    pub(super) card_height: i32,
}
