#![allow(unused)] //TODO

use {
    std::{
        borrow::Cow,
        str::FromStr,
        time::Duration,
    },
    omsim_rs::data::*,
    rocket::response::content::RawHtml,
    rocket_util::{
        ToHtml,
        html,
    },
};

#[derive(Clone)]
pub(crate) enum Restriction {
    /// Combines multiple restrictions, all of which must hold.
    All(&'static [Restriction]),
    /// All reagents available in the puzzle must be on the board.
    AllReagentsPlaced,
    /// Must validate, no overlap, no duplicate Berlo/Ravari/disposal/proliferation, no duplicate reagents or products (by ID), no quantum track, no cabinet violations.
    DefaultPostDrm,
    /// Like `DefaultPostDrm` but also no halt instructions and no polymer outputs rotated relative to each other.
    DefaultPreDrm,
    /// The two metrics must have equal values.
    Eq(Metric, Metric),
    /// The first metric's value must be lower than or equal to the second metric's.
    Le(Metric, Metric),
    /// The solution must enter a steady state where it keeps producing every product.
    Looping,
    /// Gold waste must not accumulate in the steady state.
    NoGoldWaste,
    /// No dropping a molecule of the correct footprint but the wrong atom types or bonds onto an output glyph.
    NoOutputBondConditionals,
    /// No dropping a molecule of the correct shape but the wrong atom types onto an output glyph.
    NoOutputConditionals,
    /// No breaking any triplex bonds.
    NoTriplexUnbonding,
    /// No restrictions whatsoever, any file is accepted as a valid solution. Typically used for aesthetics/shitpost metrics.
    None,
    /// The second variable reagent may not be grabbed.
    OneVariableReagentPull,
    /// No track parts.
    Trackless,
    /// Modifies the given restriction to change the conduits the solution is matched against. Can be used to define freespace conduits.
    WithConduits(&'static Restriction, Vec<Conduit>),
    /// Modifies the given restriction to both allow and require the special overlapped parts used for Miraculous Autosalt.
    WithMiraculousAutosalt(&'static Restriction),
    /// Modifies the given restriction to both allow and require the special overlapped parts used for Ravari's Rage.
    WithRavarisRage(&'static Restriction),
}

impl ToHtml for Restriction {
    fn to_html(&self) -> RawHtml<String> {
        html! {
            @match self {
                Self::All(restrictions) => @if let Some((first, rest)) = restrictions.split_first() {
                    : first;
                    @for restriction in rest {
                        : " & ";
                        : restriction;
                    }
                } else {
                    : Self::None;
                }
                Self::AllReagentsPlaced => : "all reagents placed";
                Self::DefaultPostDrm => {
                    : "default restrictions post-";
                    i : "De Re Metallica";
                }
                Self::DefaultPreDrm => {
                    : "default restrictions pre-";
                    i : "De Re Metallica";
                }
                Self::Eq(m1, m2) => {
                    : m1;
                    : " = ";
                    : m2;
                }
                Self::Le(m1, m2) => {
                    : m1;
                    : " ≤ ";
                    : m2;
                }
                Self::Looping => : "looping";
                Self::NoGoldWaste => : "no gold waste";
                Self::NoOutputBondConditionals => : "no output bond conditionals";
                Self::NoOutputConditionals => : "no output conditionals";
                Self::NoTriplexUnbonding => : "no triplex unbonding";
                Self::None => : "no restrictions";
                Self::OneVariableReagentPull => : "the second variable reagent may not be grabbed";
                Self::Trackless => : "trackless";
                Self::WithConduits(restriction, _) => {
                    : restriction;
                    : ", with modified conduits"; //TODO give details on conduits?
                }
                Self::WithMiraculousAutosalt(restriction) | Self::WithRavarisRage(restriction) => {
                    : restriction;
                    : ", with special overlapped glyphs";
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) enum Metric {
    Aesthetics,
    AreaInf,
    AreaV,
    Arms,
    Const(i32),
    Cost,
    Cycles,
    Div(&'static Metric, &'static Metric),
    HeightV,
    If(&'static Restriction, &'static Metric, &'static Metric),
    Instructions,
    Latency,
    MechCost,
    Parts(PartType),
    Product(&'static Metric, &'static Metric),
    Rate,
    Shitpost,
    SpeedsolveMins,
    Sum(&'static [Metric]),
    Ties(Cow<'static, [Metric]>),
    Tracks,
    VintageInstructions, // defined as `instructions with hotkey EQWSGT` https://discord.com/channels/278707932089155584/879900850661769278/879901525646905354
    WidthV,
}

impl Metric {
    pub(crate) const fn ties(submetrics: &'static [Self]) -> Self {
        Self::Ties(Cow::Borrowed(submetrics))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown Critelli metric expression: {0}")]
pub(crate) struct ParseError(String);

impl FromStr for Metric {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        //TODO parse like events.critelli.technology, see https://dezip.org/v1/7/http/events.critelli.technology/static/events.zip/events/static/verify.js?line=39#L39 (evaluateMetricExpression)
        Ok(match s {
            "cost" => Self::Cost,
            "cycles" => Self::Cycles,
            "instructions" => Self::Instructions,
            _ => return Err(ParseError(s.to_owned())),
        })
    }
}

impl ToHtml for Metric {
    fn to_html(&self) -> RawHtml<String> {
        html! {
            @match self {
                Self::Aesthetics => : "Aesthetics";
                Self::AreaInf => : "Area@∞";
                Self::AreaV => : "Area@V";
                Self::Arms => : "Arms";
                Self::Const(n) => : n;
                Self::Cost => : "Cost";
                Self::Cycles => : "Cycles";
                Self::Div(m1, m2) => {
                    : m1;
                    : " / ";
                    : m2;
                }
                Self::HeightV => : "Height@V";
                Self::If(restriction, m1, m2) => {
                    : "(if ";
                    : restriction;
                    : " then ";
                    : m1;
                    : " else ";
                    : m2;
                    : ")";
                }
                Self::Instructions => : "Instructions";
                Self::Latency => : "Latency";
                Self::MechCost => : "Mechanism Cost";
                Self::Parts(ty) => @match ty {
                    PartType::Input => : "Reagents";
                    PartType::Output => : "Regular Products";
                    PartType::PolymerOutput => : "Polymer Products";
                    PartType::Arm => : "Regular Arms";
                    PartType::BiArm => : "Bi-Arms";
                    PartType::TriArm => : "Tri-Arms";
                    PartType::HexArm => : "Hex-Arms";
                    PartType::PistonArm => : "Piston Arms";
                    PartType::Track => : "Track Parts";
                    PartType::Berlo => : "Van Berlo's Wheels";
                    PartType::Equilibrium => : "Equilibrium Glyphs";
                    PartType::Bonding => : "Regular Bonders";
                    PartType::MultiBonding => : "Multibonders";
                    PartType::Unbonding => : "Debonders";
                    PartType::Calcification => : "Calcification Glyphs";
                    PartType::Projection => : "Projection Glyphs";
                    PartType::Purification => : "Purification Glyphs";
                    PartType::Duplication => : "Duplication Glyphs";
                    PartType::Animismus => : "Animismus Glyphs";
                    PartType::Unification => : "Unification Glyphs";
                    PartType::Dispersion => : "Dispersion Glyphs";
                    PartType::TriplexBonding => : "Triplex Bonders";
                    PartType::Disposal => : "Disposal Glyphs";
                    PartType::Conduit => : "Conduits";
                }
                Self::Product(m1, m2) => {
                    : m1;
                    : " × ";
                    : m2;
                }
                Self::Rate => : "Rate";
                Self::SpeedsolveMins => : "Minutes to Solve";
                Self::Shitpost => : "Shitpost";
                Self::Sum(metrics) => @if let Some((first, rest)) = metrics.split_first() {
                    : first;
                    @for metric in rest {
                        : " + ";
                        : metric;
                    }
                } else {
                    : Self::Const(0);
                }
                Self::Ties(metrics) => @if let Some((first, rest)) = metrics.split_first() {
                    : first;
                    @for metric in rest {
                        : " > ";
                        : metric;
                    }
                } else {
                    : Self::Const(0);
                }
                Self::Tracks => : "Track Hexes";
                Self::VintageInstructions => : "Vintage Instructions";
                Self::WidthV => : "Width@V";
            }
        }
    }
}

pub(crate) fn tournament2019metrics(sum: &'static [Metric]) -> Vec<(Restriction, Metric)> {
    vec![
        (Restriction::DefaultPreDrm, Metric::Ties(Cow::Borrowed(&[Metric::Cycles, Metric::Cost]))),
        (Restriction::DefaultPreDrm, Metric::Ties(Cow::Borrowed(&[Metric::AreaV, Metric::Cycles]))),
        (Restriction::DefaultPreDrm, Metric::Ties(Cow::Borrowed(&[Metric::Cost, Metric::AreaV]))),
        (Restriction::DefaultPreDrm, Metric::Sum(sum)),
    ]
}

pub(crate) enum ComputationMetric {
    AverageNoVary(Metric),
    GeoMeanNoVary(Metric),
    Max(Metric),
    Min(Metric),
    RestrictedMax(Metric),
}

impl ToHtml for ComputationMetric {
    fn to_html(&self) -> RawHtml<String> {
        html! {
            @match self {
                Self::AverageNoVary(metric) => {
                    : "(average of all variants, no mid-solve reagent variation) ";
                    : metric;
                }
                Self::GeoMeanNoVary(metric) => {
                    : "(geometric mean of all variants, no mid-solve reagent variation) ";
                    : metric;
                }
                Self::Max(metric) => {
                    : "(maximum of all variants) ";
                    : metric;
                }
                Self::Min(metric) => {
                    : "(minimum of all variants) ";
                    : metric;
                }
                Self::RestrictedMax(metric) => {
                    : "(maximum of select variants) ";
                    : metric;
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) enum Metrics {
    Unknown,
    None,
    Normal(Cow<'static, [(Restriction, Metric)]>),
    Computation(&'static [(Restriction, ComputationMetric)]),
}
