use {
    std::{
        fmt,
        num::NonZero,
    },
    enum_iterator::{
        Sequence,
        all,
    },
    rocket::{
        State,
        http::{
            impl_from_uri_param_identity,
            uri,
        },
        request::FromParam,
        response::content::RawHtml,
        uri,
    },
    rocket_util::{
        ToHtml,
        html,
    },
    url::Url,
    crate::{
        Config,
        Error,
        MoleculeExt as _,
        Tab,
        external_link,
        molecules,
        page,
        proto::FormMolecule,
    },
};

pub(crate) enum Source {
    Computation {
        url: &'static str,
    },
    Critelli {
        url_part: &'static str,
    },
    CritelliComputation {
        url_part: &'static str,
    },
    CritelliPrivate {
        url_part: &'static str,
        file_stem: &'static str,
    },
    Official {
        zlbb_id: &'static str,
    },
    OfficialNonLb,
    Other {
        url: &'static str,
    },
    Tutorial,
    Zlbb {
        zlbb_id: &'static str,
        url: &'static str,
    },
}

impl Source {
    pub(crate) fn url(&self) -> Option<Url> {
        match self {
            Self::Critelli { url_part } | Self::CritelliComputation { url_part } | Self::CritelliPrivate { url_part, .. } => Some(format!("https://events.critelli.technology/{url_part}").parse().unwrap()),
            Self::Official { .. } | Self::OfficialNonLb | Self::Tutorial => None,
            Self::Computation { url } | Self::Other { url } | Self::Zlbb { url, .. } => Some(url.parse().unwrap()),
        }
    }
}

fn computation(url: &'static str) -> Source {
    Source::Computation { url }
}

fn critelli(url_part: &'static str) -> Source {
    Source::Critelli { url_part }
}

fn critelli_computation(url_part: &'static str) -> Source {
    Source::CritelliComputation { url_part }
}

fn critelli_private(url_part: &'static str, file_stem: &'static str) -> Source {
    Source::CritelliPrivate { url_part, file_stem }
}

fn official(zlbb_id: &'static str) -> Source {
    Source::Official { zlbb_id }
}

#[allow(unused)] // intermittently useful in case the leaderboard takes a while to update
fn official_non_lb() -> Source {
    Source::OfficialNonLb
}

fn other(url: &'static str) -> Source {
    Source::Other { url }
}

fn tutorial() -> Source {
    Source::Tutorial
}

fn zlbb(zlbb_id: &'static str, url: &'static str) -> Source {
    Source::Zlbb { zlbb_id, url }
}

macro_rules! puzzles {
    ($($variant:ident => $name:literal, $source:expr,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Sequence)]
        pub(crate) enum Puzzle {
            $($variant,)*
        }

        impl Puzzle {
            pub(crate) fn url_part(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)*
                }
            }

            pub(crate) fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            pub(crate) fn source(&self) -> Source {
                match self {
                    $(Self::$variant => $source,)*
                }
            }
        }

        impl<'a> FromParam<'a> for Puzzle {
            type Error = ();

            fn from_param(param: &'a str) -> Result<Self, Self::Error> {
                match param {
                    $(stringify!($variant) => Ok(Self::$variant),)*
                    _ => Err(()),
                }
            }
        }
    };
}

impl Puzzle {
    pub(crate) fn is_custom(&self) -> bool {
        self.source().url().is_some()
    }
}

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl ToHtml for Puzzle {
    fn to_html(&self) -> RawHtml<String> {
        self.as_str().to_html()
    }

    fn push_html(&self, buf: &mut RawHtml<String>) {
        self.as_str().push_html(buf)
    }
}

impl uri::fmt::UriDisplay<uri::fmt::Path> for Puzzle {
    fn fmt(&self, f: &mut uri::fmt::Formatter<'_, uri::fmt::Path>) -> fmt::Result {
        f.write_value(self.url_part())
    }
}

impl_from_uri_param_identity!([uri::fmt::Path] Puzzle);

#[rocket::get("/puzzle")]
pub(crate) async fn index(config: &State<Config>, http_client: &State<reqwest::Client>) -> RawHtml<String> {
    page(config, http_client, Tab::Puzzles, false, "Puzzles — Opus Magnum Molecule Database", html! {
        ul {
            @for puzzle in all::<Puzzle>() {
                li {
                    a(href = uri!(get(puzzle))) : puzzle;
                }
            }
        }
    }, html! {}).await
}

