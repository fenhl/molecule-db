#![allow(unused)] //TODO

use omsim_rs::data::*;

#[derive(Clone)]
pub(crate) enum Restriction {
    /// Combines multiple restrictions, all of which must hold.
    All(&'static [Restriction]),
    /// All reagents available in the puzzle must be on the board.
    AllReagentsPlaced,
    /// Like `DefaultPostDrm` but also no halt instructions and no polymer outputs rotated relative to each other.
    DefaultPreDrm,
    /// Must validate, no overlap, no duplicate Berlo/Ravari/disposal/proliferation, no duplicate reagents or products (by ID), no quantum track, no cabinet violations.
    DefaultPostDrm,
    /// The two metrics must have equal values.
    Eq(Metric, Metric),
    /// The solution must enter a steady state where it keeps producing every product.
    Looping,
    /// Gold waste must not accumulate in the steady state.
    NoGoldWaste,
    /// No dropping a molecule of the correct shape but the wrong atom types onto an output glyph.
    NoOutputConditionals,
    /// No dropping a molecule of the correct footprint but the wrong atom types or bonds onto an output glyph.
    NoOutputBondConditionals,
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
    Sum(&'static [Metric]),
    Ties(&'static [Metric]),
    Tracks,
    VintageInstructions, // defined as `instructions with hotkey EQWSGT` https://discord.com/channels/278707932089155584/879900850661769278/879901525646905354
    WidthV,
}

pub(crate) fn tournament2019metrics(sum: &'static [Metric]) -> Vec<(Restriction, Metric)> {
    vec![
        (Restriction::DefaultPreDrm, Metric::Ties(&[Metric::Cycles, Metric::Cost])),
        (Restriction::DefaultPreDrm, Metric::Ties(&[Metric::AreaV, Metric::Cycles])),
        (Restriction::DefaultPreDrm, Metric::Ties(&[Metric::Cost, Metric::AreaV])),
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

pub(crate) enum Part {
    Bonder,
    Unbonder,
}
