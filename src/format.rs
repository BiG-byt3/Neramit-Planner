//! Formatting of the Gantt Chart view — the model behind the standard Format tab and its dialogs:
//! Text Styles, Bar Styles / Format Bar, Layout, Gridlines and Timescale.
//!
//! Defaults follow the standard built-in "Gantt Chart" view. Enumerations mirror the type
//! library (BarShape, BarType, Gridline, TextItem, TimescaleUnit).

use serde::{Deserialize, Serialize};

pub type Rgb = [u8; 3];

pub const BLACK: Rgb = [0, 0, 0];
/// The standard default task bar blue (RGB(35, 118, 176) in the Format Bar dialog).
pub const TASK_BLUE: Rgb = [35, 118, 176];
pub const RED: Rgb = [255, 0, 0];
pub const SILVER: Rgb = [192, 192, 192];
pub const GRAY: Rgb = [128, 128, 128];
pub const TEAL: Rgb = [0, 128, 128];

/// The standard 16 named colours (Color) plus "Automatic" for pickers.
pub const NAMED_COLORS: [(&str, Rgb); 16] = [
    ("Black", [0, 0, 0]),
    ("Red", [255, 0, 0]),
    ("Yellow", [255, 255, 0]),
    ("Lime", [0, 255, 0]),
    ("Aqua", [0, 255, 255]),
    ("Blue", [0, 0, 255]),
    ("Fuchsia", [255, 0, 255]),
    ("White", [255, 255, 255]),
    ("Maroon", [128, 0, 0]),
    ("Green", [0, 128, 0]),
    ("Olive", [128, 128, 0]),
    ("Navy", [0, 0, 128]),
    ("Purple", [128, 0, 128]),
    ("Teal", [0, 128, 128]),
    ("Gray", [128, 128, 128]),
    ("Silver", [192, 192, 192]),
];

/// Theme colours offered by the standard colour pickers (first row of "Theme Colors").
pub const THEME_COLORS: [Rgb; 10] = [
    [255, 255, 255],
    [0, 0, 0],
    [231, 230, 230],
    [68, 84, 106],
    [68, 114, 196],
    [237, 125, 49],
    [165, 165, 165],
    [255, 192, 0],
    [91, 155, 213],
    [112, 173, 71],
];

pub fn color_name(c: Rgb) -> String {
    NAMED_COLORS.iter().find(|(_, v)| *v == c).map(|(n, _)| n.to_string()).unwrap_or_else(|| format!("RGB({}, {}, {})", c[0], c[1], c[2]))
}

// ====================================================================== text styles

/// TextItem — "Item to Change" in the Text Styles dialog.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum TextItem {
    All,
    Noncritical,
    Critical,
    Milestone,
    Summary,
    ProjectSummary,
    Marked,
    Highlighted,
    RowColumnTitles,
    TopTier,
    MiddleTier,
    BottomTier,
    BarTextLeft,
    BarTextRight,
    BarTextTop,
    BarTextBottom,
    BarTextInside,
    External,
    ChangedCells,
    Inactive,
    ManuallyScheduled,
    Placeholder,
    Late,
}

impl TextItem {
    pub const ALL: [TextItem; 23] = [
        TextItem::All,
        TextItem::Noncritical,
        TextItem::Critical,
        TextItem::Milestone,
        TextItem::Summary,
        TextItem::ProjectSummary,
        TextItem::Marked,
        TextItem::Highlighted,
        TextItem::RowColumnTitles,
        TextItem::TopTier,
        TextItem::MiddleTier,
        TextItem::BottomTier,
        TextItem::BarTextLeft,
        TextItem::BarTextRight,
        TextItem::BarTextTop,
        TextItem::BarTextBottom,
        TextItem::BarTextInside,
        TextItem::External,
        TextItem::ChangedCells,
        TextItem::Inactive,
        TextItem::ManuallyScheduled,
        TextItem::Placeholder,
        TextItem::Late,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TextItem::All => "All",
            TextItem::Noncritical => "Noncritical Tasks",
            TextItem::Critical => "Critical Tasks",
            TextItem::Milestone => "Milestone Tasks",
            TextItem::Summary => "Summary Tasks",
            TextItem::ProjectSummary => "Project Summary Tasks",
            TextItem::Marked => "Marked Tasks",
            TextItem::Highlighted => "Highlighted Tasks",
            TextItem::RowColumnTitles => "Row & Column Titles",
            TextItem::TopTier => "Top Timescale Tier",
            TextItem::MiddleTier => "Middle Timescale Tier",
            TextItem::BottomTier => "Bottom Timescale Tier",
            TextItem::BarTextLeft => "Bar Text - Left",
            TextItem::BarTextRight => "Bar Text - Right",
            TextItem::BarTextTop => "Bar Text - Top",
            TextItem::BarTextBottom => "Bar Text - Bottom",
            TextItem::BarTextInside => "Bar Text - Inside",
            TextItem::External => "External Tasks",
            TextItem::ChangedCells => "Changed Cells",
            TextItem::Inactive => "Inactive Tasks",
            TextItem::ManuallyScheduled => "Manually Scheduled Tasks",
            TextItem::Placeholder => "Placeholder Tasks",
            TextItem::Late => "Late Tasks",
        }
    }
}