#[rocket::get("/puzzle/<puzzle>")]
pub(crate) async fn get(config: &State<Config>, http_client: &State<reqwest::Client>, puzzle: Puzzle) -> Result<RawHtml<String>, Error> {
    Ok(page(config, http_client, Tab::Puzzles, true, html! {
        : puzzle;
        : " — Opus Magnum Molecule Database";
    }, html! {
        h1 : puzzle;
        p {
            @match puzzle.source() {
                source @ (Source::Critelli { .. } | Source::CritelliComputation { .. } | Source::CritelliPrivate { .. }) => : external_link(config, http_client, source.url().unwrap().as_str(), "Event page").await?;
                source @ (Source::Computation { .. } | Source::Other { .. } | Source::Zlbb { .. }) => : external_link(config, http_client, source.url().unwrap().as_str(), "Source").await?;
                Source::Official { .. } | Source::OfficialNonLb => : "official puzzle";
                Source::Tutorial => : "tutorial puzzle";
            }
        }
        h2 : "Reagents";
        div(class = "row") {
            @for (idx, (molecule, name)) in molecules::molecules()
                .into_iter()
                .flat_map(|(molecule, appearances)| appearances.into_iter().filter_map(move |(iter_puzzle, in_out, name)| (iter_puzzle == puzzle && in_out.is_reagent()).then(|| (molecule.clone(), name))))
                .enumerate()
            {
                div {
                    h3 {
                        @if let Some(name) = name {
                            : name;
                        } else {
                            span(class = "muted") : "unnamed";
                        }
                    }
                    a(href = uri!(crate::index(Some(FormMolecule(molecule.clone())), _))) : molecule.draw(&format!("reagent{idx}"));
                }
            }
        }
        h2 : "Products";
        div(class = "row") {
            @for (idx, (molecule, name)) in molecules::molecules()
                .into_iter()
                .flat_map(|(molecule, appearances)| appearances.into_iter().filter_map(move |(iter_puzzle, in_out, name)| (iter_puzzle == puzzle && in_out.is_product()).then(|| (molecule.clone(), name))))
                .enumerate()
            {
                div {
                    h3 {
                        @if let Some(name) = name {
                            : name;
                        } else {
                            span(class = "muted") : "unnamed";
                        }
                    }
                    a(href = uri!(crate::index(Some(FormMolecule(molecule.clone())), _))) : molecule.draw(&format!("product{idx}"));
                }
            }
        }
    }, html! {}).await)
}

