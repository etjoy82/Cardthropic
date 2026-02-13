//! Variant metadata registry (labels, emoji, availability flags).
//!
//! Keep this aligned with `variant_engine` so every GameMode has:
//! - a user-facing spec here
//! - an engine implementation/stub in `variant_engine`.

use crate::game::GameMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantSpec {
    pub mode: GameMode,
    pub id: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub engine_ready: bool,
    pub settings_placeholder: &'static str,
}

pub trait SolitaireVariant: Sync {
    fn spec(&self) -> VariantSpec;
}

#[derive(Debug, Clone, Copy)]
pub struct KlondikeVariant;

#[derive(Debug, Clone, Copy)]
pub struct SpiderVariant;

#[derive(Debug, Clone, Copy)]
pub struct FreecellVariant;

impl SolitaireVariant for KlondikeVariant {
    fn spec(&self) -> VariantSpec {
        KLONDIKE_SPEC
    }
}

impl SolitaireVariant for SpiderVariant {
    fn spec(&self) -> VariantSpec {
        SPIDER_SPEC
    }
}

impl SolitaireVariant for FreecellVariant {
    fn spec(&self) -> VariantSpec {
        FREECELL_SPEC
    }
}

const KLONDIKE_SPEC: VariantSpec = VariantSpec {
    mode: GameMode::Klondike,
    id: "klondike",
    label: "Klondike",
    emoji: "🥇",
    engine_ready: true,
    settings_placeholder: "",
};

const SPIDER_SPEC: VariantSpec = VariantSpec {
    mode: GameMode::Spider,
    id: "spider",
    label: "Spider",
    emoji: "🕷️",
    engine_ready: false,
    settings_placeholder: "Spider settings will appear once Spider is playable.",
};

const FREECELL_SPEC: VariantSpec = VariantSpec {
    mode: GameMode::Freecell,
    id: "freecell",
    label: "FreeCell",
    emoji: "🗽",
    engine_ready: false,
    settings_placeholder: "FreeCell settings will appear once FreeCell is playable.",
};

const KLONDIKE_VARIANT: KlondikeVariant = KlondikeVariant;
const SPIDER_VARIANT: SpiderVariant = SpiderVariant;
const FREECELL_VARIANT: FreecellVariant = FreecellVariant;

const VARIANTS: [&'static dyn SolitaireVariant; 3] =
    [&KLONDIKE_VARIANT, &SPIDER_VARIANT, &FREECELL_VARIANT];

const VARIANT_SPECS: [VariantSpec; 3] = [KLONDIKE_SPEC, SPIDER_SPEC, FREECELL_SPEC];

pub fn all_variants() -> &'static [&'static dyn SolitaireVariant] {
    &VARIANTS
}

pub fn variant_for_mode(mode: GameMode) -> &'static dyn SolitaireVariant {
    match mode {
        GameMode::Klondike => &KLONDIKE_VARIANT,
        GameMode::Spider => &SPIDER_VARIANT,
        GameMode::Freecell => &FREECELL_VARIANT,
    }
}

#[cfg(test)]
pub fn all_variant_specs() -> &'static [VariantSpec] {
    &VARIANT_SPECS
}

pub fn spec_for_mode(mode: GameMode) -> &'static VariantSpec {
    VARIANT_SPECS
        .iter()
        .find(|spec| spec.mode == mode)
        .unwrap_or(&VARIANT_SPECS[0])
}

pub fn spec_for_id(id: &str) -> Option<&'static VariantSpec> {
    VARIANT_SPECS.iter().find(|spec| spec.id == id)
}
