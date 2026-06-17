use {
    std::{
        borrow::Cow,
        collections::{
            HashMap,
            HashSet,
        },
        fmt,
        iter,
        num::NonZero,
    },
    bitvec::prelude::*,
    collect_mac::collect,
    enum_iterator::{
        Sequence,
        all,
    },
    itermore::{
        IterArrayCombinationsWithReps as _,
        IterArrayWindows as _,
    },
    itertools::Itertools as _,
    nonempty_collections::{
        IntoIteratorExt as _,
        IntoNonEmptyIterator as _,
        NonEmptyIterator as _,
        NEVec,
    },
    omsim_rs::data::*,
    rand::{
        prelude::*,
        rng,
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
    roman_numerals::ToRoman as _,
    url::Url,
    crate::{
        Config,
        Error,
        IfNoneMatch,
        MoleculeExt as _,
        StaticPageResponse,
        Tab,
        external_link,
        molecules,
        puzzle::OfficialCollection::*,
        proto::FormMolecule,
        static_page,
    },
};

pub(crate) enum OfficialCollection {
    Prologue,
    Campaign(u8),
    Appendix,
    Drm(u8),
    Journal(u8, u8, &'static [&'static str]),
}

impl ToHtml for OfficialCollection {
    fn to_html(&self) -> RawHtml<String> {
        fn join<T: fmt::Display>(elts: impl IntoIterator<Item = T>) -> Option<String> {
            elts.try_into_nonempty_iter().map(|iter| {
                let (first, rest) = iter.next();
                let mut rest = rest.fuse();
                match (rest.next(), rest.next()) {
                    (None, _) => first.to_string(),
                    (Some(second), None) => format!("{first} and {second}"),
                    (Some(second), Some(third)) => {
                        let mut rest = [second, third].into_nonempty_iter().chain(rest).collect::<NEVec<_>>();
                        let last = rest.pop().expect("rest contains at least second and third");
                        format!("{first}; {}; and {last}", rest.into_iter().format("; "))
                    }
                }
            })
        }

        html! {
            @match self {
                Self::Prologue => : "prologue";
                Self::Campaign(chapter) => {
                    : "campaign chapter ";
                    : chapter.to_roman();
                }
                Self::Appendix => : "appendix";
                Self::Drm(chapter) => {
                    i : "De Re Metallica";
                    : " chapter ";
                    : chapter.to_roman();
                }
                Self::Journal(volume, issue, credit) => {
                    : "journal volume ";
                    : volume.to_roman();
                    : ", issue ";
                    : issue.to_roman();
                    : ", credited to ";
                    : join(*credit);
                }
            }
        }
    }
}

pub(crate) enum Source {
    Computation {
        url: &'static str,
        num_permutations: u32,
        permutations: Box<dyn Iterator<Item = [Vec<Molecule>; 2]> + Send>,
        js_get_permutation: Cow<'static, str>,
    },
    Critelli {
        url_part: &'static str,
    },
    CritelliComputation {
        url_part: &'static str,
        num_permutations: u32,
        permutations: Box<dyn Iterator<Item = [Vec<Molecule>; 2]> + Send>,
        js_get_permutation: Cow<'static, str>,
    },
    CritelliPrivate {
        url_part: &'static str,
        file_stem: &'static str,
    },
    Official {
        collection: OfficialCollection,
        zlbb_id: &'static str,
    },
    OfficialNonLb {
        collection: OfficialCollection,
    },
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
            Self::Critelli { url_part } | Self::CritelliComputation { url_part, .. } | Self::CritelliPrivate { url_part, .. } => Some(format!("https://events.critelli.technology/{url_part}").parse().unwrap()),
            Self::Official { .. } | Self::OfficialNonLb { .. } | Self::Tutorial => None,
            Self::Computation { url, .. } | Self::Other { url } | Self::Zlbb { url, .. } => Some(url.parse().unwrap()),
        }
    }

    fn computation_data(&self) -> Option<(u32, &str)> {
        match self {
            Self::Computation { num_permutations, js_get_permutation, .. } | Self::CritelliComputation { num_permutations, js_get_permutation, .. } => Some((*num_permutations, js_get_permutation)),
            Self::Critelli { .. } | Self::CritelliPrivate { .. } | Self::Official { .. } | Self::OfficialNonLb { .. } | Self::Other { .. } | Self::Tutorial | Self::Zlbb { .. } => None,
        }
    }

    #[allow(unused)] // may be useful for validation later
    fn computation_permutations(self) -> impl Iterator<Item = [Vec<Molecule>; 2]> {
        match self {
            Self::Computation { permutations, .. } | Self::CritelliComputation { permutations, .. } => permutations,
            Self::Critelli { .. } | Self::CritelliPrivate { .. } | Self::Official { .. } | Self::OfficialNonLb { .. } | Self::Other { .. } | Self::Tutorial | Self::Zlbb { .. } => Box::new(iter::empty()),
        }
    }
}

fn computation<I: IntoIterator<Item = [Vec<Molecule>; 2]>>(url: &'static str, num_permutations: u32, permutations: I, js_get_permutation: impl Into<Cow<'static, str>>) -> Source
where I::IntoIter: Send + 'static {
    Source::Computation {
        permutations: Box::new(permutations.into_iter()),
        js_get_permutation: js_get_permutation.into(),
        url, num_permutations,
    }
}

fn critelli(url_part: &'static str) -> Source {
    Source::Critelli { url_part }
}

fn critelli_computation<I: IntoIterator<Item = [Vec<Molecule>; 2]>>(url_part: &'static str, num_permutations: u32, permutations: I, js_get_permutation: impl Into<Cow<'static, str>>) -> Source
where I::IntoIter: Send + 'static {
    Source::CritelliComputation {
        permutations: Box::new(permutations.into_iter()),
        js_get_permutation: js_get_permutation.into(),
        url_part, num_permutations,
    }
}

fn critelli_private(url_part: &'static str, file_stem: &'static str) -> Source {
    Source::CritelliPrivate { url_part, file_stem }
}

fn official(collection: OfficialCollection, zlbb_id: &'static str) -> Source {
    Source::Official { collection, zlbb_id }
}

#[allow(unused)] // intermittently useful in case the leaderboard takes a while to update
fn official_non_lb(collection: OfficialCollection) -> Source {
    Source::OfficialNonLb { collection }
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
pub(crate) async fn index(config: &State<Config>, http_client: &State<reqwest::Client>, if_none_match: IfNoneMatch<'_>) -> StaticPageResponse {
    static_page(config, http_client, if_none_match, Tab::Puzzles, false, "Puzzles — Opus Magnum Molecule Database", html! {
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
pub(crate) async fn get(config: &State<Config>, http_client: &State<reqwest::Client>, if_none_match: IfNoneMatch<'_>, puzzle: Puzzle) -> Result<StaticPageResponse, Error> {
    Ok(static_page(config, http_client, if_none_match, Tab::Puzzles, true, html! {
        : puzzle;
        : " — Opus Magnum Molecule Database";
    }, html! {
        h1 : puzzle;
        p {
            @match puzzle.source() {
                source @ (Source::Critelli { .. } | Source::CritelliComputation { .. } | Source::CritelliPrivate { .. }) => : external_link(config, http_client, source.url().unwrap().as_str(), "Event page").await?;
                source @ (Source::Computation { .. } | Source::Other { .. } | Source::Zlbb { .. }) => : external_link(config, http_client, source.url().unwrap().as_str(), "Source").await?;
                Source::Official { collection, .. } | Source::OfficialNonLb { collection } => : collection;
                Source::Tutorial => : "tutorial";
            }
        }
        @if let Puzzle::MemoryLane = puzzle {
            p : "Note: The mapping from the variable input to the variable output is defined by each individual solution. This page shows a random encoding, refresh it to generate a new one.";
        }
        h2 : "Reagents";
        div(id = "reagents", class = "row") {
            @for (idx1, (molecule, name, count)) in molecules::molecules()
                .into_iter()
                .flat_map(|(molecule, appearances)| appearances.into_iter().filter_map(move |(iter_puzzle, i, _, name)| (iter_puzzle == puzzle && i > 0).then(|| (molecule.clone(), name, i))))
                .enumerate()
            {
                @for idx2 in 0..count {
                    div {
                        h3 {
                            @if let Some(name) = name {
                                : name;
                            } else {
                                span(class = "muted") : "unnamed";
                            }
                        }
                        a(href = uri!(crate::index(Some(FormMolecule(molecule.clone())), _))) : molecule.draw(&format!("reagent{idx1}_{idx2}"));
                    }
                }
            }
        }
        h2 : "Products";
        div(id = "products", class = "row") {
            @for (idx1, (molecule, name, count)) in molecules::molecules()
                .into_iter()
                .flat_map(|(molecule, appearances)| appearances.into_iter().filter_map(move |(iter_puzzle, _, o, name)| (iter_puzzle == puzzle && o > 0).then(|| (molecule.clone(), name, o))))
                .enumerate()
            {
                @for idx2 in 0..count {
                    div {
                        h3 {
                            @if let Some(name) = name {
                                : name;
                            } else {
                                span(class = "muted") : "unnamed";
                            }
                        }
                        a(href = uri!(crate::index(Some(FormMolecule(molecule.clone())), _))) : molecule.draw(&format!("product{idx1}_{idx2}"));
                    }
                }
            }
        }
        @let source = puzzle.source();
        @if let Some((num_permutations, js_get_permutation)) = source.computation_data() {
            script {
                : RawHtml(format!("
                    const numPermutations = {num_permutations};
                    function getPermutation(idx) {{
                        {js_get_permutation}
                    }}
                "));
                : RawHtml("
                    const reagents = document.getElementById('reagents');
                    const products = document.getElementById('products');
                    let remainingPermutations = [...Array(numPermutations).keys()];
                    function nextPermutation() {
                        if (remainingPermutations.length == 0) {
                            remainingPermutations = [...Array(numPermutations).keys()];
                        }
                        const [permutationIdx] = remainingPermutations.splice(Math.floor(Math.random() * remainingPermutations.length), 1);
                        const permutation = getPermutation(permutationIdx);
                        for (const elt of [...document.getElementsByClassName('computation-molecule')]) {
                            elt.remove();
                        }
                        for (const [idx, reagent] of permutation.reagents.entries()) {
                            const id = `computation-reagent${idx}`;
                            const div = document.createElement('div');
                            div.setAttribute('class', 'computation-molecule');
                            const h3 = document.createElement('h3');
                            const span = document.createElement('span');
                            span.setAttribute('class', 'muted');
                            span.appendChild(document.createTextNode('variable'));
                            h3.appendChild(span);
                            div.appendChild(h3);
                            const canvas = document.createElement('canvas');
                            canvas.setAttribute('class', 'molecule-canvas');
                            canvas.setAttribute('id', id);
                            div.appendChild(canvas); //TODO linkify
                            reagents.appendChild(div);
                            drawProduct(id, ...reagent);
                        }
                        for (const [idx, product] of permutation.products.entries()) {
                            const id = `computation-product${idx}`;
                            const div = document.createElement('div');
                            div.setAttribute('class', 'computation-molecule');
                            const h3 = document.createElement('h3');
                            const span = document.createElement('span');
                            span.setAttribute('class', 'muted');
                            span.appendChild(document.createTextNode('variable'));
                            h3.appendChild(span);
                            div.appendChild(h3);
                            const canvas = document.createElement('canvas');
                            canvas.setAttribute('class', 'molecule-canvas');
                            canvas.setAttribute('id', id);
                            div.appendChild(canvas); //TODO linkify
                            products.appendChild(div);
                            drawProduct(id, ...product);
                        }
                    }
                    nextPermutation();
                    setInterval(nextPermutation, 2000);
                ");
            }
        }
    }, html! {}).await)
}

const CARDINALS: [Atom; 4] = [Atom::Earth, Atom::Air, Atom::Water, Atom::Fire];
const METALS: [Atom; 6] = [Atom::Lead, Atom::Tin, Atom::Iron, Atom::Copper, Atom::Silver, Atom::Gold];

fn stick(string: &[Atom]) -> Molecule {
    Molecule {
        atoms: string.iter().enumerate().map(|(r, kind)| (HexIndex { q: 0, r: r as i32 }, *kind)).collect(),
        bonds: (1..string.len()).map(|end| Bond { start: HexIndex { q: 0, r: end as i32 - 1 }, end: HexIndex { q: 0, r: end as i32 }, ty: BondType::Normal }).collect(),
    }
}

/// Translated from <http://critelli.technology/transmogrification.html>
fn state_from_enumeration_index(mut index: u16) -> Molecule {
    const ATOMS_BY_ENCODING: [Atom; 15] = [
        Atom::Salt,
        Atom::Air,
        Atom::Earth,
        Atom::Fire,
        Atom::Water,
        Atom::Quicksilver,
        Atom::Gold,
        Atom::Silver,
        Atom::Copper,
        Atom::Iron,
        Atom::Tin,
        Atom::Lead,
        Atom::Vitae,
        Atom::Mors,
        Atom::Quintessence,
    ];

    fn triangular(mut index: u16) -> [u16; 2] {
        let mut n = 1;
        loop {
            if index < n {
                return [n, index]
            }
            index -= n;
            n += 1;
        }
    }

    fn tetrahedral(mut index: u16) -> [u16; 3] {
        let mut n = 2;
        loop {
            if index < n * (n - 1) / 2 {
                let [a, b] = triangular(index);
                return [n, a, b]
            }
            index -= n * (n - 1) / 2;
            n += 1;
        }
    }

    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(index)],
            ],
            bonds: collect![],
        }
    }
    index -= 15;
    if index < 120 {
        let [a, b] = triangular(index);
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(a - 1)],
                HexIndex { q: 1, r: 0 } => ATOMS_BY_ENCODING[usize::from(b)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 120;
    if index < 1800 {
        let b = index % 15;
        let [a, c] = triangular(index / 15);
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(a - 1)],
                HexIndex { q: 1, r: 0 } => ATOMS_BY_ENCODING[usize::from(b)],
                HexIndex { q: 2, r: 0 } => ATOMS_BY_ENCODING[usize::from(c)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 1800;
    if index < 3375 {
        let a = index % 15;
        index = index / 15;
        let b = index % 15;
        index = index / 15;
        let c = index % 15;
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(a)],
                HexIndex { q: 1, r: -1 } => ATOMS_BY_ENCODING[usize::from(b)],
                HexIndex { q: 1, r: 0 } => ATOMS_BY_ENCODING[usize::from(c)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 3375;
    if index < 3375 {
        let a = index % 15;
        index = index / 15;
        let b = index % 15;
        index = index / 15;
        let c = index % 15;
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(a)],
                HexIndex { q: 1, r: -1 } => ATOMS_BY_ENCODING[usize::from(b)],
                HexIndex { q: 2, r: -1 } => ATOMS_BY_ENCODING[usize::from(c)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 2, r: -1 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 3375;
    if index < 225 {
        let a = index % 15;
        index = index / 15;
        let b = index % 15;
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(a)],
                HexIndex { q: 1, r: -1 } => ATOMS_BY_ENCODING[usize::from(b)],
                HexIndex { q: 1, r: 0 } => ATOMS_BY_ENCODING[usize::from(a)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 225;
    if index < 910 {
        let flip = index % 2 != 0;
        index = index / 2;
        let [a, b, c] = tetrahedral(index);
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(a)],
                HexIndex { q: 1, r: -1 } => if flip { ATOMS_BY_ENCODING[usize::from(b)] } else { ATOMS_BY_ENCODING[usize::from(c)] },
                HexIndex { q: 1, r: 0 } => if flip { ATOMS_BY_ENCODING[usize::from(c)] } else { ATOMS_BY_ENCODING[usize::from(b)] },
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 910;
    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
                HexIndex { q: 2, r: 0 } => ATOMS_BY_ENCODING[usize::from(index)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 15;
    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => ATOMS_BY_ENCODING[usize::from(index)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 15;
    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(index)],
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        }
    }
    index -= 15;
    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 2, r: -1 } => ATOMS_BY_ENCODING[usize::from(index)],
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 2, r: -1 }, ty: BondType::Normal },
            ],
        }
    }
    index -= 15;
    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => ATOMS_BY_ENCODING[usize::from(index)],
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 2, r: -1 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 2, r: -1 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        }
    }
    index -= 15;
    if index < 15 {
        return Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => ATOMS_BY_ENCODING[usize::from(index)],
                HexIndex { q: 1, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        }
    }
    index -= 15;
    match index {
        0 => Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        },
        1 => Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
                HexIndex { q: 2, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        },
        2 => Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        },
        3 => Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 2, r: -1 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 2, r: -1 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        },
        4 => Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal },
            ],
        },
        5 => Molecule {
            atoms: collect![
                HexIndex { q: 0, r: 0 } => Atom::Fire,
                HexIndex { q: 1, r: -1 } => Atom::Fire,
                HexIndex { q: 1, r: 0 } => Atom::Fire,
            ],
            bonds: collect![
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 0, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 1, r: -1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
                Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Triplex { red: true, black: true, yellow: true } },
            ],
        },
        _ => panic!("number out of range"),
    }
}