/// Font, colour and cell background of one text item (or of one cell when used as a cell format).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TextStyle {
    #[serde(default)]
    pub font: String,
    /// point size as shown in the dialogs (Calibri 11 in MPP)
    #[serde(default = "default_size")]
    pub size: f32,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strike: bool,
    /// `None` = Automatic
    #[serde(default)]
    pub color: Option<Rgb>,
    #[serde(default)]
    pub background: Option<Rgb>,
}

fn default_size() -> f32 {
    11.0
}

pub const DEFAULT_FONT: &str = "Noto Sans";

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle { font: DEFAULT_FONT.into(), size: 11.0, bold: false, italic: false, underline: false, strike: false, color: None, background: None }
    }
}

impl TextStyle {
    pub fn bold() -> Self {
        TextStyle { bold: true, ..Default::default() }
    }

    /// Pixel size of the body text for a point size: Noto Sans has a larger x-height than
    /// Calibri, so the standard Calibri 11 pt matches Noto Sans at 12 px.
    pub fn px(&self) -> f32 {
        // a real Windows font (Calibri, Segoe UI, Cordia New ...) is drawn at its true size
        // like in MPP (pt x 96 / 72); the tuning below is for the bundled Noto Sans
        if !self.font.is_empty() && self.font != DEFAULT_FONT {
            return (self.size * 4.0 / 3.0).clamp(6.0, 96.0);
        }
        // small sizes (timescale, bar text: 8 pt) get a readable floor on screen
        (self.size * 12.0 / 11.0).max(self.size + 1.5).clamp(6.0, 72.0)
    }

    /// Layer `over` on top of `self`: explicit settings of `over` win.
    pub fn merged(&self, over: &CellFormat) -> TextStyle {
        let mut s = self.clone();
        let set = |bit: u8| over.exact & bit != 0;
        if set(1) {
            s.bold = over.bold;
        }
        if set(2) {
            s.italic = over.italic;
        }
        if set(4) {
            s.underline = over.underline;
        }
        if set(8) {
            s.strike = over.strike;
        }
        if over.bold {
            s.bold = true;
        }
        if over.italic {
            s.italic = true;
        }
        if over.underline {
            s.underline = true;
        }
        if over.strike {
            s.strike = true;
        }
        if let Some(c) = over.color {
            s.color = Some(c);
        }
        if let Some(b) = over.background {
            s.background = Some(b);
        }
        if let Some(z) = over.size {
            s.size = z;
        }
        if let Some(f) = &over.font {
            s.font = f.clone();
        }
        s
    }
}

/// Formatting applied directly to a cell (Font group / Font dialog), layered over the text styles.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct CellFormat {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strike: bool,
    #[serde(default)]
    pub color: Option<Rgb>,
    #[serde(default)]
    pub background: Option<Rgb>,
    #[serde(default)]
    pub size: Option<f32>,
    #[serde(default)]
    pub font: Option<String>,
    /// Read from an MPP file: the style flags it sets on or off (1 bold, 2 italic, 4 underline,
    /// 8 strike), e.g. a summary row's cell that is not bold
    #[serde(default, skip_serializing_if = "is_zero")]
    pub exact: u8,
    /// Read from an MPP file: a cell's own format replaces its row's format
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub whole: bool,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

impl CellFormat {
    pub fn is_empty(&self) -> bool {
        *self == CellFormat::default()
    }
}

// ====================================================================== bar styles

/// BarShape: the middle part of a bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MidShape {
    None,
    Full,
    Top,
    Middle,
    Bottom,
    LineTop,
    LineMiddle,
    LineBottom,
}

impl MidShape {
    pub const ALL: [MidShape; 8] = [MidShape::None, MidShape::Full, MidShape::Top, MidShape::Middle, MidShape::Bottom, MidShape::LineTop, MidShape::LineMiddle, MidShape::LineBottom];
    pub fn label(self) -> &'static str {
        match self {
            MidShape::None => "No Shape",
            MidShape::Full => "Rectangle",
            MidShape::Top => "Rectangle (top)",
            MidShape::Middle => "Rectangle (middle)",
            MidShape::Bottom => "Rectangle (bottom)",
            MidShape::LineTop => "Line (top)",
            MidShape::LineMiddle => "Line (middle)",
            MidShape::LineBottom => "Line (bottom)",
        }
    }
}

/// Fill pattern of the middle part.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Pattern {
    Hollow,
    Solid,
    LightDots,
    MediumDots,
    DenseDots,
    DiagonalUp,
    DiagonalDown,
    Horizontal,
    Vertical,
    Cross,
    DiagonalCross,
}

impl Pattern {
    pub const ALL: [Pattern; 11] = [
        Pattern::Hollow,
        Pattern::Solid,
        Pattern::LightDots,
        Pattern::MediumDots,
        Pattern::DenseDots,
        Pattern::DiagonalUp,
        Pattern::DiagonalDown,
        Pattern::Horizontal,
        Pattern::Vertical,
        Pattern::Cross,
        Pattern::DiagonalCross,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Hollow => "Hollow",
            Pattern::Solid => "Solid Fill",
            Pattern::LightDots => "Light Dots",
            Pattern::MediumDots => "Medium Dots",
            Pattern::DenseDots => "Dense Dots",
            Pattern::DiagonalUp => "Diagonal Up",
            Pattern::DiagonalDown => "Diagonal Down",
            Pattern::Horizontal => "Horizontal Lines",
            Pattern::Vertical => "Vertical Lines",
            Pattern::Cross => "Cross Hatch",
            Pattern::DiagonalCross => "Diagonal Cross",
        }
    }
}