puzzles! {
    AWelcomeToHouseColvan => "A Welcome to House Colvan", zlbb("w2450560971", "https://drive.google.com/drive/folders/1Lk1kj1YERh0yWvgIK89dpd_L7TzLhhTo"),
    AblativeCrystal => "Ablative Crystal", official("P068"),
    AbrasiveParticles => "Abrasive Particles", official("P079"),
    ActivePolymerase => "Active Polymerase", zlbb("w2501728219", "https://reddit.com/r/opus_magnum/comments/fe8l4r/week_6_active_polymerase/"),
    AetherDetector => "Aether Detector", official("P077"),
    AetherReactor => "Aether Reactor", critelli("Week_5_AetherReactor"),
    AirshipFuel => "Airship Fuel", official("P008"),
    AlchemicalJewel => "Alchemical Jewel", official("P035"),
    AlchemicalSlag => "Alchemical Slag", official("P099"),
    AlcoholSeparation => "Alcohol Separation", official("P024"),
    AmalgamatedGoldRing => "Amalgamated Gold Ring", zlbb("w2501727808", "https://reddit.com/r/opus_magnum/comments/ewj8ml/tournament_week_2_amalgamated_gold_ring/"),
    AmeliasCatalyst => "Amelia's Catalyst", critelli("e9d7e303b465bbcba16fc72b0db96bc1"),
    AnimismusBuffer => "Animismus Buffer", official("P104"),
    ArmorFilament => "Armor Filament", official("P020"),
    ArmorPolish => "Armor Polish", official("P213"),
    ArqueritePromotion => "Arquerite Promotion", other("https://discord.com/channels/278707932089155584/296373951800541186/857072161097515049"),
    ArtificialOre => "Artificial Ore", zlbb("w2591419339", "https://discord.com/channels/278707932089155584/296373951800541186/879900850661769278"),
    Asbestos => "Asbestos", critelli("OM2025Weeklies6_Asbestos"),
    AssassinsFilament => "Assassin's Filament", official("P097"),
    BalancedGold => "Balanced Gold", official("P282"),
    BalingFiber => "Baling Fiber", official("P286"),
    BandageThread => "Bandage Thread", official("P248"),
    BeautySalve => "Beauty Salve", official("P246"),
    BerlosDualism => "Berlo's Dualism", critelli("b2567dd6f278003b0048996f5de5b64f"),
    BicrystalTransceiver => "Bicrystal Transceiver", critelli("OM2023_W6_BicrystalTransceiver"),
    BiosteelFilament => "Biosteel Filament", critelli("OM2023_W4_BiosteelFilament"),
    BlackPowder => "Black Powder", critelli("OM2023Weeklies_BlackPowder"),
    BlastCordage => "Blast Cordage", official("P299"),
    BloodStanchingPowder => "Blood-Stanching Powder", official("P087"),
    BlueVitriol => "Blue Vitriol (2024 tournament)", critelli("bb94e99e5b9f4d14791f50e953e6f2bb"),
    BlueVitriolJournal => "Blue Vitriol (Journal issue XI)", official("P241"),
    Boozesort => "Boozesort", critelli_computation("OM2025Weeklies8_Boozesort"),
    BrazingCathode => "Brazing Cathode", critelli("OM2022Weeklies_BrazingCathode"),
    BreathableFluid => "Breathable Fluid", critelli("OM2024Weeklies_BreathableFluid"),
    BulkTransmutation => "Bulk Transmutation", critelli("OM2025week6_Bulk_Transmutation"),
    BuoyantCable => "Buoyant Cable", official("P062"),
    BurningSpiritOfSaturn => "Burning Spirit of Saturn", critelli("d3f9ca519aacb2d782a5388098299acd"),
    CalligraphersInk => "Calligrapher's Ink", official("P277"),
    CalmBeforeTheStorm => "Calm Before the Storm", zlbb("w2450512434", "https://drive.google.com/drive/folders/1JVbrvF7dcTKmYN68eGlWhTryXy1TEnc8"),
    CancerMedicine => "Cancer Medicine", critelli("d40f1593c0731c9325dd5c5b9ed3db4b"),
    CanisterShot => "Canister Shot", official("P270"),
    CelestialThread => "Celestial Thread", official("P101"),
    ChildrensToys => "Children's toys", critelli("f308e34f12f681c32580ee82d0c96c72"),
    ChromaticAberration => "Chromatic Aberration", critelli("26a4f980a1b475197735802f9cb75836"),
    ClimbingRopeFiber => "Climbing Rope Fiber", official("P027"),
    Clusterfgold => "Clusterfgold", critelli("7ce689ab3de9678bfb297994d79f17e1"),
    ColvanBlue => "Colvan Blue", other("https://discord.com/channels/278707932089155584/296373951800541186/851990719530532864"),
    CompoundAnaesthetic => "Compound Anaesthetic", official("P244"),
    ConductiveEnamel => "Conductive Enamel", official("P093"),
    ConnectTheDots => "Connect the Dots", zlbb("w3101135731", "https://reddit.com/r/opus_magnum/comments/eohzw1/opus_magnum_tournament_2020/"),
    CoolEarrings => "Cool Earrings", critelli("OM2023_WO_CoolEarrings"),
    CorporateWasteReduction => "Corporate Waste Reduction", critelli("88fed58ce6219be91a046e9a62a004c7"),
    CouragePotion => "Courage Potion", official("P021"),
    CranberryGlass => "Cranberry Glass", other("https://discord.com/channels/278707932089155584/296373951800541186/864679286073720832"),
    CreativeAccounting => "Creative Accounting", zlbb("w1698785633", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    CrimsonCrystalBisection => "Crimson Crystal Bisection", other("https://discord.com/channels/278707932089155584/296373951800541186/867218205080158208"),
    Critellium => "Critellium", critelli("e8f14a0982aaacbb3254457e77c23a0b"),
    CrystalCompression => "Crystal Compression", other("https://discord.com/channels/278707932089155584/296373951800541186/862141779825655818"),
    CrystallizedAir => "Crystallized Air", critelli("OM2025week5_Crystallized_Air"),
    CultivationTonic => "Cultivation Tonic", official("P249"),
    Cuprite => "Cuprite (2022 weeklies)", critelli("OM2022Weeklies_Cuprite"),
    CupriteJournal => "Cuprite (Journal issue XII)", official("P253"),
    CuqueritePromotion => "Cuquerite Promotion", critelli("af91511ae10c69e531d347ad8656e594"),
    CuriousLipstick => "Curious Lipstick", official("P041"),
    DarkMatterCandidate => "Dark Matter Candidate", critelli("OM2023Weeklies_DarkMatterCandidate"),
    DeepFriedRocketPropellant => "Deep-Fried Rocket Propellant", critelli("OM2024Weeklies_DeepFriedRocketPropellant"),
    DeepFriedUnstableCompound => "Deep-Fried Unstable Compound", critelli("OM2024Weeklies_BSides_DeepFriedUnstableCompound"),
    DehydratedWater => "Dehydrated Water", critelli("OM2022Weeklies_DehydratedWater"),
    DentalAmalgam => "Dental Amalgam (2024 tournament)", critelli("c3dda33075913461bebd7cd7c8759669"),
    DentalAmalgamJournal => "Dental Amalgam (Journal issue XII)", official("P252"),
    DestabilizedNature => "Destabilized Nature", critelli("683142751f1988e34bb824ac9302ed20"),
    DoYouRemember => "Do You Remember", zlbb("w1698787731", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    DurableStitching => "Durable Stitching", official("P296"),
    DwarvenFireWine => "Dwarven Fire Wine", zlbb("w1698786588", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    DyeHard => "Dye Hard", critelli("OM2023Weeklies_DyeHard"),
    ElectrumSeparation => "Electrum Separation", official("P103"),
    ElementalComparator => "Elemental Comparator", critelli_computation("OM2023_W8w_ElementalComparator"),
    ElementalCopper => "Elemental Copper", official("P202"),
    ElementalJewelSetting => "Elemental Jewel Setting", zlbb("w2450512809", "https://drive.google.com/drive/folders/1P7fsijiuJTI-1LKrpT7IMn7nj2V5PAvC"),
    EmbalmingFluid => "Embalming Fluid", official("P108"),
    EmergencyAntidote => "Emergency Antidote", zlbb("w2450512232", "https://drive.google.com/drive/folders/1SL0WExUVLu6_xsvZCA9z29PH6RuFBrBd"),
    EndGame => "End Game", critelli("OM2023_W0_EndGame"),
    EndurancePotion => "Endurance Potion", official("P293"),
    EphemeralMatrix => "Ephemeral Matrix", critelli("5a5504a1f72574d23012a6458d1a29b1"),
    EssenceOfCitrus => "Essence of Citrus", official("P257"),
    EvilOre => "Evil Ore", zlbb("w1698788220", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    ExMateria => "Ex Materia", official("P217"),
    ExperimentalCatalyst => "Experimental Catalyst", official("P220"),
    ExplorersSalve => "Explorer's Salve", official("P059"),
    ExplosiveAlloy => "Explosive Alloy", official("P251"),
    ExplosiveFingerTrap => "Explosive Finger Trap", other("https://discord.com/channels/278707932089155584/296373951800541186/874833948570689577"),
    ExplosiveLogicUnit => "Explosive Logic Unit", computation("https://drive.google.com/drive/folders/1A_GkcZV1fxMt3vII6qGsD_wX2ULzG8Ca"),
    ExplosivePhial => "Explosive Phial", official("P017"),
    ExplosiveVictrite => "Explosive Victrite", official("P100"),
    ExtractionFromSapa => "Extraction from Sapa", critelli("65f67bd69c877c6b90352dea6c440aa1"),
    EyedropsOfIrritation => "Eyedrops Of Irritation", critelli("OM2023Weeklies_EyedropsOfIrritation"),
    EyedropsOfRevelation => "Eyedrops of Revelation", official("P081"),
    FacePowder => "Face Powder", official("P009"),
    FaeroFilament => "Faero Filament", critelli("OM2024Weeklies_FaeroFilament"),
    FerrousWheel => "Ferrous Wheel", zlbb("w2565611826", "https://discord.com/channels/278707932089155584/296373951800541186/869756148788625459"),
    FilmCrystal => "Film Crystal", critelli("Week_4_FilmCrystal"),
    FireworksPowder => "Fireworks Powder", official("P211"),
    FlakeSalt => "Flake Salt", critelli("Week_-1_FlakeSalt"),
    FragrantPowders => "Fragrant Powders", official("P075"),
    FructifyingWater => "Fructifying Water", official("P289"),
    Fulmination => "Fulmination", critelli("Week_6_Fulmination"),
    GalenaSeparation => "Galena Separation", official("P207"),
    Galvanization => "Galvanization", critelli("58aea7d6d7ba1b4e8b73d2219de0a8cc"),
    GeneralAnaesthetic => "General Anaesthetic", official("P086"),
    GildingWax => "Gilding Wax", official("P279"),
    GlitraPaint => "Glitra Paint", official("P275"),
    GlorpsConstruct => "Glorp's Construct", critelli("b721b7ba8e14db667d5ea374eaca9a9e"),
    GoldenThread => "Golden Thread", official("P037"),
    GreenVitriol => "Green Vitriol (2021 weeklies)", zlbb("w2539581468", "https://discord.com/channels/278707932089155584/296373951800541186/859612178902286376"),
    GreenVitriolJournal => "Green Vitriol (Journal issue XI)", official("P240"),
    GrenadePellet => "Grenade Pellet", official("P259"),
    Gunmetal => "Gunmetal", official("P272"),
    HabitabilityDetector => "Habitability Detector", critelli_computation("OM2023_W8_HabitabilityDetector"),
    HairProduct => "Hair Product", official("P016"),
    HangoverCure => "Hangover Cure", official("P013"),
    HealthTonic => "Health Tonic", official("P014"),
    HemisphereChange => "Hemisphere Change", critelli("af2bef37158400f2caf15403a1d3c687"),
    HexstabilizedSalt => "Hexstabilized Salt", official("P091b"),
    HexstabilizedTeulingsMors => "Hexstabilized Teuling's Mors", critelli("OM2023Weeklies_HexstabilizedTeulingsMors"),
    HighExplosive => "High Explosive", official("P271"),
    HighGlossFinish => "High Gloss Finish", zlbb("w2501728349", "https://reddit.com/r/opus_magnum/comments/fhui7x/week_7_high_gloss_finish/"),
    HornSilver => "Horn Silver", zlbb("w2513871683", "https://discord.com/channels/278707932089155584/296373951800541186/849437821918904350"),
    HotIce => "Hot Ice", critelli("OM2022Weeklies_HotIce"),
    HydrophobicWater => "Hydrophobic Water", critelli("om2025week1_Hydrophobic_Water"),
    HydroponicSolution => "Hydroponic Solution", critelli("OM2023_W3_HydroponicSolution"),
    HyperVolatileGas => "Hyper-volatile Gas", official("P106"),
    IcelandicLavaSalt => "Icelandic Lava Salt", critelli("952a099fce7b49281d4b95f0f37dae8e"),
    IgnitionCord => "Ignition Cord", critelli("OM2022Weeklies_IgnitionCord"),
    ImmortalFilament => "Immortal Filament", critelli("483f5c168a293fbed5aaf12990be50cf"),
    ImprovedExplosivePhial => "Improved Explosive Phial", zlbb("w2450508212", "https://drive.google.com/drive/folders/1aRi8dJIu7YPhikm-QXRboJybAr9ZlW0j"),
    InBerlosBasement => "In Berlo's Basement", critelli("66f74ef1aae21439a688b1cc54ca894c"),
    InLocoDispono => "In Loco Dispono", critelli("5b464528478f002ba6866f690bd01f40"),
    InductiveFoil => "Inductive Foil", official("P280"),
    InstantMirrorCoat => "Instant Mirror Coat", critelli("OM2024Weeklies_InstantMirrorCoat"),
    IntumescentLead => "Intumescent Lead", critelli("fc37c3c4183d77bb17bf827ae66c53d7"),
    InvariantMetal => "Invariant Metal", official("P215"),
    InvigoratingTonic => "Invigorating Tonic", official("P291"),
    InvisibleInk => "Invisible Ink", official("P032"),
    JewelBox => "Jewel Box", critelli("OM2025Weeklies1_JewelBox"),
    Lambent29 => "Lambent II/IX", official("P058"),
    Lambent67 => "Lambent LXVII", critelli_computation("0c61ded553925ac6b6d567386c9982b8"),
    LamplightGas => "Lamplight Gas", official("P092"),
    LapidarySaw => "Lapidary Saw", official("P278"),
    LatchHookFireworks => "Latch-Hook Fireworks", critelli("638f26965e21b260086e1919b264eab5"),
    LeachingAgent => "Leaching Agent", official("P263"),
    LeaveNoTrace => "Leave No Trace", critelli("Week_0_LeaveNoTrace"),
    LeaveningAgent => "Leavening Agent", official("P287"),
    LessonArms => "Lesson: Arms", tutorial(),
    LessonBonding => "Lesson: Bonding", tutorial(),
    LessonIntroduction => "Lesson: Introduction", tutorial(),
    LessonPistons => "Lesson: Pistons", tutorial(),
    LessonPivots => "Lesson: Pivots", tutorial(),
    LessonTracks => "Lesson: Tracks", tutorial(),
    LessonTransmutation => "Lesson: Transmutation", tutorial(),
    LifeSensingPotion => "Life-Sensing Potion", official("P030b"),
    LighthouseMirror => "Lighthouse Mirror", official("P258"),
    LithargeSeparation => "Litharge Separation", official("P031b"),
    LocalAnaesthetic => "Local Anaesthetic", critelli("2414043fbe61ace7b2335186a998fcd8"),
    Lodestone => "Lodestone", official("P256"),
    Logistics => "Logistics", critelli("d18b0ff75237478b9f7d27f799222e46"),
    LookAndSay => "Look-And-Say", critelli_computation("OM2024Weeklies_LookAndSay"),
    LubricatingFilament => "Lubricating Filament", official("P065"),
    LubricatingSolvents => "Lubricating Solvents", critelli("Week_3_LubricatingSolvents"),
    LuminousVapor => "Luminous Vapor", official("P247"),
    Lustre => "Lustre", official("P090"),
    LustrousSyrup => "Lustrous Syrup", critelli("OM2022Weeklies_LustrousSyrup"),
    MagisteryOfSaturn => "Magistery of Saturn", critelli("9679227965273efecaca286dadb4dabf"),
    Marlstone => "Marlstone", official("P288"),
    MartialRegulus => "Martial Regulus", critelli("OM2022Weeklies_MartialRegulus"),
    MaterialSalvage => "Material Salvage", critelli("om2025break_Material_Salvage"),
    MemoryLane => "Memory Lane", critelli_computation("OM2025week8_Memory_Lane"),
    MetalCalculus => "Metal Calculus", computation("https://reddit.com/r/opus_magnum/comments/flp70t/the_final_week_of_the_tournament_metal_calculus/"),
    MetalDivision => "Metal Division", official("P208"),
    MetallicTincture => "Metallic Tincture", official("P206"),
    MiraculousAutosalt => "Miraculous Autosalt", zlbb("w1698787102", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    MiraculousDentifrice => "Miraculous Dentifrice", official("P266"),
    MirrorPolish => "Mirror Polish", official("P269"),
    MirroringAmalgam => "Mirroring Amalgam", official("P298"),
    MistOfDousing => "Mist of Dousing", zlbb("w2450512021", "https://drive.google.com/drive/folders/1JX9JEdzXfFgn1-z4Yno_oMjSHHg8eGxE"),
    MistOfGlaciation => "Mist of Glaciation", official("P283"),
    MistOfHallucination => "Mist of Hallucination", official("P038"),
    MistOfIncapacitation => "Mist of Incapacitation", official("P018"),
    MixedUseLubricant => "Mixed-Use Lubricant", official("P205"),
    MoonlightCatalyst => "Moonlight Catalyst", critelli("OM2025Weeklies11_MoonlightCatalyst"),
    MooringCable => "Mooring Cable", official("P255"),
    MyArmsAreBound => "My Arms Are Bound", critelli_private("ea51e6dcb8f5b83c0ef6e6b1f965d57a", "MyArmsAreBound"),
    NightmareFuel => "Nightmare Fuel", critelli("OM2022Weeklies_NightmareFuel"),
    Nylon => "Nylon", critelli("OM2025Weeklies9_Nylon"),
    OneLastFavor => "One Last Favor", critelli("eee9d425ba613ebb491183f4f6349af9"),
    OrangeVitriol => "Orange Vitriol", critelli("547de89828787801144566f080b0b213"),
    OrnamentalPlating => "Ornamental Plating", critelli("OM2024Weeklies_OrnamentalPlating"),
    Overloaded => "Overloaded", zlbb("w2501728107", "https://reddit.com/r/opus_magnum/comments/f7674d/week_5_overloaded/"),
    PalatableTissue => "Palatable Tissue", critelli("OM2024Weeklies_PalatableTissue"),
    Panacea => "Panacea", zlbb("w1698789743", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    PanaceaToPoison => "Panacea to Poison", zlbb("w2450511665", "https://drive.google.com/drive/folders/1-Ky7fk653U6Zr9bPgEzGDtI3QX7vVJlE"),
    ParadeRocketFuel => "Parade-Rocket Fuel", official("P082"),
    ParetoPoppers => "Pareto Poppers", critelli("OM2025Weeklies4_ParetoPoppers"),
    PassThroughAlloy => "Pass-Through Alloy", critelli("OM2025week7_Pass-Through_Alloy"),
    PatternMetal => "Pattern Metal", official("P221"),
    PhilosophersCatalyst => "Philosopher's Catalyst", critelli("OM2022Weeklies_PhiloCatalyst"),
    PigIron => "Pig Iron", official("P260"),
    PitchDropExperiment => "Pitch Drop Experiment", critelli("OM2024Weeklies_PitchDropExperiment"),
    Plastic => "Plastic", critelli("ac6ab9dc43b0ac7ad9277c25ed5375e1"),
    PotentPainkillers => "Potent Painkillers", critelli("Week_7_PotentPainkillers"),
    PotentPotables => "Potent Potables", zlbb("w2501727721", "https://reddit.com/r/opus_magnum/comments/et5lyo/tournament_week_1_potent_potables/"),
    PousseCafe => "Pousse-Café", critelli("f883eb7701f420e1b1960eabe37b7fc1"),
    PrecisionMachineOil => "Precision Machine Oil", official("P012"),
    PreservativeSalt => "Preservative Salt", official("P060"),
    PreservingWax => "Preserving Wax", official("P267"),
    ProbeModule => "Probe Module", critelli("OM2023_W5_ProbeModule"),
    ProofOfCompleteness => "Proof of Completeness", official("P069"),
    ProspectorsSolvent => "Prospector's Solvent", official("P261"),
    PurifiedGold => "Purified Gold", official("P036"),
    QuickeningCordial => "Quickening Cordial", official("P290"),
    QuietHours => "Quiet Hours", critelli("ff6feb9ee69a0450a117eb2a7c7de784"),
    QuintessentialAerogel => "Quintessential Aerogel", critelli("OM2022Weeklies_QuintAerogel"),
    QuintessentialCatalyst => "Quintessential Catalyst", other("https://discord.com/channels/278707932089155584/296373951800541186/877363315687436349"),
    QuintessentialExplosive => "Quintessential Explosive", critelli("OM2023Weeklies_QuintExplosive"),
    QuintessentialMedium => "Quintessential Medium", official("P107"),
    QuintessentialStabilizer => "Quintessential Stabilizer", zlbb("w2450512626", "https://drive.google.com/drive/folders/1sommL5qdrN8fa0-D_dnwEJxVlwu34QgT"),
    RadioReceivers => "Radio Receivers", critelli("Week_8_RadioReceivers"),
    RatPoison => "Rat Poison", official("P074"),
    RavarisRage => "Ravari's Rage", other("https://drive.google.com/drive/folders/1iy7KDmdkO4HGbjD1_jX_qb8mQ2IeSmS1"),
    RavarisRoad => "Ravari's Road", critelli("OM2025Weeklies7_RavarisRoad"),
    RavarisWheel => "Ravari's Wheel", official("P064"),
    ReactiveCinnabar => "Reactive Cinnabar", official("P056"),
    ReactiveGold => "Reactive Gold", official("P095"),
    ReactiveLead => "Reactive Lead", official("P210"),
    RealgarSeparation => "Realgar Separation", official("P264"),
    RecipeForDisaster => "Recipe for Disaster", critelli("OM2025Weeklies10_RecipeForDisaster"),
    ReclaimedGold => "Reclaimed Gold", official("P254"),
    ReconstructedSolvent => "Reconstructed Solvent", official("P084"),
    RefinedBronze => "Refined Bronze", official("P067"),
    RefinedGold => "Refined Gold", official("P010"),
    ResonantCrystal => "Resonant Crystal", official("P066"),
    ResonantDust => "Resonant Dust", official("P243"),
    RetroRefining => "Retro Refining", critelli("OM2024Weeklies_RetroRefining"),
    RibbedFletching => "Ribbed Fletching", official("P273"),
    RingEnlargement => "Ring Enlargement", critelli("OM2023Weeklies_RingEnlargement"),
    RocketPropellant => "Rocket Propellant", official("P019"),
    Rosewater => "Rosewater", official("P285"),
    Roshambonite => "Roshambonite", critelli("b9ec009e82738509a09e554f359fb300"),
    RustRemoval => "Rust Removal", critelli("Week_1_RustRemoval"),
    SailclothThread => "Sailcloth Thread", official("P061"),
    SaltPackagingFactory => "Salt Packaging Factory", critelli("OM2023Weeklies_SaltPackagingFactory"),
    SaltOfHartshorn => "Salt of Hartshorn", official("P281"),
    SaltOfSaturnByVinegar => "Salt of Saturn by Vinegar", critelli("9cec1eb8ff25a6ee4c73c77176524146"),
    SandOfSuspension => "Sand of Suspension", critelli("OM2025Weeklies2_SandOfSuspension"),
    SaturnsTree => "Saturn's Tree", critelli("9358b0efab6c1f32b850f898b8b2a675"),
    SaveriosTest => "Saverio's Test", official("P201"),
    SaveriosTransformer => "Saverio's Transformer", official("P204"),
    ScrapMetal => "Scrap Metal", critelli("a1dcf5402132b1b4c28d63ef02942db8"),
    SealSolvent => "Seal Solvent", official("P026"),
    SelfOrganizingFluid => "Self-Organizing Fluid", critelli("OM2025week3_Self-Organizing_Fluid"),
    SelfPressurizingGas => "Self-Pressurizing Gas", critelli("OM2023_W1_SelfPressurizingGas"),
    SeptstabilizedSalt => "Septstabilized Salt", critelli("9c14e48d17cebae4165828037b56cc7c"),
    ServinsWheel => "Servin's Wheel", critelli("OM2022Weeklies_ServinsWheel"),
    ShimmeringChain => "Shimmering Chain", official("P295"),
    SigmarsGarden => "Sigmar's Garden", critelli("af2e9ada2b4a888463d1aef200c58eb9"),
    SilverAppleOfDiscord => "Silver Apple of Discord", critelli("3e07b2ebbabd5ea7e9b87f8cd35d679f"),
    SilverCaustic => "Silver Caustic", official("P057"),
    SilverDust => "Silver Dust", official("P203"),
    SilverPaint => "Silver Paint", official("P076"),
    Simulacrum => "Simulacrum", critelli("ff3f211965e5ba83ac8a080335ddd04e"),
    SleepingTonic => "Sleeping Tonic", official("P242"),
    SmogNeutralization => "Smog Neutralization", critelli("OM2025Weeklies5_SmogNeutralization"),
    SnowAmputation => "Snow Amputation", critelli("0d8384f8b042a93f2cbb1af82c339b9c"),
    SodaAsh => "Soda Ash", official("P265"),
    SolderWire => "Solder Wire", official("P209"),
    SoothingSalve => "Soothing Salve", critelli("Week_2_SoothingSalve"),
    SophickMercury => "Sophick Mercury (2024 weeklies)", critelli("OM2024Weeklies_SophickMercury"),
    SophickMercuryJournal => "Sophick Mercury (Journal issue XII)", official("P250"),
    SparkingPyrite => "Sparking Pyrite", official("P274"),
    SpecialAmaro => "Special Amaro", official("P083"),
    SpyglassCrystal => "Spyglass Crystal", official("P063"),
    StabilizedEverything => "Stabilized Everything", other("https://discord.com/channels/278707932089155584/296373951800541186/882448331119427604"),
    StabilizedGold => "Stabilized Gold", critelli("OM2022Weeklies_StabilizedGold"),
    StabilizedWater => "Stabilized Water", official("P007"),
    StainRemover => "Stain Remover", official("P034"),
    StaminaPotion => "Stamina Potion", official("P015"),
    SteelWool => "Steel Wool", official("P268"),
    StormSensingPotion => "Storm-Sensing Potion", official("P294"),
    SuperconductiveCopper => "Superconductive Copper", other("https://discord.com/channels/278707932089155584/296373951800541186/854533289816358922"),
    SurrenderFlare => "Surrender Flare", official("P022"),
    SurveyingMagnet => "Surveying Magnet", official("P262"),
    SuspiciouslyStableSubstance => "Suspiciously Stable Substance", critelli("OM2022Weeklies_SSS"),
    SutureThread => "Suture Thread", official("P085"),
    SwampFiber => "Swamp Fiber", zlbb("w2501727889", "https://reddit.com/r/opus_magnum/comments/f05mp5/week_3_swamp_fiber/"),
    SweeperRod => "Sweeper Rod", critelli("OM2022Weeklies_SweeperRod"),
    SwordAlloy => "Sword Alloy", official("P033"),
    SynthesisViaAlcohol => "Synthesis via Alcohol", official("P071"),
    SyntheticMalachite => "Synthetic Malachite", official("P109"),
    Taricene => "Taricene", critelli("21042bbbbef69aef2000df9b97a4df9b"),
    TaxFraud => "Tax Fraud", critelli("a39b83445002fbdea218e8b184a9e478"),
    TheAmazingEverythingMachine => "The Amazing Everything-Machine", critelli_computation("OM2025week4_The_Amazing_Everything-Machine"),
    ThermalFuse => "Thermal Fuse", critelli("d3689000418b9687654554f28324d8d0"),
    ThermicCapacitor => "Thermic Capacitor", critelli("om2025week2_Thermic_Capacitor"),
    ThermiteTape => "Thermite Tape", critelli("OM2025Weeklies3_ThermiteTape"),
    TimingCrystal => "Timing Crystal", official("P042"),
    Tinsel => "Tinsel", critelli("bf33acfc5ce6933ae39b8ee5deb663af"),
    TonicOfHydration => "Tonic of Hydration", official("P089"),
    TonicOfTransmogrification => "Tonic of Transmogrification", critelli_computation("OM2023Weeklies_TransTonic"),
    TouchGrass => "Touch Grass", critelli("OM2024Weeklies_TouchGrass"),
    Touchstone => "Touchstone (2024 tournament)", critelli("6f37903681423b320da82fb57900291d"),
    TouchstoneJournal => "Touchstone (Journal issue X)", official("P245"),
    Transmutation110 => "Transmutation CX", critelli_computation("Week_9_TransmutationCX"),
    UmbralMascara => "Umbral Mascara", official("P292"),
    UniversalCompound => "Universal Compound", official("P072"),
    UniversalSolvent => "Universal Solvent", official("P043"),
    UnstableCompound => "Unstable Compound", official("P040"),
    UnstableSovrium => "Unstable Sovrium", critelli("OM2024Weeklies_UnstableSovrium"),
    Unwinding => "Unwinding", zlbb("w1611998067", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    VaccineTemplate => "Vaccine Template", critelli("5d936a1ca336f6658f097260eda56a8f"),
    VanBerlosChain => "Van Berlo's Chain", official("P055"),
    VanBerlosPivots => "Van Berlo's Pivots", official("P096"),
    VanBerlosWheel => "Van Berlo's Wheel", official("P054"),
    VanishingMaterial => "Vanishing Material", official("P105"),
    VaporOfLevity => "Vapor of Levity", official("P078"),
    VaporizedPropellant => "Vaporized Propellant", other("https://discord.com/channels/278707932089155584/296373951800541186/872298625798119455"),
    VaporousSolvent => "Vaporous Solvent", official("P098"),
    VermilionPigment => "Vermilion Pigment", official("P276"),
    VeryDarkThread => "Very Dark Thread", official("P029"),
    ViaMedia => "Via Media", official("P284"),
    ViaQuicksilver => "Via Quicksilver", official("P216"),
    VirulentVector => "Virulent Vector", zlbb("w1698785238", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    ViscousAdhesive => "Viscous Adhesive", critelli("OM2023Weeklies_ViscousAdhesive"),
    ViscousSludge => "Viscous Sludge", official("P080"),
    VisillaryAnaesthetic => "Visillary Anaesthetic", official("P102"),
    VolatilityAndTranquility => "Volatility and Tranquility", zlbb("w2501727977", "https://reddit.com/r/opus_magnum/comments/f3n89y/week_4_volatility_and_tranquility/"),
    VoltaicCoil => "Voltaic Coil", official("P039"),
    WakefulnessPotion => "Wakefulness Potion", official("P088"),
    WarmingTonic => "Warming Tonic", official("P028"),
    WarpFuel => "Warp Fuel", critelli("OM2023_W7_WarpFuel"),
    WasteReclamation => "Waste Reclamation", critelli("OM2023_W2_WasteReclamation"),
    WaterPurifier => "Water Purifier", official("P025"),
    WaterproofSealant => "Waterproof Sealant", official("P011"),
    WeldingThermite => "Welding Thermite", official("P094"),
    WheelInversion => "Wheel Inversion", computation("https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    WheelRepresentation => "Wheel Representation", official("P070"),
    WireFormingAndUnforming => "Wire Forming and Unforming", zlbb("w1698784331", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    XylemSubstitute => "Xylem Substitute", official("P297"),
}

#[test]
fn test_puzzles_sorted() -> std::io::Result<()> {
    use itertools::Itertools as _;

    if !all::<Puzzle>().is_sorted_by_key(|puzzle| puzzle.as_str()) {
        std::fs::write("actual.txt", all::<Puzzle>().join("\n"))?;
        std::fs::write("sorted.txt", all::<Puzzle>().sorted_by_key(|puzzle| puzzle.as_str()).join("\n"))?;
        panic!("puzzles not sorted by name")
    }
    Ok(())
}