fn js_get_permutation_serverside(permutations: Vec<[Vec<Molecule>; 2]>) -> String {
    format!("
        const permutations = [{}];
        return permutations[idx];
    ", permutations.into_iter().map(|[reagents, products]| format!(
        "{{reagents: [{}], products: [{}]}}",
        reagents.into_iter().map(|reagent| format!("[{}]", reagent.draw_params())).join(", "),
        products.into_iter().map(|product| format!("[{}]", product.draw_params())).join(", "),
    )).join(", "))
}

puzzles! {
    AWelcomeToHouseColvan => "A Welcome to House Colvan", zlbb("w2450560971", "https://drive.google.com/drive/folders/1Lk1kj1YERh0yWvgIK89dpd_L7TzLhhTo"),
    AblativeCrystal => "Ablative Crystal", official(Journal(99, 3, &["Sheu, C."]), "P068"),
    AbrasiveParticles => "Abrasive Particles", official(Appendix, "P079"),
    ActivePolymerase => "Active Polymerase", zlbb("w2501728219", "https://reddit.com/r/opus_magnum/comments/fe8l4r/week_6_active_polymerase/"),
    AetherDetector => "Aether Detector", official(Appendix, "P077"),
    AetherReactor => "Aether Reactor", critelli("Week_5_AetherReactor"),
    AirshipFuel => "Airship Fuel", official(Campaign(1), "P008"),
    AlchemicalJewel => "Alchemical Jewel", official(Campaign(4), "P035"),
    AlchemicalSlag => "Alchemical Slag", official(Journal(99, 7, &["Price, A."]), "P099"),
    AlcoholSeparation => "Alcohol Separation", official(Campaign(3), "P024"),
    AmalgamatedGoldRing => "Amalgamated Gold Ring", zlbb("w2501727808", "https://reddit.com/r/opus_magnum/comments/ewj8ml/tournament_week_2_amalgamated_gold_ring/"),
    AmeliasCatalyst => "Amelia's Catalyst", critelli("e9d7e303b465bbcba16fc72b0db96bc1"),
    AnimismusBuffer => "Animismus Buffer", official(Journal(99, 8, &["Price, A."]), "P104"),
    ArmorFilament => "Armor Filament", official(Campaign(2), "P020"),
    ArmorPolish => "Armor Polish", official(Drm(2), "P213"),
    ArqueritePromotion => "Arquerite Promotion", other("https://discord.com/channels/278707932089155584/296373951800541186/857072161097515049"),
    ArtificialOre => "Artificial Ore", zlbb("w2591419339", "https://discord.com/channels/278707932089155584/296373951800541186/879900850661769278"),
    Asbestos => "Asbestos", critelli("OM2025Weeklies6_Asbestos"),
    AssassinsFilament => "Assassin's Filament", official(Journal(99, 7, &["Adam, V."]), "P097"),
    BalancedGold => "Balanced Gold", official(Journal(108, 6, &["Påhlson, M."]), "P282"),
    BalingFiber => "Baling Fiber", official(Journal(108, 7, &["Warden, I."]), "P286"),
    BandageThread => "Bandage Thread", official(Journal(99, 10, &["Woodstalker, H."]), "P248"),
    BeautySalve => "Beauty Salve", official(Journal(99, 10, &["Rakl, D."]), "P246"),
    BerlosDualism => "Berlo's Dualism", critelli("b2567dd6f278003b0048996f5de5b64f"),
    BicrystalTransceiver => "Bicrystal Transceiver", critelli("OM2023_W6_BicrystalTransceiver"),
    BiosteelFilament => "Biosteel Filament", critelli("OM2023_W4_BiosteelFilament"),
    BlackPowder => "Black Powder", critelli("OM2023Weeklies_BlackPowder"),
    BlastCordage => "Blast Cordage", official(Journal(108, 9, &["Darex, B. R. K."]), "P299"),
    BloodStanchingPowder => "Blood-Stanching Powder", official(Journal(99, 5, &["Quinn, T."]), "P087"),
    BlueVitriol => "Blue Vitriol (2024 tournament)", critelli("bb94e99e5b9f4d14791f50e953e6f2bb"),
    BlueVitriolJournal => "Blue Vitriol (journal)", official(Journal(99, 11, &["Critelli, Z."]), "P241"),
    Boozesort => "Boozesort", critelli_computation(
        "OM2025Weeklies8_Boozesort",
        81,
        {
            const BOOZE: [Atom; 3] = [Atom::Fire, Atom::Salt, Atom::Water];
            BOOZE.into_iter().array_combinations_with_reps().map(|mut atoms @ [a, b, c, d]| [
                vec![Molecule { atoms: collect![HexIndex { q: 0, r: 1 } => a, HexIndex { q: 1, r: 1 } => b, HexIndex { q: 1, r: 2 } => c, HexIndex { q: 2, r: 0 } => d], bonds: collect![Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 1, r: 2 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal }] }],
                {
                    atoms.sort_by_key(|atom| BOOZE.into_iter().position(|iter_atom| iter_atom == *atom));
                    vec![stick(&atoms)]
                },
            ])
        },
        "
            const booze = ['fire', 'salt', 'water'];
            const a = booze[Math.floor(idx / 27)];
            const b = booze[Math.floor(idx / 9) % 3];
            const c = booze[Math.floor(idx / 3) % 3];
            const d = booze[idx % 3];
            return {
                reagents: [drawParams([
                    {kind: a, q: 0, r: 1},
                    {kind: b, q: 1, r: 1},
                    {kind: c, q: 1, r: 2},
                    {kind: d, q: 2, r: 0},
                ], [
                    {start: {q: 0, r: 1}, end: {q: 1, r: 1}, red: false, black: false, yellow: false},
                    {start: {q: 1, r: 1}, end: {q: 1, r: 2}, red: false, black: false, yellow: false},
                    {start: {q: 1, r: 1}, end: {q: 2, r: 0}, red: false, black: false, yellow: false},
                ])],
                products: [stick([a, b, c, d].sort((a, b) => booze.indexOf(a) - booze.indexOf(b)))],
            };
        ",
    ),
    BrazingCathode => "Brazing Cathode", critelli("OM2022Weeklies_BrazingCathode"),
    BrazingSilver => "Brazing Silver", official(Journal(108, 12, &["Biggie, B."]), "P310"),
    BreathableFluid => "Breathable Fluid", critelli("OM2024Weeklies_BreathableFluid"),
    BrilliantPigment => "Brilliant Pigment", official(Journal(108, 12, &["Ames, F. L."]), "P311"),
    BulkTransmutation => "Bulk Transmutation", critelli("OM2025week6_Bulk_Transmutation"),
    BuoyantCable => "Buoyant Cable", official(Journal(99, 2, &["Venator, M."]), "P062"),
    BurningSpiritOfSaturn => "Burning Spirit of Saturn", critelli("d3f9ca519aacb2d782a5388098299acd"),
    CalligraphersInk => "Calligrapher's Ink", official(Journal(108, 5, &["Tracy, E."]), "P277"),
    CalmBeforeTheStorm => "Calm Before the Storm", zlbb("w2450512434", "https://drive.google.com/drive/folders/1JVbrvF7dcTKmYN68eGlWhTryXy1TEnc8"),
    CancerMedicine => "Cancer Medicine", critelli("d40f1593c0731c9325dd5c5b9ed3db4b"),
    CanisterShot => "Canister Shot", official(Journal(108, 4, &["Fauss, A."]), "P270"),
    CelestialThread => "Celestial Thread", official(Journal(99, 8, &["Day, A."]), "P101"),
    ChildrensToys => "Children's Toys", critelli("f308e34f12f681c32580ee82d0c96c72"),
    ChromaticAberration => "Chromatic Aberration", critelli("26a4f980a1b475197735802f9cb75836"),
    ClimbingRopeFiber => "Climbing Rope Fiber", official(Campaign(3), "P027"),
    Clusterfgold => "Clusterfgold", critelli("7ce689ab3de9678bfb297994d79f17e1"),
    ColvanBlue => "Colvan Blue", other("https://discord.com/channels/278707932089155584/296373951800541186/851990719530532864"),
    CompoundAnaesthetic => "Compound Anaesthetic", official(Journal(99, 11, &["Azur, I."]), "P244"),
    ConductiveEnamel => "Conductive Enamel", official(Journal(99, 6, &["Henderson, I."]), "P093"),
    ConnectTheDots => "Connect the Dots", zlbb("w3101135731", "https://reddit.com/r/opus_magnum/comments/eohzw1/opus_magnum_tournament_2020/"),
    CoolEarrings => "Cool Earrings", critelli("OM2023_WO_CoolEarrings"),
    CorporateWasteReduction => "Corporate Waste Reduction", critelli("88fed58ce6219be91a046e9a62a004c7"),
    CouragePotion => "Courage Potion", official(Campaign(2), "P021"),
    CranberryGlass => "Cranberry Glass", other("https://discord.com/channels/278707932089155584/296373951800541186/864679286073720832"),
    CreativeAccounting => "Creative Accounting", zlbb("w1698785633", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    CrimsonCrystalBisection => "Crimson Crystal Bisection", other("https://discord.com/channels/278707932089155584/296373951800541186/867218205080158208"),
    Critellium => "Critellium", critelli("e8f14a0982aaacbb3254457e77c23a0b"),
    CrystalCompression => "Crystal Compression", other("https://discord.com/channels/278707932089155584/296373951800541186/862141779825655818"),
    CrystallizedAir => "Crystallized Air", critelli("OM2025week5_Crystallized_Air"),
    CultivationTonic => "Cultivation Tonic", official(Journal(99, 10, &["Nick, P. A."]), "P249"),
    Cuprite => "Cuprite (2022 weeklies)", critelli("OM2022Weeklies_Cuprite"),
    CupriteJournal => "Cuprite (journal)", official(Journal(99, 12, &["Critelli, Z."]), "P253"),
    CuqueritePromotion => "Cuquerite Promotion", critelli("af91511ae10c69e531d347ad8656e594"),
    CuriousLipstick => "Curious Lipstick", official(Campaign(5), "P041"),
    DarkMatterCandidate => "Dark Matter Candidate", critelli("OM2023Weeklies_DarkMatterCandidate"),
    DeepFriedRocketPropellant => "Deep-Fried Rocket Propellant", critelli("OM2024Weeklies_DeepFriedRocketPropellant"),
    DeepFriedUnstableCompound => "Deep-Fried Unstable Compound", critelli("OM2024Weeklies_BSides_DeepFriedUnstableCompound"),
    DehydratedWater => "Dehydrated Water", critelli("OM2022Weeklies_DehydratedWater"),
    DentalAmalgam => "Dental Amalgam (2024 tournament)", critelli("c3dda33075913461bebd7cd7c8759669"),
    DentalAmalgamJournal => "Dental Amalgam (journal)", official(Journal(99, 12, &["Critelli, Z."]), "P252"),
    DestabilizedNature => "Destabilized Nature", critelli("683142751f1988e34bb824ac9302ed20"),
    DoYouRemember => "Do You Remember", zlbb("w1698787731", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    DurableStitching => "Durable Stitching", official(Journal(108, 9, &["Apia, B."]), "P296"),
    DwarvenFireWine => "Dwarven Fire Wine", zlbb("w1698786588", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    DyeHard => "Dye Hard", critelli("OM2023Weeklies_DyeHard"),
    Electrum => "Electrum", official(Journal(108, 10, &["Icai, J."]), "P303"),
    ElectrumSeparation => "Electrum Separation", official(Journal(99, 8, &["Rallus, U."]), "P103"),
    ElementalComparator => "Elemental Comparator", critelli_computation(
        "OM2023_W8w_ElementalComparator",
        16,
        CARDINALS.into_iter().array_combinations_with_reps().map(|[a, b]| [
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => a], bonds: collect![] }, Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => b], bonds: collect![] }],
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => if a == b { Atom::Gold } else { Atom::Salt }], bonds: collect![] }],
        ]),
        "
            const cardinals = ['earth', 'air', 'water', 'fire'];
            const a = cardinals[Math.floor(idx / 4)];
            const b = cardinals[idx % 4];
            return {
                reagents: [stick([a]), stick([b])],
                products: [stick([a == b ? 'gold' : 'salt'])],
            };
        ",
    ),
    ElementalCopper => "Elemental Copper", official(Drm(1), "P202"),
    ElementalJewelSetting => "Elemental Jewel Setting", zlbb("w2450512809", "https://drive.google.com/drive/folders/1P7fsijiuJTI-1LKrpT7IMn7nj2V5PAvC"),
    EmbalmingFluid => "Embalming Fluid", official(Journal(99, 9, &["Day, A."]), "P108"),
    EmergencyAntidote => "Emergency Antidote", zlbb("w2450512232", "https://drive.google.com/drive/folders/1SL0WExUVLu6_xsvZCA9z29PH6RuFBrBd"),
    EndGame => "End Game", critelli("OM2023_W0_EndGame"),
    EndurancePotion => "Endurance Potion", official(Journal(108, 8, &["Wellman, G."]), "P293"),
    EphemeralMatrix => "Ephemeral Matrix", critelli("5a5504a1f72574d23012a6458d1a29b1"),
    EssenceOfCitrus => "Essence of Citrus", official(Journal(108, 1, &["Fontenelle, A."]), "P257"),
    EvilOre => "Evil Ore", zlbb("w1698788220", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    ExMateria => "Ex Materia", official(Drm(3), "P217"),
    ExperimentalCatalyst => "Experimental Catalyst", official(Drm(3), "P220"),
    ExplorersSalve => "Explorer's Salve", official(Journal(99, 2, &["Laj, T."]), "P059"),
    ExplosiveAlloy => "Explosive Alloy", official(Journal(99, 12, &["Via Lactea, V."]), "P251"),
    ExplosiveFingerTrap => "Explosive Finger Trap", other("https://discord.com/channels/278707932089155584/296373951800541186/874833948570689577"),
    ExplosiveLogicUnit => "Explosive Logic Unit", computation(
        "https://drive.google.com/drive/folders/1A_GkcZV1fxMt3vII6qGsD_wX2ULzG8Ca",
        65536,
        all().array_combinations_with_reps().map(|[a, b]| {
            fn bits(n: u8) -> Vec<Atom> {
                n.view_bits::<Msb0>().into_iter().map(|bit| if *bit { Atom::Fire } else { Atom::Salt }).collect()
            }

            [
                vec![stick(&bits(a)), stick(&bits(b))],
                vec![stick(&bits(a.wrapping_sub(b)))],
            ]
        }),
        "
            const a = Math.floor(idx / 256);
            const b = idx % 256;
            function bits(n) {
                const bitsSuffix = n.toString(2);
                return [...'0'.repeat(8 - bitsSuffix.length) + bitsSuffix].map((bit) => bit == '1' ? 'fire' : 'salt');
            }
            return {
                reagents: [stick(bits(a)), stick(bits(b))],
                products: [stick(bits(((a - b) >>> 0) % 256))],
            };
        ",
    ),
    ExplosivePhial => "Explosive Phial", official(Campaign(2), "P017"),
    ExplosiveVictrite => "Explosive Victrite", official(Journal(99, 8, &["Adam, V."]), "P100"),
    ExtractionFromSapa => "Extraction from Sapa", critelli("65f67bd69c877c6b90352dea6c440aa1"),
    EyedropsOfIrritation => "Eyedrops Of Irritation", critelli("OM2023Weeklies_EyedropsOfIrritation"),
    EyedropsOfRevelation => "Eyedrops of Revelation", official(Appendix, "P081"),
    FacePowder => "Face Powder", official(Campaign(1), "P009"),
    FaeroFilament => "Faero Filament", critelli("OM2024Weeklies_FaeroFilament"),
    FerrousWheel => "Ferrous Wheel", zlbb("w2565611826", "https://discord.com/channels/278707932089155584/296373951800541186/869756148788625459"),
    FilmCrystal => "Film Crystal", critelli("Week_4_FilmCrystal"),
    FireworksPowder => "Fireworks Powder", official(Drm(2), "P211"),
    FlakeSalt => "Flake Salt", critelli("Week_-1_FlakeSalt"),
    FragrantPowders => "Fragrant Powders", official(Appendix, "P075"),
    FructifyingWater => "Fructifying Water", official(Journal(108, 7, &["Thinnes, Y."]), "P289"),
    Fulmination => "Fulmination", critelli("Week_6_Fulmination"),
    GalenaSeparation => "Galena Separation", official(Drm(1), "P207"),
    Galvanization => "Galvanization", critelli("58aea7d6d7ba1b4e8b73d2219de0a8cc"),
    GeneralAnaesthetic => "General Anaesthetic", official(Journal(99, 5, &["Wu, B."]), "P086"),
    GildingWax => "Gilding Wax", official(Journal(108, 5, &["Mallard, J."]), "P279"),
    GlitraPaint => "Glitra Paint", official(Journal(108, 5, &["Fauss, A."]), "P275"),
    GlorpsConstruct => "Glorp's Construct", critelli("b721b7ba8e14db667d5ea374eaca9a9e"),
    GoldenThread => "Golden Thread", official(Campaign(4), "P037"),
    GreenVitriol => "Green Vitriol (2021 weeklies)", zlbb("w2539581468", "https://discord.com/channels/278707932089155584/296373951800541186/859612178902286376"),
    GreenVitriolJournal => "Green Vitriol (journal)", official(Journal(99, 11, &["Critelli, Z."]), "P240"),
    GrenadePellet => "Grenade Pellet", official(Journal(108, 1, &["Daas, S."]), "P259"),
    Grindstone => "Grindstone", official(Journal(108, 11, &["Aonum, C."]), "P305"),
    GuipureLace => "Guipure Lace", official(Journal(108, 12, &["Ran, F.", "Thinnes, Y."]), "P312"),
    Gunmetal => "Gunmetal", official(Journal(108, 4, &["Joshua, Y."]), "P272"),
    HabitabilityDetector => "Habitability Detector", critelli_computation(
        "OM2023_W8_HabitabilityDetector",
        262144,
        CARDINALS.into_iter().array_combinations_with_reps().map(|[p1, p2, p3, s1, s2, s3, s4, s5, s6]| [
            vec![stick(&[p1, p2, p3]), stick(&[s1, s2, s3, s4, s5, s6])],
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => if [s1, s2, s3, s4, s5, s6].into_iter().array_windows().any(|window| window == [p1, p2, p3]) { Atom::Gold } else { Atom::Salt }], bonds: collect![] }],
        ]),
        "
            const cardinals = ['earth', 'air', 'water', 'fire'];
            const p1 = cardinals[Math.floor(idx / 4 ** 8)];
            const p2 = cardinals[Math.floor(idx / 4 ** 7) % 4];
            const p3 = cardinals[Math.floor(idx / 4 ** 6) % 4];
            const s1 = cardinals[Math.floor(idx / 4 ** 5) % 4];
            const s2 = cardinals[Math.floor(idx / 4 ** 4) % 4];
            const s3 = cardinals[Math.floor(idx / 4 ** 3) % 4];
            const s4 = cardinals[Math.floor(idx / 4 ** 2) % 4];
            const s5 = cardinals[Math.floor(idx / 4) % 4];
            const s6 = cardinals[idx % 4];
            return {
                reagents: [stick([p1, p2, p3]), stick([s1, s2, s3, s4, s5, s6])],
                products: [stick([(
                    s1 == p1 && s2 == p2 && s3 == p3
                    || s2 == p1 && s3 == p2 && s4 == p3
                    || s3 == p1 && s4 == p2 && s5 == p3
                    || s4 == p1 && s5 == p2 && s6 == p3
                ) ? 'gold' : 'salt'])],
            };
        ",
    ),
    HairProduct => "Hair Product", official(Campaign(2), "P016"),
    HangoverCure => "Hangover Cure", official(Campaign(1), "P013"),
    HealthTonic => "Health Tonic", official(Campaign(1), "P014"),
    HemisphereChange => "Hemisphere Change", critelli("af2bef37158400f2caf15403a1d3c687"),
    HexstabilizedSalt => "Hexstabilized Salt", official(Journal(99, 6, &["Griswold, I."]), "P091b"),
    HexstabilizedTeulingsMors => "Hexstabilized Teuling's Mors", critelli("OM2023Weeklies_HexstabilizedTeulingsMors"),
    HighExplosive => "High Explosive", official(Journal(108, 4, &["Nick, P. A."]), "P271"),
    HighGlossFinish => "High Gloss Finish", zlbb("w2501728349", "https://reddit.com/r/opus_magnum/comments/fhui7x/week_7_high_gloss_finish/"),
    HornSilver => "Horn Silver", zlbb("w2513871683", "https://discord.com/channels/278707932089155584/296373951800541186/849437821918904350"),
    HotIce => "Hot Ice", critelli("OM2022Weeklies_HotIce"),
    HydrophobicWater => "Hydrophobic Water", critelli("om2025week1_Hydrophobic_Water"),
    HydroponicSolution => "Hydroponic Solution", critelli("OM2023_W3_HydroponicSolution"),
    HyperVolatileGas => "Hyper-volatile Gas", official(Journal(99, 9, &["Fauss, A."]), "P106"),
    IcelandicLavaSalt => "Icelandic Lava Salt", critelli("952a099fce7b49281d4b95f0f37dae8e"),
    IgnitionCord => "Ignition Cord", critelli("OM2022Weeklies_IgnitionCord"),
    ImmortalFilament => "Immortal Filament", critelli("483f5c168a293fbed5aaf12990be50cf"),
    ImprovedExplosivePhial => "Improved Explosive Phial", zlbb("w2450508212", "https://drive.google.com/drive/folders/1aRi8dJIu7YPhikm-QXRboJybAr9ZlW0j"),
    InBerlosBasement => "In Berlo's Basement", critelli("66f74ef1aae21439a688b1cc54ca894c"),
    InLocoDispono => "In Loco Dispono", critelli("5b464528478f002ba6866f690bd01f40"),
    InductiveFoil => "Inductive Foil", official(Journal(108, 6, &["Aonum, C."]), "P280"),
    InstantMirrorCoat => "Instant Mirror Coat", critelli("OM2024Weeklies_InstantMirrorCoat"),
    IntumescentLead => "Intumescent Lead", critelli("fc37c3c4183d77bb17bf827ae66c53d7"),
    InvariantMetal => "Invariant Metal", official(Drm(3), "P215"),
    InvigoratingTonic => "Invigorating Tonic", official(Journal(108, 8, &["Silva, J."]), "P291"),
    InvisibleInk => "Invisible Ink", official(Campaign(4), "P032"),
    JewelBox => "Jewel Box", critelli("OM2025Weeklies1_JewelBox"),
    Lambent29 => "Lambent II/IX", official(Journal(99, 1, &["Van Berlo, C."]), "P058"),
    Lambent67 => "Lambent LXVII", critelli_computation(
        "0c61ded553925ac6b6d567386c9982b8",
        72,
        {
            // translated from https://lambentlxvii.pages.dev/
            const CELLS: [[i32; 2]; 36] = [[-3, 0], [-3, 1], [-3, 2], [-3, 3], [-2,-1], [-2, 0], [-2, 1], [-2, 2], [-2, 3], [-1,-2], [-1,-1], [-1, 0], [-1, 1], [-1, 2], [-1, 3], [0,-3], [0,-2], [0,-1], [0, 1], [0, 2], [0, 3], [1,-3], [1,-2], [1,-1], [1, 0], [1, 1], [1, 2], [2,-3], [2,-2], [2,-1], [2, 0], [2, 1], [3,-3], [3,-2], [3,-1], [3, 0]];
            const SOLUTIONS: &[[u8; 36]] = &[[1, 3, 3, 3, 1, 2, 2, 2, 3, 1, 2, 5, 8, 8, 8, 1, 5, 5, 8, 4, 4, 5, 9, 9, 4, 4, 7, 9, 6, 6, 7, 7, 9, 6, 6, 7], [1, 3, 3, 3, 1, 9, 9, 4, 3, 1, 9, 5, 5, 4, 4, 1, 6, 9, 5, 5, 4, 6, 6, 2, 2, 2, 7, 6, 2, 8, 7, 7, 8, 8, 8, 7], [1, 3, 3, 3, 1, 9, 9, 4, 3, 1, 9, 5, 5, 4, 4, 1, 6, 9, 5, 5, 4, 6, 6, 7, 8, 8, 8, 6, 7, 7, 8, 2, 7, 2, 2, 2], [1, 3, 3, 3, 1, 9, 9, 4, 3, 1, 9, 5, 5, 4, 4, 1, 8, 9, 5, 5, 4, 2, 8, 8, 7, 7, 7, 2, 8, 6, 6, 7, 2, 2, 6, 6], [1, 3, 3, 3, 1, 4, 2, 2, 3, 1, 6, 4, 4, 2, 9, 1, 6, 6, 4, 2, 9, 8, 6, 5, 5, 9, 9, 8, 8, 7, 5, 5, 8, 7, 7, 7], [1, 3, 3, 3, 1, 4, 5, 5, 3, 1, 7, 4, 4, 5, 5, 1, 7, 7, 4, 8, 8, 7, 9, 2, 2, 2, 8, 9, 2, 6, 6, 8, 9, 9, 6, 6], [1, 3, 3, 3, 1, 4, 5, 5, 3, 1, 7, 4, 4, 5, 5, 1, 7, 7, 4, 6, 6, 7, 9, 9, 8, 6, 6, 9, 8, 8, 8, 2, 9, 2, 2, 2], [1, 3, 3, 3, 1, 5, 2, 4, 3, 1, 8, 5, 2, 4, 4, 1, 8, 5, 2, 2, 4, 8, 8, 5, 9, 9, 7, 6, 6, 9, 7, 7, 6, 6, 9, 7], [1, 3, 3, 3, 1, 6, 6, 8, 3, 1, 2, 6, 6, 8, 8, 1, 2, 4, 9, 8, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 3, 3, 3, 1, 8, 8, 8, 3, 1, 2, 8, 6, 6, 9, 1, 2, 4, 6, 6, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 3, 3, 3, 1, 8, 8, 8, 3, 1, 7, 8, 5, 9, 9, 1, 7, 7, 5, 4, 9, 7, 2, 2, 5, 4, 9, 6, 6, 2, 5, 4, 6, 6, 2, 4], [1, 8, 8, 7, 1, 4, 8, 9, 7, 1, 4, 8, 9, 7, 7, 1, 3, 4, 9, 9, 5, 3, 4, 6, 6, 5, 5, 3, 6, 6, 5, 2, 3, 2, 2, 2], [1, 9, 9, 8, 1, 9, 8, 8, 8, 1, 2, 9, 3, 3, 3, 1, 2, 4, 6, 6, 3, 2, 4, 5, 5, 6, 6, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 7, 2, 2, 1, 4, 7, 5, 2, 1, 4, 7, 7, 5, 2, 1, 3, 4, 5, 8, 8, 3, 4, 9, 9, 5, 8, 3, 9, 6, 6, 8, 3, 9, 6, 6], [1, 8, 7, 7, 1, 8, 2, 7, 3, 1, 8, 8, 2, 7, 3, 1, 5, 5, 2, 2, 3, 6, 6, 5, 5, 9, 3, 6, 6, 4, 4, 9, 4, 4, 9, 9], [1, 8, 6, 6, 1, 8, 4, 6, 6, 1, 8, 8, 4, 4, 2, 1, 5, 5, 9, 4, 2, 7, 7, 5, 5, 9, 2, 7, 3, 9, 9, 2, 7, 3, 3, 3], [1, 8, 6, 7, 1, 8, 6, 6, 7, 1, 8, 8, 6, 7, 7, 1, 3, 9, 4, 4, 5, 3, 9, 4, 4, 5, 5, 3, 9, 9, 5, 2, 3, 2, 2, 2], [1, 8, 9, 9, 1, 8, 9, 4, 9, 1, 8, 8, 3, 4, 4, 1, 5, 5, 3, 2, 4, 7, 7, 5, 5, 3, 2, 7, 6, 6, 3, 2, 7, 6, 6, 2], [1, 8, 9, 9, 1, 8, 9, 4, 9, 1, 8, 8, 5, 4, 4, 1, 2, 2, 5, 6, 4, 7, 7, 2, 5, 6, 6, 7, 3, 2, 5, 6, 7, 3, 3, 3], [1, 6, 6, 5, 1, 4, 6, 6, 5, 1, 7, 4, 4, 5, 2, 1, 9, 7, 4, 5, 2, 9, 7, 7, 3, 8, 2, 9, 9, 3, 8, 2, 3, 3, 8, 8], [1, 6, 6, 5, 1, 4, 6, 6, 5, 1, 7, 4, 4, 5, 9, 1, 3, 7, 4, 5, 9, 3, 7, 7, 8, 9, 9, 3, 8, 8, 8, 2, 3, 2, 2, 2], [1, 6, 6, 5, 1, 4, 6, 6, 5, 1, 2, 4, 4, 5, 9, 1, 8, 2, 4, 5, 9, 8, 8, 2, 2, 9, 9, 3, 8, 7, 7, 7, 3, 3, 3, 7], [1, 7, 7, 7, 1, 9, 9, 7, 3, 1, 9, 5, 9, 2, 3, 1, 5, 5, 2, 4, 3, 5, 6, 6, 2, 4, 3, 6, 6, 8, 2, 4, 8, 8, 8, 4], [1, 7, 7, 7, 1, 2, 2, 7, 8, 1, 3, 3, 2, 8, 8, 1, 3, 4, 2, 6, 8, 3, 9, 4, 4, 6, 6, 9, 5, 5, 4, 6, 9, 9, 5, 5], [1, 7, 7, 7, 1, 9, 9, 7, 8, 1, 9, 5, 5, 8, 8, 1, 6, 9, 5, 5, 8, 6, 6, 3, 3, 4, 4, 6, 3, 4, 4, 2, 3, 2, 2, 2], [1, 7, 7, 7, 1, 9, 9, 7, 8, 1, 9, 5, 5, 8, 8, 1, 6, 9, 5, 5, 8, 6, 6, 2, 2, 2, 3, 6, 2, 4, 4, 3, 4, 4, 3, 3], [1, 7, 7, 7, 1, 9, 9, 7, 4, 1, 9, 5, 9, 4, 2, 1, 5, 5, 3, 4, 2, 5, 6, 6, 3, 4, 2, 6, 6, 8, 3, 2, 8, 8, 8, 3], [1, 7, 7, 7, 1, 6, 6, 7, 8, 1, 6, 6, 8, 8, 8, 1, 3, 9, 4, 4, 5, 3, 9, 4, 4, 5, 5, 3, 9, 9, 5, 2, 3, 2, 2, 2], [1, 2, 2, 2, 1, 2, 3, 3, 3, 1, 8, 4, 5, 5, 3, 1, 8, 4, 7, 5, 5, 8, 8, 4, 7, 7, 7, 9, 4, 9, 6, 6, 9, 9, 6, 6], [1, 2, 2, 2, 1, 2, 6, 6, 7, 1, 3, 3, 6, 6, 7, 1, 3, 4, 8, 7, 7, 3, 9, 4, 4, 8, 8, 9, 5, 5, 4, 8, 9, 9, 5, 5], [1, 2, 2, 2, 1, 2, 7, 7, 8, 1, 3, 3, 7, 8, 8, 1, 3, 4, 7, 6, 8, 3, 9, 4, 4, 6, 6, 9, 5, 5, 4, 6, 9, 9, 5, 5], [1, 2, 2, 2, 1, 2, 8, 8, 8, 1, 3, 3, 8, 6, 6, 1, 3, 4, 6, 6, 7, 3, 9, 4, 4, 7, 7, 9, 5, 5, 4, 7, 9, 9, 5, 5], [1, 2, 2, 2, 1, 2, 4, 4, 7, 1, 4, 4, 8, 8, 7, 1, 9, 9, 8, 7, 7, 9, 6, 9, 8, 5, 3, 6, 6, 5, 5, 3, 6, 5, 3, 3], [1, 2, 2, 2, 1, 2, 4, 4, 3, 1, 4, 4, 6, 6, 3, 1, 5, 5, 6, 6, 3, 7, 7, 5, 5, 9, 3, 7, 8, 8, 8, 9, 7, 8, 9, 9], [1, 2, 2, 2, 1, 2, 5, 9, 9, 1, 5, 5, 9, 8, 9, 1, 5, 4, 3, 8, 8, 7, 7, 4, 4, 3, 8, 7, 6, 6, 4, 3, 7, 6, 6, 3], [1, 2, 2, 2, 1, 2, 5, 6, 3, 1, 5, 5, 6, 6, 3, 1, 5, 8, 6, 4, 3, 7, 7, 8, 8, 4, 3, 7, 9, 8, 9, 4, 7, 9, 9, 4], [1, 2, 2, 2, 1, 2, 3, 3, 3, 1, 6, 6, 5, 5, 3, 1, 6, 6, 4, 5, 5, 7, 7, 7, 4, 8, 8, 9, 7, 9, 4, 8, 9, 9, 4, 8], [1, 2, 2, 2, 1, 2, 7, 8, 3, 1, 7, 7, 8, 8, 3, 1, 6, 7, 5, 8, 3, 6, 6, 5, 5, 9, 3, 6, 5, 4, 4, 9, 4, 4, 9, 9], [1, 2, 2, 2, 1, 2, 9, 9, 3, 1, 8, 9, 5, 9, 3, 1, 8, 8, 5, 4, 3, 7, 7, 8, 5, 4, 3, 7, 6, 6, 5, 4, 7, 6, 6, 4], [1, 9, 6, 6, 1, 9, 6, 6, 3, 1, 2, 9, 9, 5, 3, 1, 2, 4, 7, 5, 3, 2, 4, 7, 7, 5, 3, 2, 4, 7, 8, 5, 4, 8, 8, 8], [1, 6, 6, 5, 1, 6, 6, 3, 5, 1, 8, 4, 3, 5, 2, 1, 8, 4, 3, 5, 2, 8, 8, 4, 7, 3, 2, 9, 4, 9, 7, 2, 9, 9, 7, 7], [1, 6, 6, 5, 1, 6, 6, 4, 5, 1, 3, 3, 4, 5, 2, 1, 3, 7, 4, 5, 2, 3, 9, 7, 4, 8, 2, 9, 7, 7, 8, 2, 9, 9, 8, 8], [1, 6, 6, 3, 1, 6, 6, 3, 8, 1, 2, 3, 3, 8, 9, 1, 2, 4, 8, 8, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 6, 6, 3, 1, 6, 6, 8, 3, 1, 2, 8, 8, 8, 3, 1, 2, 4, 9, 3, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 6, 6, 8, 1, 6, 6, 8, 8, 1, 2, 3, 3, 3, 8, 1, 2, 4, 9, 3, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 6, 6, 4, 1, 6, 6, 4, 3, 1, 2, 2, 2, 4, 3, 1, 2, 5, 4, 8, 3, 7, 7, 5, 9, 8, 3, 7, 5, 9, 8, 8, 7, 5, 9, 9], [1, 6, 6, 2, 1, 6, 6, 2, 3, 1, 8, 4, 2, 5, 3, 1, 8, 4, 2, 5, 3, 8, 8, 4, 7, 5, 3, 9, 4, 9, 7, 5, 9, 9, 7, 7], [1, 4, 9, 9, 1, 4, 9, 2, 2, 1, 8, 4, 9, 3, 2, 1, 8, 4, 5, 3, 2, 8, 8, 7, 7, 5, 3, 6, 6, 7, 5, 3, 6, 6, 7, 5], [1, 4, 9, 9, 1, 4, 9, 6, 6, 1, 8, 4, 9, 6, 6, 1, 8, 4, 2, 2, 2, 8, 8, 7, 2, 5, 3, 7, 7, 5, 5, 3, 7, 5, 3, 3], [1, 4, 9, 9, 1, 4, 9, 6, 6, 1, 8, 4, 9, 6, 6, 1, 8, 4, 3, 3, 5, 8, 8, 7, 3, 5, 5, 7, 7, 3, 5, 2, 7, 2, 2, 2], [1, 4, 9, 9, 1, 4, 9, 7, 7, 1, 8, 4, 9, 7, 2, 1, 8, 4, 5, 7, 2, 8, 8, 6, 6, 5, 2, 3, 6, 6, 5, 2, 3, 3, 3, 5], [1, 2, 9, 9, 1, 2, 6, 6, 9, 1, 2, 6, 6, 9, 5, 1, 8, 2, 5, 5, 7, 8, 8, 4, 5, 7, 7, 3, 8, 4, 4, 7, 3, 3, 3, 4], [1, 7, 6, 6, 1, 7, 7, 6, 6, 1, 7, 4, 4, 9, 9, 1, 4, 4, 9, 8, 8, 5, 5, 2, 2, 9, 8, 3, 5, 5, 2, 8, 3, 3, 3, 2], [1, 7, 6, 6, 1, 7, 7, 6, 6, 1, 7, 4, 4, 9, 9, 1, 4, 4, 9, 2, 9, 8, 8, 8, 5, 3, 2, 8, 5, 5, 3, 2, 5, 3, 3, 2], [1, 7, 9, 9, 1, 7, 7, 5, 9, 1, 7, 5, 5, 9, 2, 1, 8, 5, 6, 6, 2, 8, 8, 4, 6, 6, 2, 3, 8, 4, 4, 2, 3, 3, 3, 4], [1, 8, 8, 8, 1, 9, 8, 9, 3, 1, 2, 9, 9, 6, 3, 1, 2, 4, 6, 6, 3, 2, 4, 5, 5, 6, 3, 2, 4, 7, 5, 5, 4, 7, 7, 7], [5, 5, 2, 2, 9, 9, 5, 5, 2, 9, 1, 1, 1, 1, 2, 4, 9, 8, 7, 7, 7, 4, 4, 8, 8, 6, 7, 3, 4, 8, 6, 6, 3, 3, 3, 6]];

            const BONDS: &[[i32; 4]] = &[[-3, 0, -3, 1], [-3, 0, -2, -1], [-3, 0, -2, 0], [-3, 1, -3, 2], [-3, 1, -2, 0], [-3, 1, -2, 1], [-3, 2, -3, 3], [-3, 2, -2, 1], [-3, 2, -2, 2], [-3, 3, -2, 2], [-3, 3, -2, 3], [-2, -1, -2, 0], [-2, -1, -1, -2], [-2, -1, -1, -1], [-2, 0, -2, 1], [-2, 0, -1, -1], [-2, 0, -1, 0], [-2, 1, -2, 2], [-2, 1, -1, 0], [-2, 1, -1, 1], [-2, 2, -2, 3], [-2, 2, -1, 1], [-2, 2, -1, 2], [-2, 3, -1, 2], [-2, 3, -1, 3], [-1, -2, -1, -1], [-1, -2, 0, -3], [-1, -2, 0, -2], [-1, -1, -1, 0], [-1, -1, 0, -2], [-1, -1, 0, -1], [-1, 0, -1, 1], [-1, 0, 0, -1], [-1, 0, 0, 0], [-1, 1, -1, 2], [-1, 1, 0, 0], [-1, 1, 0, 1], [-1, 2, -1, 3], [-1, 2, 0, 1], [-1, 2, 0, 2], [-1, 3, 0, 2], [-1, 3, 0, 3], [0, -3, 0, -2], [0, -3, 1, -3], [0, -2, 0, -1], [0, -2, 1, -3], [0, -2, 1, -2], [0, -1, 0, 0], [0, -1, 1, -2], [0, -1, 1, -1], [0, 0, 0, 1], [0, 0, 1, -1], [0, 0, 1, 0], [0, 1, 0, 2], [0, 1, 1, 0], [0, 1, 1, 1], [0, 2, 0, 3], [0, 2, 1, 1], [0, 2, 1, 2], [0, 3, 1, 2], [1, -3, 1, -2], [1, -3, 2, -3], [1, -2, 1, -1], [1, -2, 2, -3], [1, -2, 2, -2], [1, -1, 1, 0], [1, -1, 2, -2], [1, -1, 2, -1], [1, 0, 1, 1], [1, 0, 2, -1], [1, 0, 2, 0], [1, 1, 1, 2], [1, 1, 2, 0], [1, 1, 2, 1], [1, 2, 2, 1], [2, -3, 2, -2], [2, -3, 3, -3], [2, -2, 2, -1], [2, -2, 3, -3], [2, -2, 3, -2], [2, -1, 2, 0], [2, -1, 3, -2], [2, -1, 3, -1], [2, 0, 2, 1], [2, 0, 3, -1], [2, 0, 3, 0], [2, 1, 3, 0], [3, -3, 3, -2], [3, -2, 3, -1], [3, -1, 3, 0]];
            SOLUTIONS.iter().copied().map(|solution| {
                let solution = CELLS.into_iter().zip_eq(solution).collect::<HashMap<_, _>>();
                Molecule {
                    atoms: iter::once((HexIndex { q: 0, r: 0 }, Atom::Gold)).chain(CELLS.into_iter().map(|[q, r]| (HexIndex { q, r }, Atom::Fire))).collect(),
                    bonds: BONDS.iter().copied().map(|[sq, sr, eq, er]| Bond { start: HexIndex { q: sq, r: sr }, end: HexIndex { q: eq, r: er }, ty: if solution.get(&[sq, sr]).is_some_and(|s| solution.get(&[eq, er]).is_some_and(|e| s == e)) { BondType::Triplex { red: true, black: true, yellow: true } } else { BondType::Normal } }).collect(),
                }
            }).flat_map(|molecule| [molecule.mirrored(), molecule]).map(|molecule| [vec![], vec![molecule]])
        },
        "
            const cells = [[-3, 0], [-3, 1], [-3, 2], [-3, 3], [-2,-1], [-2, 0], [-2, 1], [-2, 2], [-2, 3], [-1,-2], [-1,-1], [-1, 0], [-1, 1], [-1, 2], [-1, 3], [0,-3], [0,-2], [0,-1], [0, 1], [0, 2], [0, 3], [1,-3], [1,-2], [1,-1], [1, 0], [1, 1], [1, 2], [2,-3], [2,-2], [2,-1], [2, 0], [2, 1], [3,-3], [3,-2], [3,-1], [3, 0]];
            const solutions = [[1, 3, 3, 3, 1, 2, 2, 2, 3, 1, 2, 5, 8, 8, 8, 1, 5, 5, 8, 4, 4, 5, 9, 9, 4, 4, 7, 9, 6, 6, 7, 7, 9, 6, 6, 7], [1, 3, 3, 3, 1, 9, 9, 4, 3, 1, 9, 5, 5, 4, 4, 1, 6, 9, 5, 5, 4, 6, 6, 2, 2, 2, 7, 6, 2, 8, 7, 7, 8, 8, 8, 7], [1, 3, 3, 3, 1, 9, 9, 4, 3, 1, 9, 5, 5, 4, 4, 1, 6, 9, 5, 5, 4, 6, 6, 7, 8, 8, 8, 6, 7, 7, 8, 2, 7, 2, 2, 2], [1, 3, 3, 3, 1, 9, 9, 4, 3, 1, 9, 5, 5, 4, 4, 1, 8, 9, 5, 5, 4, 2, 8, 8, 7, 7, 7, 2, 8, 6, 6, 7, 2, 2, 6, 6], [1, 3, 3, 3, 1, 4, 2, 2, 3, 1, 6, 4, 4, 2, 9, 1, 6, 6, 4, 2, 9, 8, 6, 5, 5, 9, 9, 8, 8, 7, 5, 5, 8, 7, 7, 7], [1, 3, 3, 3, 1, 4, 5, 5, 3, 1, 7, 4, 4, 5, 5, 1, 7, 7, 4, 8, 8, 7, 9, 2, 2, 2, 8, 9, 2, 6, 6, 8, 9, 9, 6, 6], [1, 3, 3, 3, 1, 4, 5, 5, 3, 1, 7, 4, 4, 5, 5, 1, 7, 7, 4, 6, 6, 7, 9, 9, 8, 6, 6, 9, 8, 8, 8, 2, 9, 2, 2, 2], [1, 3, 3, 3, 1, 5, 2, 4, 3, 1, 8, 5, 2, 4, 4, 1, 8, 5, 2, 2, 4, 8, 8, 5, 9, 9, 7, 6, 6, 9, 7, 7, 6, 6, 9, 7], [1, 3, 3, 3, 1, 6, 6, 8, 3, 1, 2, 6, 6, 8, 8, 1, 2, 4, 9, 8, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 3, 3, 3, 1, 8, 8, 8, 3, 1, 2, 8, 6, 6, 9, 1, 2, 4, 6, 6, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 3, 3, 3, 1, 8, 8, 8, 3, 1, 7, 8, 5, 9, 9, 1, 7, 7, 5, 4, 9, 7, 2, 2, 5, 4, 9, 6, 6, 2, 5, 4, 6, 6, 2, 4], [1, 8, 8, 7, 1, 4, 8, 9, 7, 1, 4, 8, 9, 7, 7, 1, 3, 4, 9, 9, 5, 3, 4, 6, 6, 5, 5, 3, 6, 6, 5, 2, 3, 2, 2, 2], [1, 9, 9, 8, 1, 9, 8, 8, 8, 1, 2, 9, 3, 3, 3, 1, 2, 4, 6, 6, 3, 2, 4, 5, 5, 6, 6, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 7, 2, 2, 1, 4, 7, 5, 2, 1, 4, 7, 7, 5, 2, 1, 3, 4, 5, 8, 8, 3, 4, 9, 9, 5, 8, 3, 9, 6, 6, 8, 3, 9, 6, 6], [1, 8, 7, 7, 1, 8, 2, 7, 3, 1, 8, 8, 2, 7, 3, 1, 5, 5, 2, 2, 3, 6, 6, 5, 5, 9, 3, 6, 6, 4, 4, 9, 4, 4, 9, 9], [1, 8, 6, 6, 1, 8, 4, 6, 6, 1, 8, 8, 4, 4, 2, 1, 5, 5, 9, 4, 2, 7, 7, 5, 5, 9, 2, 7, 3, 9, 9, 2, 7, 3, 3, 3], [1, 8, 6, 7, 1, 8, 6, 6, 7, 1, 8, 8, 6, 7, 7, 1, 3, 9, 4, 4, 5, 3, 9, 4, 4, 5, 5, 3, 9, 9, 5, 2, 3, 2, 2, 2], [1, 8, 9, 9, 1, 8, 9, 4, 9, 1, 8, 8, 3, 4, 4, 1, 5, 5, 3, 2, 4, 7, 7, 5, 5, 3, 2, 7, 6, 6, 3, 2, 7, 6, 6, 2], [1, 8, 9, 9, 1, 8, 9, 4, 9, 1, 8, 8, 5, 4, 4, 1, 2, 2, 5, 6, 4, 7, 7, 2, 5, 6, 6, 7, 3, 2, 5, 6, 7, 3, 3, 3], [1, 6, 6, 5, 1, 4, 6, 6, 5, 1, 7, 4, 4, 5, 2, 1, 9, 7, 4, 5, 2, 9, 7, 7, 3, 8, 2, 9, 9, 3, 8, 2, 3, 3, 8, 8], [1, 6, 6, 5, 1, 4, 6, 6, 5, 1, 7, 4, 4, 5, 9, 1, 3, 7, 4, 5, 9, 3, 7, 7, 8, 9, 9, 3, 8, 8, 8, 2, 3, 2, 2, 2], [1, 6, 6, 5, 1, 4, 6, 6, 5, 1, 2, 4, 4, 5, 9, 1, 8, 2, 4, 5, 9, 8, 8, 2, 2, 9, 9, 3, 8, 7, 7, 7, 3, 3, 3, 7], [1, 7, 7, 7, 1, 9, 9, 7, 3, 1, 9, 5, 9, 2, 3, 1, 5, 5, 2, 4, 3, 5, 6, 6, 2, 4, 3, 6, 6, 8, 2, 4, 8, 8, 8, 4], [1, 7, 7, 7, 1, 2, 2, 7, 8, 1, 3, 3, 2, 8, 8, 1, 3, 4, 2, 6, 8, 3, 9, 4, 4, 6, 6, 9, 5, 5, 4, 6, 9, 9, 5, 5], [1, 7, 7, 7, 1, 9, 9, 7, 8, 1, 9, 5, 5, 8, 8, 1, 6, 9, 5, 5, 8, 6, 6, 3, 3, 4, 4, 6, 3, 4, 4, 2, 3, 2, 2, 2], [1, 7, 7, 7, 1, 9, 9, 7, 8, 1, 9, 5, 5, 8, 8, 1, 6, 9, 5, 5, 8, 6, 6, 2, 2, 2, 3, 6, 2, 4, 4, 3, 4, 4, 3, 3], [1, 7, 7, 7, 1, 9, 9, 7, 4, 1, 9, 5, 9, 4, 2, 1, 5, 5, 3, 4, 2, 5, 6, 6, 3, 4, 2, 6, 6, 8, 3, 2, 8, 8, 8, 3], [1, 7, 7, 7, 1, 6, 6, 7, 8, 1, 6, 6, 8, 8, 8, 1, 3, 9, 4, 4, 5, 3, 9, 4, 4, 5, 5, 3, 9, 9, 5, 2, 3, 2, 2, 2], [1, 2, 2, 2, 1, 2, 3, 3, 3, 1, 8, 4, 5, 5, 3, 1, 8, 4, 7, 5, 5, 8, 8, 4, 7, 7, 7, 9, 4, 9, 6, 6, 9, 9, 6, 6], [1, 2, 2, 2, 1, 2, 6, 6, 7, 1, 3, 3, 6, 6, 7, 1, 3, 4, 8, 7, 7, 3, 9, 4, 4, 8, 8, 9, 5, 5, 4, 8, 9, 9, 5, 5], [1, 2, 2, 2, 1, 2, 7, 7, 8, 1, 3, 3, 7, 8, 8, 1, 3, 4, 7, 6, 8, 3, 9, 4, 4, 6, 6, 9, 5, 5, 4, 6, 9, 9, 5, 5], [1, 2, 2, 2, 1, 2, 8, 8, 8, 1, 3, 3, 8, 6, 6, 1, 3, 4, 6, 6, 7, 3, 9, 4, 4, 7, 7, 9, 5, 5, 4, 7, 9, 9, 5, 5], [1, 2, 2, 2, 1, 2, 4, 4, 7, 1, 4, 4, 8, 8, 7, 1, 9, 9, 8, 7, 7, 9, 6, 9, 8, 5, 3, 6, 6, 5, 5, 3, 6, 5, 3, 3], [1, 2, 2, 2, 1, 2, 4, 4, 3, 1, 4, 4, 6, 6, 3, 1, 5, 5, 6, 6, 3, 7, 7, 5, 5, 9, 3, 7, 8, 8, 8, 9, 7, 8, 9, 9], [1, 2, 2, 2, 1, 2, 5, 9, 9, 1, 5, 5, 9, 8, 9, 1, 5, 4, 3, 8, 8, 7, 7, 4, 4, 3, 8, 7, 6, 6, 4, 3, 7, 6, 6, 3], [1, 2, 2, 2, 1, 2, 5, 6, 3, 1, 5, 5, 6, 6, 3, 1, 5, 8, 6, 4, 3, 7, 7, 8, 8, 4, 3, 7, 9, 8, 9, 4, 7, 9, 9, 4], [1, 2, 2, 2, 1, 2, 3, 3, 3, 1, 6, 6, 5, 5, 3, 1, 6, 6, 4, 5, 5, 7, 7, 7, 4, 8, 8, 9, 7, 9, 4, 8, 9, 9, 4, 8], [1, 2, 2, 2, 1, 2, 7, 8, 3, 1, 7, 7, 8, 8, 3, 1, 6, 7, 5, 8, 3, 6, 6, 5, 5, 9, 3, 6, 5, 4, 4, 9, 4, 4, 9, 9], [1, 2, 2, 2, 1, 2, 9, 9, 3, 1, 8, 9, 5, 9, 3, 1, 8, 8, 5, 4, 3, 7, 7, 8, 5, 4, 3, 7, 6, 6, 5, 4, 7, 6, 6, 4], [1, 9, 6, 6, 1, 9, 6, 6, 3, 1, 2, 9, 9, 5, 3, 1, 2, 4, 7, 5, 3, 2, 4, 7, 7, 5, 3, 2, 4, 7, 8, 5, 4, 8, 8, 8], [1, 6, 6, 5, 1, 6, 6, 3, 5, 1, 8, 4, 3, 5, 2, 1, 8, 4, 3, 5, 2, 8, 8, 4, 7, 3, 2, 9, 4, 9, 7, 2, 9, 9, 7, 7], [1, 6, 6, 5, 1, 6, 6, 4, 5, 1, 3, 3, 4, 5, 2, 1, 3, 7, 4, 5, 2, 3, 9, 7, 4, 8, 2, 9, 7, 7, 8, 2, 9, 9, 8, 8], [1, 6, 6, 3, 1, 6, 6, 3, 8, 1, 2, 3, 3, 8, 9, 1, 2, 4, 8, 8, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 6, 6, 3, 1, 6, 6, 8, 3, 1, 2, 8, 8, 8, 3, 1, 2, 4, 9, 3, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 6, 6, 8, 1, 6, 6, 8, 8, 1, 2, 3, 3, 3, 8, 1, 2, 4, 9, 3, 9, 2, 4, 5, 5, 9, 9, 2, 4, 7, 5, 5, 4, 7, 7, 7], [1, 6, 6, 4, 1, 6, 6, 4, 3, 1, 2, 2, 2, 4, 3, 1, 2, 5, 4, 8, 3, 7, 7, 5, 9, 8, 3, 7, 5, 9, 8, 8, 7, 5, 9, 9], [1, 6, 6, 2, 1, 6, 6, 2, 3, 1, 8, 4, 2, 5, 3, 1, 8, 4, 2, 5, 3, 8, 8, 4, 7, 5, 3, 9, 4, 9, 7, 5, 9, 9, 7, 7], [1, 4, 9, 9, 1, 4, 9, 2, 2, 1, 8, 4, 9, 3, 2, 1, 8, 4, 5, 3, 2, 8, 8, 7, 7, 5, 3, 6, 6, 7, 5, 3, 6, 6, 7, 5], [1, 4, 9, 9, 1, 4, 9, 6, 6, 1, 8, 4, 9, 6, 6, 1, 8, 4, 2, 2, 2, 8, 8, 7, 2, 5, 3, 7, 7, 5, 5, 3, 7, 5, 3, 3], [1, 4, 9, 9, 1, 4, 9, 6, 6, 1, 8, 4, 9, 6, 6, 1, 8, 4, 3, 3, 5, 8, 8, 7, 3, 5, 5, 7, 7, 3, 5, 2, 7, 2, 2, 2], [1, 4, 9, 9, 1, 4, 9, 7, 7, 1, 8, 4, 9, 7, 2, 1, 8, 4, 5, 7, 2, 8, 8, 6, 6, 5, 2, 3, 6, 6, 5, 2, 3, 3, 3, 5], [1, 2, 9, 9, 1, 2, 6, 6, 9, 1, 2, 6, 6, 9, 5, 1, 8, 2, 5, 5, 7, 8, 8, 4, 5, 7, 7, 3, 8, 4, 4, 7, 3, 3, 3, 4], [1, 7, 6, 6, 1, 7, 7, 6, 6, 1, 7, 4, 4, 9, 9, 1, 4, 4, 9, 8, 8, 5, 5, 2, 2, 9, 8, 3, 5, 5, 2, 8, 3, 3, 3, 2], [1, 7, 6, 6, 1, 7, 7, 6, 6, 1, 7, 4, 4, 9, 9, 1, 4, 4, 9, 2, 9, 8, 8, 8, 5, 3, 2, 8, 5, 5, 3, 2, 5, 3, 3, 2], [1, 7, 9, 9, 1, 7, 7, 5, 9, 1, 7, 5, 5, 9, 2, 1, 8, 5, 6, 6, 2, 8, 8, 4, 6, 6, 2, 3, 8, 4, 4, 2, 3, 3, 3, 4], [1, 8, 8, 8, 1, 9, 8, 9, 3, 1, 2, 9, 9, 6, 3, 1, 2, 4, 6, 6, 3, 2, 4, 5, 5, 6, 3, 2, 4, 7, 5, 5, 4, 7, 7, 7], [5, 5, 2, 2, 9, 9, 5, 5, 2, 9, 1, 1, 1, 1, 2, 4, 9, 8, 7, 7, 7, 4, 4, 8, 8, 6, 7, 3, 4, 8, 6, 6, 3, 3, 3, 6]];
            const bonds = [[-3, 0, -3, 1], [-3, 0, -2, -1], [-3, 0, -2, 0], [-3, 1, -3, 2], [-3, 1, -2, 0], [-3, 1, -2, 1], [-3, 2, -3, 3], [-3, 2, -2, 1], [-3, 2, -2, 2], [-3, 3, -2, 2], [-3, 3, -2, 3], [-2, -1, -2, 0], [-2, -1, -1, -2], [-2, -1, -1, -1], [-2, 0, -2, 1], [-2, 0, -1, -1], [-2, 0, -1, 0], [-2, 1, -2, 2], [-2, 1, -1, 0], [-2, 1, -1, 1], [-2, 2, -2, 3], [-2, 2, -1, 1], [-2, 2, -1, 2], [-2, 3, -1, 2], [-2, 3, -1, 3], [-1, -2, -1, -1], [-1, -2, 0, -3], [-1, -2, 0, -2], [-1, -1, -1, 0], [-1, -1, 0, -2], [-1, -1, 0, -1], [-1, 0, -1, 1], [-1, 0, 0, -1], [-1, 0, 0, 0], [-1, 1, -1, 2], [-1, 1, 0, 0], [-1, 1, 0, 1], [-1, 2, -1, 3], [-1, 2, 0, 1], [-1, 2, 0, 2], [-1, 3, 0, 2], [-1, 3, 0, 3], [0, -3, 0, -2], [0, -3, 1, -3], [0, -2, 0, -1], [0, -2, 1, -3], [0, -2, 1, -2], [0, -1, 0, 0], [0, -1, 1, -2], [0, -1, 1, -1], [0, 0, 0, 1], [0, 0, 1, -1], [0, 0, 1, 0], [0, 1, 0, 2], [0, 1, 1, 0], [0, 1, 1, 1], [0, 2, 0, 3], [0, 2, 1, 1], [0, 2, 1, 2], [0, 3, 1, 2], [1, -3, 1, -2], [1, -3, 2, -3], [1, -2, 1, -1], [1, -2, 2, -3], [1, -2, 2, -2], [1, -1, 1, 0], [1, -1, 2, -2], [1, -1, 2, -1], [1, 0, 1, 1], [1, 0, 2, -1], [1, 0, 2, 0], [1, 1, 1, 2], [1, 1, 2, 0], [1, 1, 2, 1], [1, 2, 2, 1], [2, -3, 2, -2], [2, -3, 3, -3], [2, -2, 2, -1], [2, -2, 3, -3], [2, -2, 3, -2], [2, -1, 2, 0], [2, -1, 3, -2], [2, -1, 3, -1], [2, 0, 2, 1], [2, 0, 3, -1], [2, 0, 3, 0], [2, 1, 3, 0], [3, -3, 3, -2], [3, -2, 3, -1], [3, -1, 3, 0]];
            const solution = solutions[Math.floor(idx / 2)];
            const mirrored = idx % 2 == 0;
            return {
                reagents: [],
                products: [drawParams(
                    [{kind: 'gold', q: 3, r: 3}].concat(cells.map(([q, r]) => { return {kind: 'fire', q: (mirrored ? q + r : q) + 3, r: (mirrored ? -r : r) + 3}; })),
                    bonds.map(([sq, sr, eq, er]) => {
                        const isTriplex = solution[cells.findIndex(([q, r]) => q == sq && r == sr)] == solution[cells.findIndex(([q, r]) => q == eq && r == er)];
                        return {start: {q: (mirrored ? sq + sr : sq) + 3, r: (mirrored ? -sr : sr) + 3}, end: {q: (mirrored ? eq + er : eq) + 3, r: (mirrored ? -er : er) + 3}, red: isTriplex, black: isTriplex, yellow: isTriplex};
                    }),
                )],
            };
        ",
    ),
    LamplightGas => "Lamplight Gas", official(Journal(99, 6, &["Klusseter, S."]), "P092"),
    LapidarySaw => "Lapidary Saw", official(Journal(108, 5, &["Ames, F. L."]), "P278"),
    LapisSolaris => "Lapis Solaris", official(Journal(108, 12, &["Fauss, A.", "Mallard, J."]), "P313"),
    LatchHookFireworks => "Latch-Hook Fireworks", critelli("638f26965e21b260086e1919b264eab5"),
    LeachingAgent => "Leaching Agent", official(Journal(108, 2, &["Gue, E."]), "P263"),
    LeaveNoTrace => "Leave No Trace", critelli("Week_0_LeaveNoTrace"),
    LeaveningAgent => "Leavening Agent", official(Journal(108, 7, &["Citrus, L."]), "P287"),
    LessonArms => "Lesson: Arms", tutorial(),
    LessonBonding => "Lesson: Bonding", tutorial(),
    LessonIntroduction => "Lesson: Introduction", tutorial(),
    LessonPistons => "Lesson: Pistons", tutorial(),
    LessonPivots => "Lesson: Pivots", tutorial(),
    LessonTracks => "Lesson: Tracks", tutorial(),
    LessonTransmutation => "Lesson: Transmutation", tutorial(),
    LifeSensingPotion => "Life-Sensing Potion", official(Campaign(3), "P030b"),
    LighthouseMirror => "Lighthouse Mirror", official(Journal(108, 1, &["Servin, H."]), "P258"),
    LithargeSeparation => "Litharge Separation", official(Campaign(4), "P031b"),
    LocalAnaesthetic => "Local Anaesthetic", critelli("2414043fbe61ace7b2335186a998fcd8"),
    Lodestone => "Lodestone", official(Journal(108, 1, &["Critelli, Z."]), "P256"),
    Logistics => "Logistics", critelli("d18b0ff75237478b9f7d27f799222e46"),
    LookAndSay => "Look-And-Say", critelli_computation(
        "OM2024Weeklies_LookAndSay",
        46656,
        METALS.into_iter().array_combinations_with_reps().map(|input| {
            fn look_and_say(string: [Atom; 6]) -> Vec<Atom> {
                let mut buf = Vec::default();
                let mut idx = 0;
                while let Some(kind) = string.get(idx) {
                    idx += 1;
                    let mut count = 0;
                    while string.get(idx).is_some_and(|next_kind| next_kind == kind) {
                        count += 1;
                        idx += 1;
                    }
                    buf.push(METALS[count]);
                    buf.push(*kind);
                }
                buf
            }

            [
                vec![stick(&input)],
                vec![stick(&look_and_say(input))],
            ]
        }),
        "
            const metals = ['lead', 'tin', 'iron', 'copper', 'silver', 'gold'];
            function lookAndSay(string) {
                const buf = [];
                let idx = 0;
                while (idx < string.length) {
                    const kind = string[idx];
                    idx += 1;
                    let count = 0;
                    while (idx < string.length && string[idx] == kind) {
                        count++;
                        idx++;
                    }
                    buf.push(metals[count]);
                    buf.push(kind);
                }
                return buf;
            }
            const input = [
                metals[Math.floor(idx / 6 ** 5)],
                metals[Math.floor(idx / 6 ** 4) % 6],
                metals[Math.floor(idx / 6 ** 3) % 6],
                metals[Math.floor(idx / 6 ** 2) % 6],
                metals[Math.floor(idx / 6) % 6],
                metals[idx % 6],
            ];
            return {
                reagents: [stick(input)],
                products: [stick(lookAndSay(input))],
            };
        ",
    ),
    LubricatingFilament => "Lubricating Filament", official(Journal(99, 3, &["Klusseter, S."]), "P065"),
    LubricatingSolvents => "Lubricating Solvents", critelli("Week_3_LubricatingSolvents"),
    LuminousVapor => "Luminous Vapor", official(Journal(99, 10, &["Nick, P. A."]), "P247"),
    Lustre => "Lustre", official(Journal(99, 6, &["Servin, H."]), "P090"),
    LustrousSyrup => "Lustrous Syrup", critelli("OM2022Weeklies_LustrousSyrup"),
    MagisteryOfSaturn => "Magistery of Saturn", critelli("9679227965273efecaca286dadb4dabf"),
    Marlstone => "Marlstone", official(Journal(108, 7, &["Il Sichay, V."]), "P288"),
    MartialRegulus => "Martial Regulus", critelli("OM2022Weeklies_MartialRegulus"),
    MaterialSalvage => "Material Salvage", critelli("om2025break_Material_Salvage"),
    MemoryLane => "Memory Lane", {
        let encoding = [Atom::Salt, Atom::Fire].into_iter().array_combinations_with_reps::<5>()
            .zip_eq(METALS.into_iter().array_combinations_with_reps::<2>().collect_vec().partial_shuffle(&mut rng(), 32).0)
            .map(|(input, output)| [
                vec![stick(&input)],
                vec![stick(output)],
            ])
            .collect_vec();
        critelli_computation(
            "OM2025week8_Memory_Lane",
            32,
            encoding.clone(),
            js_get_permutation_serverside(encoding),
        )
    },
    MetalCalculus => "Metal Calculus", computation(
        "https://reddit.com/r/opus_magnum/comments/flp70t/the_final_week_of_the_tournament_metal_calculus/",
        10077696,
        METALS.into_iter().array_combinations_with_reps().map(|input| {
            fn derivative(string: [Atom; 9]) -> Vec<Atom> {
                string.into_iter().array_windows().map(|[a, b]| match METALS.into_iter().position(|iter_atom| iter_atom == b).unwrap() as i8 - METALS.into_iter().position(|iter_atom| iter_atom == a).unwrap() as i8 {
                    ..=-3 => Atom::Mors,
                    -2 => Atom::Earth,
                    -1 => Atom::Water,
                    0 => Atom::Salt,
                    1 => Atom::Fire,
                    2 => Atom::Air,
                    3.. => Atom::Vitae,
                }).collect()
            }

            [
                vec![stick(&input)],
                vec![stick(&derivative(input))],
            ]
        }),
        "
            const metals = ['lead', 'tin', 'iron', 'copper', 'silver', 'gold'];
            function derivative(string) {
                const buf = [];
                for (let idx = 0; idx < 8; idx++) {
                    const diff = metals.indexOf(string[idx + 1]) - metals.indexOf(string[idx]);
                    if (diff < -2) {
                        buf.push('mors');
                    } else {
                        switch (diff) {
                            case -2: {
                                buf.push('earth');
                                break;
                            }
                            case -1: {
                                buf.push('water');
                                break;
                            }
                            case 0: {
                                buf.push('salt');
                                break;
                            }
                            case 1: {
                                buf.push('fire');
                                break;
                            }
                            case 2: {
                                buf.push('air');
                                break;
                            }
                            default: {
                                buf.push('vitae');
                                break;
                            }
                        }
                    }
                }
                return buf;
            }
            const input = [
                metals[Math.floor(idx / 6 ** 8)],
                metals[Math.floor(idx / 6 ** 7) % 6],
                metals[Math.floor(idx / 6 ** 6) % 6],
                metals[Math.floor(idx / 6 ** 5) % 6],
                metals[Math.floor(idx / 6 ** 4) % 6],
                metals[Math.floor(idx / 6 ** 3) % 6],
                metals[Math.floor(idx / 6 ** 2) % 6],
                metals[Math.floor(idx / 6) % 6],
                metals[idx % 6],
            ];
            return {
                reagents: [stick(input)],
                products: [stick(derivative(input))],
            };
        ",
    ),
    MetalDivision => "Metal Division", official(Drm(2), "P208"),
    MetallicTincture => "Metallic Tincture", official(Drm(1), "P206"),
    MildSteel => "Mild Steel", official(Journal(108, 11, &["Kagami, T."]), "P308"),
    MineralOil => "Mineral Oil", official(Journal(108, 11, &["Kia, J."]), "P307"),
    MiraculousAutosalt => "Miraculous Autosalt", zlbb("w1698787102", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    MiraculousDentifrice => "Miraculous Dentifrice", official(Journal(108, 3, &["Objectus, R."]), "P266"),
    MirrorPolish => "Mirror Polish", official(Journal(108, 3, &["Xenas, M."]), "P269"),
    MirroringAmalgam => "Mirroring Amalgam", official(Journal(108, 9, &["Il Sichay, V."]), "P298"),
    MistOfClarification => "Mist of Clarification", official(Journal(108, 10, &["Wallace, A."]), "P300"),
    MistOfDousing => "Mist of Dousing", zlbb("w2450512021", "https://drive.google.com/drive/folders/1JX9JEdzXfFgn1-z4Yno_oMjSHHg8eGxE"),
    MistOfGlaciation => "Mist of Glaciation", official(Journal(108, 6, &["Mann, M."]), "P283"),
    MistOfHallucination => "Mist of Hallucination", official(Campaign(5), "P038"),
    MistOfIncapacitation => "Mist of Incapacitation", official(Campaign(2), "P018"),
    MixedUseLubricant => "Mixed-Use Lubricant", official(Drm(1), "P205"),
    MoonlightCatalyst => "Moonlight Catalyst", critelli("OM2025Weeklies11_MoonlightCatalyst"),
    MooringCable => "Mooring Cable", official(Journal(108, 1, &["Pugano, P."]), "P255"),
    MosaicTessera => "Mosaic Tessera", official(Journal(108, 10, &["Chi, X. N."]), "P304"),
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
    ParadeRocketFuel => "Parade-Rocket Fuel", official(Appendix, "P082"),
    ParetoPoppers => "Pareto Poppers", critelli("OM2025Weeklies4_ParetoPoppers"),
    ParticleReconstruction => "Particle Reconstruction", {
        let permutations = [
            collect![as HashSet<_>: Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal }],
            collect![Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 0, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal }],
            collect![Bond { start: HexIndex { q: 0, r: 0 }, end: HexIndex { q: 0, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 0 }, ty: BondType::Normal }],
        ].into_iter().map(|bonds| [
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => Atom::Earth, HexIndex { q: 0, r: 1 } => Atom::Water, HexIndex { q: 1, r: 0 } => Atom::Fire], bonds: bonds.clone() }],
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => Atom::Earth, HexIndex { q: 0, r: 1 } => Atom::Water, HexIndex { q: 1, r: 0 } => Atom::Fire], bonds }],
        ]).collect_vec();
        critelli_computation(
            "OM2023Weeklies_ParticleReconstruction",
            3,
            permutations.clone(),
            js_get_permutation_serverside(permutations),
        )
    },
    PassThroughAlloy => "Pass-Through Alloy", critelli("OM2025week7_Pass-Through_Alloy"),
    PatternMetal => "Pattern Metal", official(Drm(3), "P221"),
    PhilosophersCatalyst => "Philosopher's Catalyst", critelli("OM2022Weeklies_PhiloCatalyst"),
    PigIron => "Pig Iron", official(Journal(108, 2, &["Woodstalker, H."]), "P260"),
    PitchDropExperiment => "Pitch Drop Experiment", critelli("OM2024Weeklies_PitchDropExperiment"),
    Plastic => "Plastic", critelli("ac6ab9dc43b0ac7ad9277c25ed5375e1"),
    PotentPainkillers => "Potent Painkillers", critelli("Week_7_PotentPainkillers"),
    PotentPotables => "Potent Potables", zlbb("w2501727721", "https://reddit.com/r/opus_magnum/comments/et5lyo/tournament_week_1_potent_potables/"),
    PousseCafe => "Pousse-Café", critelli("f883eb7701f420e1b1960eabe37b7fc1"),
    PrecisionMachineOil => "Precision Machine Oil", official(Campaign(1), "P012"),
    PreservativeSalt => "Preservative Salt", official(Journal(99, 2, &["Fontenelle, A."]), "P060"),
    PreservingWax => "Preserving Wax", official(Journal(108, 3, &["Il Sichay, V."]), "P267"),
    ProbeModule => "Probe Module", critelli("OM2023_W5_ProbeModule"),
    ProofOfCompleteness => "Proof of Completeness", official(Journal(99, 4, &["Servin, H."]), "P069"),
    ProspectorsSolvent => "Prospector's Solvent", official(Journal(108, 2, &["Rebbick, S."]), "P261"),
    PurifiedGold => "Purified Gold", official(Campaign(4), "P036"),
    QuickeningCordial => "Quickening Cordial", official(Journal(108, 8, &["Adam, V."]), "P290"),
    QuietHours => "Quiet Hours", critelli("ff6feb9ee69a0450a117eb2a7c7de784"),
    QuintessentialAerogel => "Quintessential Aerogel", critelli("OM2022Weeklies_QuintAerogel"),
    QuintessentialCatalyst => "Quintessential Catalyst", other("https://discord.com/channels/278707932089155584/296373951800541186/877363315687436349"),
    QuintessentialExplosive => "Quintessential Explosive", critelli("OM2023Weeklies_QuintExplosive"),
    QuintessentialMedium => "Quintessential Medium", official(Journal(99, 9, &["Price, A."]), "P107"),
    QuintessentialStabilizer => "Quintessential Stabilizer", zlbb("w2450512626", "https://drive.google.com/drive/folders/1sommL5qdrN8fa0-D_dnwEJxVlwu34QgT"),
    RadioReceivers => "Radio Receivers", critelli("Week_8_RadioReceivers"),
    RatPoison => "Rat Poison", official(Appendix, "P074"),
    RavarisRage => "Ravari's Rage", other("https://drive.google.com/drive/folders/1iy7KDmdkO4HGbjD1_jX_qb8mQ2IeSmS1"),
    RavarisRoad => "Ravari's Road", critelli("OM2025Weeklies7_RavarisRoad"),
    RavarisWheel => "Ravari's Wheel", official(Journal(99, 3, &["Ravari, V."]), "P064"),
    ReactiveCinnabar => "Reactive Cinnabar", official(Journal(99, 1, &["Van Berlo, C."]), "P056"),
    ReactiveGold => "Reactive Gold", official(Journal(99, 7, &["Adam, V."]), "P095"),
    ReactiveLead => "Reactive Lead", official(Drm(2), "P210"),
    RealgarSeparation => "Realgar Separation", official(Journal(108, 2, &["Thinnes, Y."]), "P264"),
    RecipeForDisaster => "Recipe for Disaster", critelli("OM2025Weeklies10_RecipeForDisaster"),
    ReclaimedGold => "Reclaimed Gold", official(Journal(99, 12, &["Rallus, U."]), "P254"),
    ReconstructedSolvent => "Reconstructed Solvent", official(Appendix, "P084"),
    RefinedBronze => "Refined Bronze", official(Journal(99, 3, &["Wong, E."]), "P067"),
    RefinedGold => "Refined Gold", official(Campaign(1), "P010"),
    ResinGlue => "Resin Glue", official(Journal(108, 11, &["Apia, B."]), "P309"),
    ResonantCrystal => "Resonant Crystal", official(Journal(99, 3, &["Thiele, J."]), "P066"),
    ResonantDust => "Resonant Dust", official(Journal(99, 11, &["Azur, I."]), "P243"),
    RetroRefining => "Retro Refining", critelli("OM2024Weeklies_RetroRefining"),
    RibbedFletching => "Ribbed Fletching", official(Journal(108, 4, &["Zarc, T."]), "P273"),
    RingEnlargement => "Ring Enlargement", critelli("OM2023Weeklies_RingEnlargement"),
    RocketPropellant => "Rocket Propellant", official(Campaign(2), "P019"),
    RockwoolFiber => "Rockwool Fiber", official(Journal(108, 11, &["Silva, J."]), "P306"),
    Rosewater => "Rosewater", official(Journal(108, 7, &["Alinejad, Z."]), "P285"),
    Roshambonite => "Roshambonite", critelli("b9ec009e82738509a09e554f359fb300"),
    RustRemoval => "Rust Removal", critelli("Week_1_RustRemoval"),
    SailclothThread => "Sailcloth Thread", official(Journal(99, 2, &["Toscano, G."]), "P061"),
    SaltPackagingFactory => "Salt Packaging Factory", critelli("OM2023Weeklies_SaltPackagingFactory"),
    SaltOfHartshorn => "Salt of Hartshorn", official(Journal(108, 6, &["Mâché, M."]), "P281"),
    SaltOfSaturnByVinegar => "Salt of Saturn by Vinegar", critelli("9cec1eb8ff25a6ee4c73c77176524146"),
    SandOfSuspension => "Sand of Suspension", critelli("OM2025Weeklies2_SandOfSuspension"),
    SaturnsTree => "Saturn's Tree", critelli("9358b0efab6c1f32b850f898b8b2a675"),
    SaveriosTest => "Saverio's Test", official(Drm(1), "P201"),
    SaveriosTransformer => "Saverio's Transformer", official(Drm(1), "P204"),
    ScrapMetal => "Scrap Metal", critelli("a1dcf5402132b1b4c28d63ef02942db8"),
    SealSolvent => "Seal Solvent", official(Campaign(3), "P026"),
    SelfOrganizingFluid => "Self-Organizing Fluid", critelli("OM2025week3_Self-Organizing_Fluid"),
    SelfPressurizingGas => "Self-Pressurizing Gas", critelli("OM2023_W1_SelfPressurizingGas"),
    SeptstabilizedSalt => "Septstabilized Salt", critelli("9c14e48d17cebae4165828037b56cc7c"),
    ServinsWheel => "Servin's Wheel", critelli("OM2022Weeklies_ServinsWheel"),
    ShimmeringChain => "Shimmering Chain", official(Journal(108, 9, &["Mallard, J."]), "P295"),
    SigmarsGarden => "Sigmar's Garden", critelli("af2e9ada2b4a888463d1aef200c58eb9"),
    SilverAppleOfDiscord => "Silver Apple of Discord", critelli("3e07b2ebbabd5ea7e9b87f8cd35d679f"),
    SilverCaustic => "Silver Caustic", official(Journal(99, 1, &["Van Berlo, C."]), "P057"),
    SilverDust => "Silver Dust", official(Drm(1), "P203"),
    SilverPaint => "Silver Paint", official(Appendix, "P076"),
    Simulacrum => "Simulacrum", critelli("ff3f211965e5ba83ac8a080335ddd04e"),
    SleepingTonic => "Sleeping Tonic", official(Journal(99, 11, &["Axton, H."]), "P242"),
    SmogNeutralization => "Smog Neutralization", critelli("OM2025Weeklies5_SmogNeutralization"),
    SnowAmputation => "Snow Amputation", critelli("0d8384f8b042a93f2cbb1af82c339b9c"),
    SodaAsh => "Soda Ash", official(Journal(108, 3, &["Gue, E."]), "P265"),
    SolderWire => "Solder Wire", official(Drm(2), "P209"),
    SoothingSalve => "Soothing Salve", critelli("Week_2_SoothingSalve"),
    SophickMercury => "Sophick Mercury (2024 weeklies)", critelli("OM2024Weeklies_SophickMercury"),
    SophickMercuryJournal => "Sophick Mercury (journal)", official(Journal(99, 12, &["Il Sichay, V.", "Nick, P. A."]), "P250"),
    SparkingPyrite => "Sparking Pyrite", official(Journal(108, 4, &["Azur, I."]), "P274"),
    SpecialAmaro => "Special Amaro", official(Appendix, "P083"),
    SpyglassCrystal => "Spyglass Crystal", official(Journal(99, 2, &["Bruckner, J."]), "P063"),
    StabilizedEverything => "Stabilized Everything", other("https://discord.com/channels/278707932089155584/296373951800541186/882448331119427604"),
    StabilizedGold => "Stabilized Gold", critelli("OM2022Weeklies_StabilizedGold"),
    StabilizedWater => "Stabilized Water", official(Prologue, "P007"),
    StainRemover => "Stain Remover", official(Campaign(4), "P034"),
    StaminaPotion => "Stamina Potion", official(Campaign(1), "P015"),
    SteelWool => "Steel Wool", official(Journal(108, 3, &["Joshua, Y."]), "P268"),
    StormSensingPotion => "Storm-Sensing Potion", official(Journal(108, 8, &["Ames, F. L."]), "P294"),
    SuperconductiveCopper => "Superconductive Copper", other("https://discord.com/channels/278707932089155584/296373951800541186/854533289816358922"),
    SurrenderFlare => "Surrender Flare", official(Campaign(2), "P022"),
    SurveyingMagnet => "Surveying Magnet", official(Journal(108, 2, &["Warden, I."]), "P262"),
    SuspiciouslyStableSubstance => "Suspiciously Stable Substance", critelli("OM2022Weeklies_SSS"),
    SutureThread => "Suture Thread", official(Journal(99, 5, &["Rodrigues, I."]), "P085"),
    SwampFiber => "Swamp Fiber", zlbb("w2501727889", "https://reddit.com/r/opus_magnum/comments/f05mp5/week_3_swamp_fiber/"),
    SweeperRod => "Sweeper Rod", critelli("OM2022Weeklies_SweeperRod"),
    SwordAlloy => "Sword Alloy", official(Campaign(4), "P033"),
    SynthesisViaAlcohol => "Synthesis via Alcohol", official(Journal(99, 4, &["Servin, H."]), "P071"),
    SyntheticMalachite => "Synthetic Malachite", official(Journal(99, 9, &["Klusseter, S."]), "P109"),
    Taricene => "Taricene", critelli("21042bbbbef69aef2000df9b97a4df9b"),
    TaxFraud => "Tax Fraud", critelli("a39b83445002fbdea218e8b184a9e478"),
    TemperedGlass => "Tempered Glass", official(Journal(108, 10, &["Tineri, P."]), "P301"),
    TheAmazingEverythingMachine => "The Amazing Everything-Machine", critelli_computation(
        "OM2025week4_The_Amazing_Everything-Machine",
        10,
        CARDINALS.into_iter().chain(METALS).map(|atom| [
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => atom], bonds: collect![] }],
            vec![Molecule { atoms: collect![HexIndex { q: 0, r: 0 } => atom], bonds: collect![] }],
        ]),
        "
            const atoms = ['earth', 'air', 'water', 'fire', 'lead', 'tin', 'iron', 'copper', 'silver', 'gold'];
            const atom = atoms[idx];
            return {
                reagents: [stick([atom])],
                products: [stick([atom])],
            };
        ",
    ),
    ThermalFuse => "Thermal Fuse", critelli("d3689000418b9687654554f28324d8d0"),
    ThermicCapacitor => "Thermic Capacitor", critelli("om2025week2_Thermic_Capacitor"),
    ThermiteTape => "Thermite Tape", critelli("OM2025Weeklies3_ThermiteTape"),
    TimingCrystal => "Timing Crystal", official(Campaign(5), "P042"),
    Tinsel => "Tinsel", critelli("bf33acfc5ce6933ae39b8ee5deb663af"),
    TonicOfHydration => "Tonic of Hydration", official(Journal(99, 5, &["Klusseter, S."]), "P089"),
    TonicOfTransmogrification => "Tonic of Transmogrification", critelli_computation(
        "OM2023Weeklies_TransTonic",
        9916,
        (0..9916).map(|index| [
            vec![state_from_enumeration_index(index)],
            vec![],
        ]),
        "
            return {
                reagents: [drawParamsFromState(stateForEnumerationIndex(idx))],
                products: [],
            };
        ",
    ),
    TouchGrass => "Touch Grass", critelli("OM2024Weeklies_TouchGrass"),
    Touchstone => "Touchstone (2024 tournament)", critelli("6f37903681423b320da82fb57900291d"),
    TouchstoneJournal => "Touchstone (journal)", official(Journal(99, 10, &["Critelli, Z."]), "P245"),
    Transmutation110 => "Transmutation CX", {
        let permutations = all().array_combinations_with_reps().map(|[a, b, c, d, e, f]| {
            fn i(cond: bool) -> Atom {
                if cond { Atom::Fire } else { Atom::Salt }
            }

            fn o(a: bool, b: bool, c: bool) -> Atom {
                match (a, b, c) {
                    (false, false, false) => Atom::Salt,
                    (false, false, true) => Atom::Fire,
                    (false, true, false) => Atom::Fire,
                    (false, true, true) => Atom::Fire,
                    (true, false, false) => Atom::Salt,
                    (true, false, true) => Atom::Fire,
                    (true, true, false) => Atom::Fire,
                    (true, true, true) => Atom::Salt,
                }
            }

            [
                vec![Molecule { atoms: collect![HexIndex { q: 0, r: 1 } => i(f), HexIndex { q: 0, r: 2 } => i(e), HexIndex { q: 1, r: 0 } => i(a), HexIndex { q: 1, r: 1 } => Atom::Gold, HexIndex { q: 1, r: 2 } => i(d), HexIndex { q: 2, r: 0 } => i(b), HexIndex { q: 2, r: 1 } => i(c)], bonds: collect![Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 2 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 1, r: 2 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 1 }, ty: BondType::Normal }] }],
                vec![Molecule { atoms: collect![HexIndex { q: 0, r: 1 } => o(e, f, a), HexIndex { q: 0, r: 2 } => o(d, e, f), HexIndex { q: 1, r: 0 } => o(f, a, b), HexIndex { q: 1, r: 1 } => Atom::Gold, HexIndex { q: 1, r: 2 } => o(c, d, e), HexIndex { q: 2, r: 0 } => o(a, b, c), HexIndex { q: 2, r: 1 } => o(b, c, d)], bonds: collect![Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 2 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 1, r: 2 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 1 }, ty: BondType::Normal }] }],
            ]
        }).collect_vec();
        critelli_computation(
            "Week_9_TransmutationCX",
            64,
            permutations.clone(),
            js_get_permutation_serverside(permutations),
        )
    },
    UmbralMascara => "Umbral Mascara", official(Journal(108, 8, &["Demos, A."]), "P292"),
    UniversalCompound => "Universal Compound", official(Journal(99, 4, &["Tolomeo, N."]), "P072"),
    UniversalOintment => "Universal Ointment", official(Journal(108, 12, &["Demos, A."]), "P314"),
    UniversalSolvent => "Universal Solvent", official(Campaign(5), "P043"),
    UnnamedCustomPuzzle => "Unnamed Custom Puzzle", critelli_computation(
        "OM2025Weeklies12_Unnamed",
        9916,
        (0..9916).map(|index| [
            vec![state_from_enumeration_index(index)],
            vec![state_from_enumeration_index(index)],
        ]),
        "
            return {
                reagents: [drawParamsFromState(stateForEnumerationIndex(idx))],
                products: [drawParamsFromState(stateForEnumerationIndex(idx))],
            };
        ",
    ),
    UnstableCompound => "Unstable Compound", official(Campaign(5), "P040"),
    UnstableSovrium => "Unstable Sovrium", critelli("OM2024Weeklies_UnstableSovrium"),
    Unwinding => "Unwinding", zlbb("w1611998067", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    VaccineTemplate => "Vaccine Template", critelli("5d936a1ca336f6658f097260eda56a8f"),
    VanBerlosChain => "Van Berlo's Chain", official(Journal(99, 1, &["Van Berlo, C."]), "P055"),
    VanBerlosPivots => "Van Berlo's Pivots", official(Journal(99, 7, &["Kryger, L."]), "P096"),
    VanBerlosWheel => "Van Berlo's Wheel", official(Journal(99, 1, &["Van Berlo, C."]), "P054"),
    VanishingMaterial => "Vanishing Material", official(Journal(99, 9, &["Price, A."]), "P105"),
    VaporOfLevity => "Vapor of Levity", official(Appendix, "P078"),
    VaporizedPropellant => "Vaporized Propellant", other("https://discord.com/channels/278707932089155584/296373951800541186/872298625798119455"),
    VaporousSolvent => "Vaporous Solvent", official(Journal(99, 7, &["Klusseter, S."]), "P098"),
    VerdigrisGlaze => "Verdigris Glaze", official(Journal(108, 10, &["Ironside, F."]), "P302"),
    VermilionPigment => "Vermilion Pigment", official(Journal(108, 5, &["Adam, V."]), "P276"),
    VeryDarkThread => "Very Dark Thread", official(Campaign(3), "P029"),
    ViaMedia => "Via Media", official(Journal(108, 6, &["Azur, I."]), "P284"),
    ViaQuicksilver => "Via Quicksilver", official(Drm(3), "P216"),
    VirulentVector => "Virulent Vector", zlbb("w1698785238", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    ViscousAdhesive => "Viscous Adhesive", critelli("OM2023Weeklies_ViscousAdhesive"),
    ViscousSludge => "Viscous Sludge", official(Appendix, "P080"),
    VisillaryAnaesthetic => "Visillary Anaesthetic", official(Journal(99, 8, &["Rallus, U."]), "P102"),
    VolatilityAndTranquility => "Volatility and Tranquility", zlbb("w2501727977", "https://reddit.com/r/opus_magnum/comments/f3n89y/week_4_volatility_and_tranquility/"),
    VoltaicCoil => "Voltaic Coil", official(Campaign(5), "P039"),
    WakefulnessPotion => "Wakefulness Potion", official(Journal(99, 5, &["Foudre, G."]), "P088"),
    WarmingTonic => "Warming Tonic", official(Campaign(3), "P028"),
    WarpFuel => "Warp Fuel", critelli("OM2023_W7_WarpFuel"),
    WasteReclamation => "Waste Reclamation", critelli("OM2023_W2_WasteReclamation"),
    WaterPurifier => "Water Purifier", official(Campaign(3), "P025"),
    WaterproofSealant => "Waterproof Sealant", official(Campaign(1), "P011"),
    WeldingThermite => "Welding Thermite", official(Journal(99, 6, &["Le'Feur, T."]), "P094"),
    WheelInversion => "Wheel Inversion", computation(
        "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/",
        15625,
        CARDINALS.into_iter().chain(iter::once(Atom::Salt)).array_combinations_with_reps().map(|[a, b, c, d, e, f]| {
            fn invert(atom: Atom) -> Atom {
                match atom {
                    Atom::Earth => Atom::Air,
                    Atom::Air => Atom::Earth,
                    Atom::Water => Atom::Fire,
                    Atom::Fire => Atom::Water,
                    _ => atom,
                }
            }

            [
                vec![Molecule { atoms: collect![HexIndex { q: 0, r: 1 } => a, HexIndex { q: 0, r: 2 } => b, HexIndex { q: 1, r: 0 } => c, HexIndex { q: 1, r: 1 } => Atom::Gold, HexIndex { q: 1, r: 2 } => d, HexIndex { q: 2, r: 0 } => e, HexIndex { q: 2, r: 1 } => f], bonds: collect![Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 2 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 1, r: 2 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 1 }, ty: BondType::Normal }] }],
                vec![Molecule { atoms: collect![HexIndex { q: 0, r: 1 } => invert(a), HexIndex { q: 0, r: 2 } => invert(b), HexIndex { q: 1, r: 0 } => invert(c), HexIndex { q: 1, r: 1 } => Atom::Gold, HexIndex { q: 1, r: 2 } => invert(d), HexIndex { q: 2, r: 0 } => invert(e), HexIndex { q: 2, r: 1 } => invert(f)], bonds: collect![Bond { start: HexIndex { q: 0, r: 1 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 0, r: 2 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 0 }, end: HexIndex { q: 1, r: 1 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 1, r: 2 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 0 }, ty: BondType::Normal }, Bond { start: HexIndex { q: 1, r: 1 }, end: HexIndex { q: 2, r: 1 }, ty: BondType::Normal }] }],
            ]
        }),
        "
            const atoms = ['earth', 'air', 'water', 'fire', 'salt'];
            const a = atoms[Math.floor(idx / 5 ** 5)];
            const b = atoms[Math.floor(idx / 5 ** 4) % 5];
            const c = atoms[Math.floor(idx / 5 ** 3) % 5];
            const d = atoms[Math.floor(idx / 5 ** 2) % 5];
            const e = atoms[Math.floor(idx / 5) % 5];
            const f = atoms[idx % 5];
            function invert(atom) {
                switch (atom) {
                    case 'earth': return 'air';
                    case 'air': return 'earth';
                    case 'water': return 'fire';
                    case 'fire': return 'water';
                    default: return atom;
                }
            }
            return {
                reagents: [drawParams([{kind: a, q: 0, r: 1}, {kind: b, q: 0, r: 2}, {kind: c, q: 1, r: 0}, {kind: 'gold', q: 1, r: 1}, {kind: d, q: 1, r: 2}, {kind: e, q: 2, r: 0}, {kind: f, q: 2, r: 1}], [{start: {q: 0, r: 1}, end: {q: 1, r: 1}, red: false, black: false, yellow: false}, {start: {q: 0, r: 2}, end: {q: 1, r: 1}, red: false, black: false, yellow: false}, {start: {q: 1, r: 0}, end: {q: 1, r: 1}, red: false, black: false, yellow: false}, {start: {q: 1, r: 1}, end: {q: 1, r: 2}, red: false, black: false, yellow: false}, {start: {q: 1, r: 1}, end: {q: 2, r: 0}, red: false, black: false, yellow: false}, {start: {q: 1, r: 1}, end: {q: 2, r: 1}, red: false, black: false, yellow: false}])],
                products: [drawParams([{kind: invert(a), q: 0, r: 1}, {kind: invert(b), q: 0, r: 2}, {kind: invert(c), q: 1, r: 0}, {kind: 'gold', q: 1, r: 1}, {kind: invert(d), q: 1, r: 2}, {kind: invert(e), q: 2, r: 0}, {kind: invert(f), q: 2, r: 1}], [{start: {q: 0, r: 1}, end: {q: 1, r: 1}, red: false, black: false, yellow: false}, {start: {q: 0, r: 2}, end: {q: 1, r: 1}, red: false, black: false, yellow: false}, {start: {q: 1, r: 0}, end: {q: 1, r: 1}, red: false, black: false, yellow: false}, {start: {q: 1, r: 1}, end: {q: 1, r: 2}, red: false, black: false, yellow: false}, {start: {q: 1, r: 1}, end: {q: 2, r: 0}, red: false, black: false, yellow: false}, {start: {q: 1, r: 1}, end: {q: 2, r: 1}, red: false, black: false, yellow: false}])],
            };
        ",
    ),
    WheelRepresentation => "Wheel Representation", official(Journal(99, 4, &["Servin, H."]), "P070"),
    WireFormingAndUnforming => "Wire Forming and Unforming", zlbb("w1698784331", "https://reddit.com/r/opus_magnum/comments/abpxj8/opus_magnum_tourney/"),
    XylemSubstitute => "Xylem Substitute", official(Journal(108, 9, &["Clover, C."]), "P297"),
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