/// Start / end marker shapes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EndShape {
    None,
    HouseUp,
    HouseDown,
    Circle,
    Diamond,
    TriangleUp,
    TriangleDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Star,
    Square,
    Line,
    BracketLeft,
    BracketRight,
    CaretUp,
    CaretDown,
    Dot,
}

impl EndShape {
    pub const ALL: [EndShape; 19] = [
        EndShape::None,
        EndShape::HouseUp,
        EndShape::HouseDown,
        EndShape::Circle,
        EndShape::Diamond,
        EndShape::TriangleUp,
        EndShape::TriangleDown,
        EndShape::ArrowUp,
        EndShape::ArrowDown,
        EndShape::ArrowLeft,
        EndShape::ArrowRight,
        EndShape::Star,
        EndShape::Square,
        EndShape::Line,
        EndShape::BracketLeft,
        EndShape::BracketRight,
        EndShape::CaretUp,
        EndShape::CaretDown,
        EndShape::Dot,
    ];
    pub fn label(self) -> &'static str {
        match self {
            EndShape::None => "No Shape",
            EndShape::HouseUp => "House (up)",
            EndShape::HouseDown => "House (down)",
            EndShape::Circle => "Circle",
            EndShape::Diamond => "Diamond",
            EndShape::TriangleUp => "Triangle (up)",
            EndShape::TriangleDown => "Triangle (down)",
            EndShape::ArrowUp => "Arrow (up)",
            EndShape::ArrowDown => "Arrow (down)",
            EndShape::ArrowLeft => "Arrow (left)",
            EndShape::ArrowRight => "Arrow (right)",
            EndShape::Star => "Star",
            EndShape::Square => "Square",
            EndShape::Line => "Line",
            EndShape::BracketLeft => "Bracket (left)",
            EndShape::BracketRight => "Bracket (right)",
            EndShape::CaretUp => "Caret (up)",
            EndShape::CaretDown => "Caret (down)",
            EndShape::Dot => "Dot",
        }
    }
}

/// BarType: how a marker is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum EndType {
    Solid,
    Framed,
    Dashed,
}

impl EndType {
    pub const ALL: [EndType; 3] = [EndType::Solid, EndType::Framed, EndType::Dashed];
    pub fn label(self) -> &'static str {
        match self {
            EndType::Solid => "Solid",
            EndType::Framed => "Framed",
            EndType::Dashed => "Dashed",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct EndStyle {
    pub shape: EndShape,
    pub kind: EndType,
    pub color: Rgb,
}

impl EndStyle {
    pub const NONE: EndStyle = EndStyle { shape: EndShape::None, kind: EndType::Solid, color: BLACK };
    pub const fn solid(shape: EndShape, color: Rgb) -> EndStyle {
        EndStyle { shape, kind: EndType::Solid, color }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct MidStyle {
    pub shape: MidShape,
    pub pattern: Pattern,
    pub color: Rgb,
}

impl MidStyle {
    pub const NONE: MidStyle = MidStyle { shape: MidShape::None, pattern: Pattern::Solid, color: BLACK };
    pub const fn solid(shape: MidShape, color: Rgb) -> MidStyle {
        MidStyle { shape, pattern: Pattern::Solid, color }
    }
}

/// Start, middle and end parts of a bar — the "Bar Shape" tab of Format Bar.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct BarAppearance {
    pub start: EndStyle,
    pub middle: MidStyle,
    pub end: EndStyle,
}

/// Dates a bar can run from / to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BarField {
    Start,
    Finish,
    ActualStart,
    ActualFinish,
    CompleteThrough,
    Deadline,
    BaselineStart,
    BaselineFinish,
    EarlyStart,
    EarlyFinish,
    LateStart,
    LateFinish,
}

impl BarField {
    pub const ALL: [BarField; 12] = [
        BarField::Start,
        BarField::Finish,
        BarField::ActualStart,
        BarField::ActualFinish,
        BarField::CompleteThrough,
        BarField::Deadline,
        BarField::BaselineStart,
        BarField::BaselineFinish,
        BarField::EarlyStart,
        BarField::EarlyFinish,
        BarField::LateStart,
        BarField::LateFinish,
    ];
    pub fn label(self) -> &'static str {
        match self {
            BarField::Start => "Start",
            BarField::Finish => "Finish",
            BarField::ActualStart => "Actual Start",
            BarField::ActualFinish => "Actual Finish",
            BarField::CompleteThrough => "CompleteThrough",
            BarField::Deadline => "Deadline",
            BarField::BaselineStart => "Baseline Start",
            BarField::BaselineFinish => "Baseline Finish",
            BarField::EarlyStart => "Early Start",
            BarField::EarlyFinish => "Early Finish",
            BarField::LateStart => "Late Start",
            BarField::LateFinish => "Late Finish",
        }
    }
}

/// Fields that can be shown as bar text (left / right / top / bottom / inside).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum BarText {
    #[default]
    None,
    Name,
    Start,
    Finish,
    Duration,
    PercentComplete,
    ResourceNames,
    ResourceInitials,
    Deadline,
    Predecessors,
    Successors,
    Priority,
    Notes,
    Wbs,
    Cost,
    Id,
    UniqueId,
    OutlineNumber,
    TotalSlack,
    Contact,
}

impl BarText {
    pub const ALL: [BarText; 20] = [
        BarText::None,
        BarText::Name,
        BarText::Start,
        BarText::Finish,
        BarText::Duration,
        BarText::PercentComplete,
        BarText::ResourceNames,
        BarText::ResourceInitials,
        BarText::Deadline,
        BarText::Predecessors,
        BarText::Successors,
        BarText::Priority,
        BarText::Notes,
        BarText::Wbs,
        BarText::Cost,
        BarText::Id,
        BarText::UniqueId,
        BarText::OutlineNumber,
        BarText::TotalSlack,
        BarText::Contact,
    ];
    pub fn label(self) -> &'static str {
        match self {
            BarText::None => "",
            BarText::Name => "Name",
            BarText::Start => "Start",
            BarText::Finish => "Finish",
            BarText::Duration => "Duration",
            BarText::PercentComplete => "% Complete",
            BarText::ResourceNames => "Resource Names",
            BarText::ResourceInitials => "Resource Initials",
            BarText::Deadline => "Deadline",
            BarText::Predecessors => "Predecessors",
            BarText::Successors => "Successors",
            BarText::Priority => "Priority",
            BarText::Notes => "Notes",
            BarText::Wbs => "WBS",
            BarText::Cost => "Cost",
            BarText::Id => "ID",
            BarText::UniqueId => "Unique ID",
            BarText::OutlineNumber => "Outline Number",
            BarText::TotalSlack => "Total Slack",
            BarText::Contact => "Contact",
        }
    }
}

/// Task categories used in "Show For ... Tasks".
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Category {
    Normal,
    Milestone,
    Summary,
    Critical,
    Noncritical,
    Marked,
    Finished,
    InProgress,
    NotStarted,
    NotFinished,
    RolledUp,
    ProjectSummary,
    Split,
    External,
    ManuallyScheduled,
    Active,
    Placeholder,
    Late,
    Deliverable,
    GroupBySummary,
    Flag(u8),
}

impl Category {
    pub const ALL: [Category; 20] = [
        Category::Normal,
        Category::Milestone,
        Category::Summary,
        Category::Critical,
        Category::Noncritical,
        Category::Marked,
        Category::Finished,
        Category::InProgress,
        Category::NotStarted,
        Category::NotFinished,
        Category::RolledUp,
        Category::ProjectSummary,
        Category::Split,
        Category::External,
        Category::ManuallyScheduled,
        Category::Active,
        Category::Placeholder,
        Category::Late,
        Category::Deliverable,
        Category::GroupBySummary,
    ];
    pub fn label(self) -> String {
        match self {
            Category::Normal => "Normal".into(),
            Category::Milestone => "Milestone".into(),
            Category::Summary => "Summary".into(),
            Category::Critical => "Critical".into(),
            Category::Noncritical => "Noncritical".into(),
            Category::Marked => "Marked".into(),
            Category::Finished => "Finished".into(),
            Category::InProgress => "In Progress".into(),
            Category::NotStarted => "Not Started".into(),
            Category::NotFinished => "Not Finished".into(),
            Category::RolledUp => "Rolled Up".into(),
            Category::ProjectSummary => "Project Summary".into(),
            Category::Split => "Split".into(),
            Category::External => "External Tasks".into(),
            Category::ManuallyScheduled => "Manually Scheduled".into(),
            Category::Active => "Active".into(),
            Category::Placeholder => "Placeholder".into(),
            Category::Late => "Late".into(),
            Category::Deliverable => "Deliverable".into(),
            Category::GroupBySummary => "Group By Summary".into(),
            Category::Flag(n) => format!("Flag{n}"),
        }
    }
}

/// One entry of the "Show For" list, optionally negated ("Not Summary").
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ShowFor {
    pub cat: Category,
    pub not: bool,
}

const fn sf(cat: Category) -> ShowFor {
    ShowFor { cat, not: false }
}
const fn not(cat: Category) -> ShowFor {
    ShowFor { cat, not: true }
}

pub fn show_for_text(v: &[ShowFor]) -> String {
    v.iter().map(|s| if s.not { format!("Not {}", s.cat.label()) } else { s.cat.label() }).collect::<Vec<_>>().join(",")
}

/// One row of the Bar Styles dialog.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct BarStyle {
    pub name: String,
    pub show_for: Vec<ShowFor>,
    /// 1..=4
    pub row: u8,
    pub from: BarField,
    pub to: BarField,
    pub bar: BarAppearance,
    /// left, right, top, bottom, inside
    pub text: [BarText; 5],
}

impl BarStyle {
    fn new(name: &str, show_for: &[ShowFor], from: BarField, to: BarField, bar: BarAppearance) -> BarStyle {
        BarStyle { name: name.into(), show_for: show_for.to_vec(), row: 1, from, to, bar, text: [BarText::None; 5] }
    }
    fn right(mut self, t: BarText) -> BarStyle {
        self.text[1] = t;
        self
    }
}

/// The standard default "Gantt Chart" view bar styles (MPP14).
/// the default Gantt Chart bar styles (read from an MPP file: light blue tasks, grey
/// summary brackets, teal manual tasks, the hidden "*" rollup styles, red critical styles used when
/// Critical Tasks is on), plus its progress bars.
pub fn default_bar_styles() -> Vec<BarStyle> {
    let mut v: Vec<BarStyle> = serde_json::from_str(include_str!("../assets/gantt-bar-styles.json")).unwrap_or_else(|_| builtin_bar_styles());
    // the bundled list carries the standard Progress bars; older lists lacked them
    if v.iter().any(|b| b.name == "Progress") {
        return v;
    }
    if let Some(look) = v.iter().find(|b| b.name == "*Rolled Up Progress").map(|b| b.bar) {
        use Category as C;
        v.push(BarStyle::new("Progress", &[sf(C::Normal), sf(C::Active), not(C::ManuallyScheduled)], BarField::ActualStart, BarField::CompleteThrough, look));
        v.push(BarStyle::new("Manual Progress", &[sf(C::Normal), sf(C::Active), sf(C::ManuallyScheduled)], BarField::ActualStart, BarField::CompleteThrough, look));
    }
    v
}

/// the default Tracking Gantt bar styles (read from an MPP file): the current schedule
/// in the top half of the row, its progress, and the baseline in grey in the bottom half.
pub fn default_tracking_bar_styles() -> Vec<BarStyle> {
    serde_json::from_str(include_str!("../assets/tracking-gantt-bar-styles.json")).unwrap_or_else(|_| default_bar_styles())
}

/// Fallback bar styles (used only if the bundled MPP styles cannot be read).
fn builtin_bar_styles() -> Vec<BarStyle> {
    use BarField::*;
    use Category as C;
    let task = BarAppearance { start: EndStyle::NONE, middle: MidStyle::solid(MidShape::Full, TASK_BLUE), end: EndStyle::NONE };
    let split = BarAppearance { start: EndStyle::NONE, middle: MidStyle { shape: MidShape::Full, pattern: Pattern::MediumDots, color: TASK_BLUE }, end: EndStyle::NONE };
    let progress = BarAppearance { start: EndStyle::NONE, middle: MidStyle::solid(MidShape::LineMiddle, BLACK), end: EndStyle::NONE };
    let milestone = BarAppearance { start: EndStyle::solid(EndShape::Diamond, BLACK), middle: MidStyle::NONE, end: EndStyle::NONE };
    let summary = BarAppearance { start: EndStyle::solid(EndShape::BracketLeft, BLACK), middle: MidStyle::solid(MidShape::Top, BLACK), end: EndStyle::solid(EndShape::BracketRight, BLACK) };
    let proj_summary = BarAppearance { start: EndStyle::solid(EndShape::BracketLeft, GRAY), middle: MidStyle::solid(MidShape::Top, GRAY), end: EndStyle::solid(EndShape::BracketRight, GRAY) };
    let manual = BarAppearance { start: EndStyle::solid(EndShape::Line, TEAL), middle: MidStyle::solid(MidShape::Full, [120, 190, 210]), end: EndStyle::solid(EndShape::Line, TEAL) };
    let manual_summary = BarAppearance { start: EndStyle::solid(EndShape::BracketLeft, TEAL), middle: MidStyle::solid(MidShape::Top, TEAL), end: EndStyle::solid(EndShape::BracketRight, TEAL) };
    let manual_ms = BarAppearance { start: EndStyle::solid(EndShape::Diamond, TEAL), middle: MidStyle::NONE, end: EndStyle::NONE };
    let inactive = BarAppearance { start: EndStyle::NONE, middle: MidStyle { shape: MidShape::Full, pattern: Pattern::Hollow, color: GRAY }, end: EndStyle::NONE };
    let inactive_ms = BarAppearance { start: EndStyle { shape: EndShape::Diamond, kind: EndType::Framed, color: GRAY }, middle: MidStyle::NONE, end: EndStyle::NONE };
    let inactive_sum = BarAppearance { start: EndStyle { shape: EndShape::BracketLeft, kind: EndType::Framed, color: GRAY }, middle: MidStyle { shape: MidShape::Top, pattern: Pattern::Hollow, color: GRAY }, end: EndStyle { shape: EndShape::BracketRight, kind: EndType::Framed, color: GRAY } };
    let deadline = BarAppearance { start: EndStyle::solid(EndShape::ArrowDown, [0, 128, 0]), middle: MidStyle::NONE, end: EndStyle::NONE };
    let critical = BarAppearance { start: EndStyle::NONE, middle: MidStyle::solid(MidShape::Full, RED), end: EndStyle::NONE };
    let critical_progress = BarAppearance { start: EndStyle::NONE, middle: MidStyle::solid(MidShape::LineMiddle, [128, 0, 0]), end: EndStyle::NONE };
    let slack = BarAppearance { start: EndStyle::NONE, middle: MidStyle::solid(MidShape::LineBottom, BLACK), end: EndStyle::NONE };
    vec![
        BarStyle::new("Task", &[sf(C::Normal), sf(C::Active), not(C::ManuallyScheduled)], Start, Finish, task),
        BarStyle::new("Split", &[sf(C::Normal), sf(C::Split), sf(C::Active), not(C::ManuallyScheduled)], Start, Finish, split),
        BarStyle::new("Milestone", &[sf(C::Milestone), sf(C::Active), not(C::GroupBySummary), not(C::Deliverable)], Start, Start, milestone).right(BarText::Start),
        BarStyle::new("Summary", &[sf(C::Summary), sf(C::Active), not(C::ManuallyScheduled)], Start, Finish, summary),
        BarStyle::new("Project Summary", &[sf(C::ProjectSummary)], Start, Finish, proj_summary),
        BarStyle::new("Manual Task", &[sf(C::Normal), sf(C::Active), sf(C::ManuallyScheduled)], Start, Finish, manual),
        BarStyle::new("Manual Summary", &[sf(C::Summary), sf(C::Active), sf(C::ManuallyScheduled)], Start, Finish, manual_summary),
        BarStyle::new("Manual Milestone", &[sf(C::Milestone), sf(C::Active), sf(C::ManuallyScheduled)], Start, Start, manual_ms).right(BarText::Start),
        BarStyle::new("Inactive Task", &[sf(C::Normal), not(C::Active)], Start, Finish, inactive),
        BarStyle::new("Inactive Milestone", &[sf(C::Milestone), not(C::Active)], Start, Start, inactive_ms).right(BarText::Start),
        BarStyle::new("Inactive Summary", &[sf(C::Summary), not(C::Active)], Start, Finish, inactive_sum),
        BarStyle::new("Progress", &[sf(C::Normal), sf(C::Active)], ActualStart, CompleteThrough, progress),
        BarStyle::new("Deadline", &[], Deadline, Deadline, deadline),
        BarStyle::new("Critical", &[sf(C::Normal), sf(C::Critical), sf(C::Active), not(C::ManuallyScheduled)], Start, Finish, critical),
        BarStyle::new("Critical Progress", &[sf(C::Normal), sf(C::Critical), sf(C::Active)], ActualStart, CompleteThrough, critical_progress),
        BarStyle::new("Slack", &[sf(C::Normal), sf(C::Active)], Finish, LateFinish, slack),
    ]
}

// ====================================================================== layout

/// Link line style ("Links" pictures in the Layout dialog).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LinkStyle {
    None,
    #[default]
    Rectilinear,
    Straight,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Layout {
    #[serde(default)]
    pub links: LinkStyle,
    /// DateFormat for dates in bar text; 255 = the project's date format
    #[serde(default = "default_bar_date_format")]
    pub date_format: u8,
    /// 6, 8, 10, 12, 14, 18 or 24
    #[serde(default = "default_bar_height")]
    pub bar_height: u8,
    #[serde(default)]
    pub always_roll_up: bool,
    #[serde(default)]
    pub hide_rollup_when_expanded: bool,
    #[serde(default = "crate::model::default_true")]
    pub round_to_whole_days: bool,
    #[serde(default = "crate::model::default_true")]
    pub show_bar_splits: bool,
    #[serde(default = "crate::model::default_true")]
    pub show_drawings: bool,
}

fn default_bar_date_format() -> u8 {
    12
}
fn default_bar_height() -> u8 {
    12
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            links: LinkStyle::Rectilinear,
            date_format: 12,
            bar_height: 12,
            always_roll_up: false,
            hide_rollup_when_expanded: false,
            round_to_whole_days: true,
            show_bar_splits: true,
            show_drawings: true,
        }
    }
}

pub const BAR_HEIGHTS: [u8; 7] = [6, 8, 10, 12, 14, 18, 24];

// ====================================================================== gridlines

/// "Line to change" in the Gridlines dialog (Gantt Chart view).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GridKind {
    GanttRows,
    BarRows,
    MiddleTierColumn,
    BottomTierColumn,
    CurrentDate,
    SheetRows,
    SheetColumns,
    TitleVertical,
    TitleHorizontal,
    PageBreaks,
    ProjectStart,
    ProjectFinish,
    StatusDate,
    TopTierColumn,
}

impl GridKind {
    pub const ALL: [GridKind; 14] = [
        GridKind::GanttRows,
        GridKind::BarRows,
        GridKind::MiddleTierColumn,
        GridKind::BottomTierColumn,
        GridKind::CurrentDate,
        GridKind::SheetRows,
        GridKind::SheetColumns,
        GridKind::TitleVertical,
        GridKind::TitleHorizontal,
        GridKind::PageBreaks,
        GridKind::ProjectStart,
        GridKind::ProjectFinish,
        GridKind::StatusDate,
        GridKind::TopTierColumn,
    ];
    pub fn label(self) -> &'static str {
        match self {
            GridKind::GanttRows => "Gantt Rows",
            GridKind::BarRows => "Bar Rows",
            GridKind::MiddleTierColumn => "Middle Tier Column",
            GridKind::BottomTierColumn => "Bottom Tier Column",
            GridKind::CurrentDate => "Current Date",
            GridKind::SheetRows => "Sheet Rows",
            GridKind::SheetColumns => "Sheet Columns",
            GridKind::TitleVertical => "Title Vertical",
            GridKind::TitleHorizontal => "Title Horizontal",
            GridKind::PageBreaks => "Page Breaks",
            GridKind::ProjectStart => "Project Start",
            GridKind::ProjectFinish => "Project Finish",
            GridKind::StatusDate => "Status Date",
            GridKind::TopTierColumn => "Top Tier Column",
        }
    }
    /// lines that support "At interval"
    pub fn has_interval(self) -> bool {
        matches!(self, GridKind::GanttRows | GridKind::SheetRows | GridKind::SheetColumns | GridKind::MiddleTierColumn | GridKind::BottomTierColumn | GridKind::TopTierColumn)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LineType {
    #[default]
    None,
    Solid,
    Dotted,
    DenseDots,
    Dashed,
    DashDot,
}

impl LineType {
    pub const ALL: [LineType; 6] = [LineType::None, LineType::Solid, LineType::Dotted, LineType::DenseDots, LineType::Dashed, LineType::DashDot];
    pub fn label(self) -> &'static str {
        match self {
            LineType::None => "",
            LineType::Solid => "Solid",
            LineType::Dotted => "Dotted",
            LineType::DenseDots => "Dense dots",
            LineType::Dashed => "Dashed",
            LineType::DashDot => "Dash-dot",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Gridline {
    pub kind: GridKind,
    pub normal: LineType,
    pub normal_color: Rgb,
    /// 0 = none, otherwise every n-th line
    pub interval: u8,
    pub interval_type: LineType,
    pub interval_color: Rgb,
}

impl Gridline {
    const fn new(kind: GridKind, normal: LineType, normal_color: Rgb) -> Gridline {
        Gridline { kind, normal, normal_color, interval: 0, interval_type: LineType::None, interval_color: SILVER }
    }
}

/// The standard default gridlines for the Gantt Chart view.
pub fn default_gridlines() -> Vec<Gridline> {
    let light = [218, 218, 218];
    vec![
        Gridline::new(GridKind::GanttRows, LineType::None, SILVER),
        Gridline::new(GridKind::BarRows, LineType::None, SILVER),
        Gridline::new(GridKind::MiddleTierColumn, LineType::Solid, light),
        Gridline::new(GridKind::BottomTierColumn, LineType::None, SILVER),
        Gridline::new(GridKind::CurrentDate, LineType::Dashed, [255, 128, 0]),
        Gridline::new(GridKind::SheetRows, LineType::Solid, light),
        Gridline::new(GridKind::SheetColumns, LineType::Solid, light),
        Gridline::new(GridKind::TitleVertical, LineType::Solid, light),
        Gridline::new(GridKind::TitleHorizontal, LineType::Solid, light),
        Gridline::new(GridKind::PageBreaks, LineType::None, SILVER),
        Gridline::new(GridKind::ProjectStart, LineType::None, SILVER),
        Gridline::new(GridKind::ProjectFinish, LineType::None, SILVER),
        Gridline::new(GridKind::StatusDate, LineType::None, [128, 0, 128]),
        Gridline::new(GridKind::TopTierColumn, LineType::None, SILVER),
    ]
}

// ====================================================================== timescale

/// TimescaleUnit
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TsUnit {
    Years,
    HalfYears,
    Quarters,
    Months,
    ThirdsOfMonths,
    Weeks,
    Days,
    Hours,
    Minutes,
}

impl TsUnit {
    pub const ALL: [TsUnit; 9] = [TsUnit::Years, TsUnit::HalfYears, TsUnit::Quarters, TsUnit::Months, TsUnit::ThirdsOfMonths, TsUnit::Weeks, TsUnit::Days, TsUnit::Hours, TsUnit::Minutes];
    pub fn label(self) -> &'static str {
        match self {
            TsUnit::Years => "Years",
            TsUnit::HalfYears => "Half Years",
            TsUnit::Quarters => "Quarters",
            TsUnit::Months => "Months",
            TsUnit::ThirdsOfMonths => "Thirds of Months",
            TsUnit::Weeks => "Weeks",
            TsUnit::Days => "Days",
            TsUnit::Hours => "Hours",
            TsUnit::Minutes => "Minutes",
        }
    }
    /// Label formats offered for this unit (sample text as in the standard "Label" list).
    pub fn label_formats(self) -> &'static [&'static str] {
        match self {
            TsUnit::Years => &["2009", "'09", "Year 1", "Y1", "Year 1 (From End)"],
            TsUnit::HalfYears => &["1st Half 2009", "1st Half", "H1 '09", "H1", "Half 1", "Half 1 (From Start)", "Half 1, 2009"],
            TsUnit::Quarters => &["1st Quarter 2009", "Qtr 1, 2009", "Q1 '09", "Q1", "1", "Quarter 1", "Q1 (From Start)"],
            TsUnit::Months => &["January 2009", "Jan '09", "Jan", "J", "1", "Month 1", "M1", "Month 1 (From Start)"],
            TsUnit::ThirdsOfMonths => &["B, M, E", "1, 11, 21", "Beginning, Middle, End"],
            TsUnit::Weeks => &["January 26, 2009", "Jan 26, '09", "Jan 26", "26", "Mon Jan 26", "W5", "Week 5", "W5 (From Start)", "1/26"],
            TsUnit::Days => &["Monday, January 26, 2009", "Mon Jan 26, '09", "Mon Jan 26", "Mon 26", "Jan 26", "26", "M", "1/26", "Day 1", "D1"],
            TsUnit::Hours => &["Monday, January 26, 2009 8 AM", "Jan 26, '09 8 AM", "8 AM", "8", "Hour 1"],
            TsUnit::Minutes => &["8:00 AM", "8:00", "0", "Minute 1"],
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tier {
    pub units: TsUnit,
    pub count: u8,
    /// index into `units.label_formats`
    pub label: u8,
    /// 0 left, 1 center, 2 right
    pub align: u8,
    pub tick_lines: bool,
    pub fiscal_year: bool,
}

impl Tier {
    pub const fn new(units: TsUnit, label: u8, align: u8) -> Tier {
        Tier { units, count: 1, label, align, tick_lines: true, fiscal_year: false }
    }
}

/// Non-working time shading ("Non-working time" tab).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NonWorking {
    /// 0 = behind task bars, 1 = in front of task bars, 2 = do not draw
    pub draw: u8,
    pub color: Rgb,
    pub pattern: Pattern,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Timescale {
    /// 1 = one tier (middle), 2 = two tiers (middle, bottom), 3 = three tiers
    pub show: u8,
    pub top: Tier,
    pub middle: Tier,
    pub bottom: Tier,
    /// percent
    pub size: u8,
    pub separator: bool,
    /// true: tiers follow the zoom level (Zoom In / Zoom Out presets); false: fixed as configured
    pub auto: bool,
    pub nonworking: NonWorking,
}

impl Default for Timescale {
    fn default() -> Self {
        Timescale {
            show: 2,
            top: Tier::new(TsUnit::Quarters, 1, 1),
            middle: Tier::new(TsUnit::Weeks, 1, 0),
            bottom: Tier::new(TsUnit::Days, 6, 1),
            size: 100,
            separator: true,
            auto: true,
            nonworking: NonWorking { draw: 0, color: [247, 247, 247], pattern: Pattern::Solid },
        }
    }
}

// ====================================================================== all together

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Formatting {
    /// styles that differ from the defaults, by item
    #[serde(default)]
    pub text_styles: Vec<(TextItem, TextStyle)>,
    #[serde(default = "default_bar_styles")]
    pub bar_styles: Vec<BarStyle>,
    /// bar styles of the Tracking Gantt view
    #[serde(default = "default_tracking_bar_styles")]
    pub tracking_bar_styles: Vec<BarStyle>,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default = "default_gridlines")]
    pub gridlines: Vec<Gridline>,
    #[serde(default)]
    pub timescale: Timescale,
    /// Format > Bar Styles > Critical Tasks / Slack / Late Tasks check boxes
    #[serde(default)]
    pub show_critical: bool,
    #[serde(default)]
    pub show_slack: bool,
    #[serde(default)]
    pub show_late: bool,
    /// Gantt Chart Style gallery choice (0 = default)
    #[serde(default)]
    pub gallery: u8,
    /// Format > Baseline: the baseline (0 = Baseline, 1..=10) drawn under the task bars
    #[serde(default)]
    pub baseline_bars: Option<u8>,
    /// Format > Slippage: a line from that baseline's start to the task's start
    #[serde(default)]
    pub slippage: Option<u8>,
    /// Format > Gridlines > Progress Lines: a line through the progress of the tasks at the status date
    #[serde(default)]
    pub progress_lines: bool,
}

impl Default for Formatting {
    fn default() -> Self {
        Formatting {
            text_styles: vec![
                (TextItem::Summary, TextStyle::bold()),
                (TextItem::ProjectSummary, TextStyle::bold()),
                (TextItem::Marked, TextStyle { color: Some([0, 0, 192]), ..Default::default() }),
                (TextItem::Highlighted, TextStyle { background: Some([255, 255, 153]), ..Default::default() }),
                (TextItem::ChangedCells, TextStyle { background: Some([220, 230, 241]), ..Default::default() }),
                (TextItem::Inactive, TextStyle { strike: true, color: Some([128, 128, 128]), ..Default::default() }),
                (TextItem::External, TextStyle { color: Some([128, 128, 128]), ..Default::default() }),
                (TextItem::TopTier, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::MiddleTier, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::BottomTier, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::BarTextLeft, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::BarTextRight, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::BarTextTop, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::BarTextBottom, TextStyle { size: 8.0, ..Default::default() }),
                (TextItem::BarTextInside, TextStyle { size: 8.0, ..Default::default() }),
            ],
            bar_styles: default_bar_styles(),
            tracking_bar_styles: default_tracking_bar_styles(),
            layout: Layout::default(),
            gridlines: default_gridlines(),
            timescale: Timescale::default(),
            show_critical: false,
            show_slack: false,
            show_late: false,
            gallery: 0,
            baseline_bars: None,
            slippage: None,
            progress_lines: false,
        }
    }
}

impl Formatting {
    /// Effective style of a text item: the item's own style over "All".
    pub fn text_style(&self, item: TextItem) -> TextStyle {
        let all = self.text_styles.iter().find(|(i, _)| *i == TextItem::All).map(|(_, s)| s.clone()).unwrap_or_default();
        if item == TextItem::All {
            return all;
        }
        match self.text_styles.iter().find(|(i, _)| *i == item) {
            Some((_, s)) => s.clone(),
            None => all,
        }
    }

    pub fn set_text_style(&mut self, item: TextItem, style: TextStyle) {
        if let Some(e) = self.text_styles.iter_mut().find(|(i, _)| *i == item) {
            e.1 = style;
        } else {
            self.text_styles.push((item, style));
        }
    }

    pub fn gridline(&self, kind: GridKind) -> Gridline {
        self.gridlines.iter().find(|g| g.kind == kind).cloned().unwrap_or(Gridline::new(kind, LineType::None, SILVER))
    }

    pub fn set_gridline(&mut self, g: Gridline) {
        if let Some(e) = self.gridlines.iter_mut().find(|x| x.kind == g.kind) {
            *e = g;
        } else {
            self.gridlines.push(g);
        }
    }
}

