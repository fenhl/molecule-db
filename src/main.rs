#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        borrow::Cow,
        cmp::Ordering::{
            self,
            *,
        },
        collections::{
            HashMap,
            HashSet,
        },
        fmt,
        mem,
        num::NonZero,
    },
    async_proto::Protocol,
    base64::engine::{
        Engine as _,
        general_purpose::URL_SAFE as BASE64,
    },
    enum_iterator::all,
    gophermap::GopherEntry,
    itertools::Itertools as _,
    lazy_regex::regex_captures,
    omsim_rs::{
        data::*,
        parse::parse_puzzle,
    },
    reqwest as _, // gix TLS backend config
    rocket::{
        data::ToByteUnit as _,
        form,
        fs::FileServer,
        http::{
            Status,
            impl_from_uri_param_identity,
            uri::fmt::{
                Formatter,
                Query,
                UriDisplay,
            },
        },
        response::content::RawHtml,
        serde::json::Json,
        uri,
    },
    rocket_util::{
        Doctype,
        html,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    tokio::{
        io::{
            AsyncBufReadExt as _,
            AsyncReadExt as _,
            AsyncWriteExt as _,
            BufReader,
        },
        net::TcpStream,
        process::Command,
    },
    url::Url,
    wheel::{
        fs,
        traits::{
            AsyncCommandOutputExt as _,
            IoResultExt as _,
        },
    },
    crate::{
        puzzle::Puzzle,
        proto::FormMolecule,
        unparse::Unparse,
        util::{
            IntExt as _,
            IteratorExt as _,
        },
    },
};
#[cfg(any(target_os = "windows", target_os = "linux"))] use directories::UserDirs;
#[cfg(target_os = "macos")] use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/static_files.rs"));

mod molecules;
mod proto;
mod puzzle;
mod unparse;
mod util;

#[derive(Serialize)]
enum InOut {
    Reagent,
    Product,
    Both,
}

impl InOut {
    fn is_reagent(&self) -> bool {
        match self {
            Self::Product => false,
            Self::Reagent | Self::Both => true,
        }
    }

    fn is_product(&self) -> bool {
        match self {
            Self::Reagent => false,
            Self::Product | Self::Both => true,
        }
    }
}

trait MoleculeExt {
    fn min_radius(&self) -> Result<NonZero<u8>, MoleculeTooLarge>;
    fn position_normalized(&self) -> Self;
    fn normalized(&self) -> Self;
    fn mirrored(&self) -> Self;
    fn draw(&self, id: &str) -> RawHtml<String>;
}

impl MoleculeExt for Molecule {
    fn min_radius(&self) -> Result<NonZero<u8>, MoleculeTooLarge> {
        let mut min_radius = NonZero::<u8>::MIN;
        if !self.atoms.is_empty() {
            for rotation in [HexRotation::R0, HexRotation::R60, HexRotation::R120] {
                let molecule = self.rotated(HexIndex::default(), rotation);
                let (min, max) = molecule.atoms.keys().minmax_by_key(|HexIndex { q, r }| q + r).into_option().expect("molecule has no atoms, checked above");
                let min_offset = min.q + min.r;
                let max_offset = max.q + max.r;
                let dir_min_radius = NonZero::new((max_offset + 2 - min_offset).div_ceil(2).try_into().map_err(|_| MoleculeTooLarge { radius: NonZero::<u8>::MAX, min_radius: NonZero::<u8>::MAX })?).expect("max_offset should always be ≥ min_offset so the division result should always be ≥ 1");
                min_radius = min_radius.max(dir_min_radius);
            }
        }
        Ok(min_radius)
    }

    fn position_normalized(&self) -> Self {
        let offset = HexIndex {
            q: self.atoms.keys().map(|&HexIndex { q, .. }| q).min().unwrap_or_default(),
            r: self.atoms.keys().map(|&HexIndex { r, .. }| r).min().unwrap_or_default(),
        };
        let mut normalized = self.mapped_positions(|pos| pos - offset);
        normalized.bonds = normalized.bonds.into_iter().map(|Bond { start, end, ty }| Bond {
            start: if (start.q, start.r) <= (end.q, end.r) { start } else { end },
            end: if (start.q, start.r) <= (end.q, end.r) { end } else { start },
            ty,
        }).collect();
        normalized
    }

    fn normalized(&self) -> Self {
        fn atom_id(atom: Atom) -> u8 {
            match atom {
                Atom::Salt => 1,
                Atom::Air => 2,
                Atom::Earth => 3,
                Atom::Fire => 4,
                Atom::Water => 5,
                Atom::Quicksilver => 6,
                Atom::Gold => 7,
                Atom::Silver => 8,
                Atom::Copper => 9,
                Atom::Iron => 10,
                Atom::Tin => 11,
                Atom::Lead => 12,
                Atom::Vitae => 13,
                Atom::Mors => 14,
                Atom::Repeat => 15,
                Atom::Quintessence => 16,
            }
        }

        fn cmp_atoms((k1, v1): (&HexIndex, &Atom), (k2, v2): (&HexIndex, &Atom)) -> Ordering {
            k1.q.cmp(&k2.q)
            .then_with(|| k1.r.cmp(&k2.r))
            .then_with(|| atom_id(*v1).cmp(&atom_id(*v2)))
        }

        fn cmp_atoms_ref(&p1: &(&HexIndex, &Atom), &p2: &(&HexIndex, &Atom)) -> Ordering {
            cmp_atoms(p1, p2)
        }

        fn cmp_bond_types(t1: &BondType, t2: &BondType) -> Ordering {
            match (t1, t2) {
                (BondType::Normal, BondType::Normal) => Equal,
                (BondType::Normal, BondType::Triplex { .. }) => Less,
                (BondType::Triplex { .. }, BondType::Normal) => Greater,
                (BondType::Triplex { red: r1, black: b1, yellow: y1 }, BondType::Triplex { red: r2, black: b2, yellow: y2 }) => r1.cmp(r2).then_with(|| b1.cmp(b2)).then_with(|| y1.cmp(y2)),
            }
        }

        fn cmp_bonds(b1: &Bond, b2: &Bond) -> Ordering {
            let b1_start = (b1.start.q, b1.start.r);
            let b1_end = (b1.end.q, b1.end.r);
            let b1_min = b1_start.min(b1_end);
            let b1_max = b1_start.max(b1_end);
            let b2_start = (b2.start.q, b2.start.r);
            let b2_end = (b2.end.q, b2.end.r);
            let b2_min = b2_start.min(b2_end);
            let b2_max = b2_start.max(b2_end);
            b1_min.cmp(&b2_min)
            .then_with(|| b1_max.cmp(&b2_max))
            .then_with(|| cmp_bond_types(&b1.ty, &b2.ty))
        }

        fn cmp_bonds_ref(&b1: &&Bond, &b2: &&Bond) -> Ordering {
            cmp_bonds(b1, b2)
        }

        if self.atoms.values().any(|&atom| atom == Atom::Repeat) {
            self.position_normalized()
        } else {
            all().map(|rotation| self.rotated(HexIndex::default(), rotation).position_normalized()).min_by(|m1, m2|
                m1.atoms.iter().sorted_unstable_by(cmp_atoms_ref)._cmp_by(m2.atoms.iter().sorted_unstable_by(cmp_atoms_ref), cmp_atoms)
                .then_with(|| m1.bonds.iter().sorted_unstable_by(cmp_bonds_ref)._cmp_by(m2.bonds.iter().sorted_unstable_by(cmp_bonds_ref), cmp_bonds))
            ).expect("all::<Rotation>() is nonempty") //TODO make a nonempty variant of all()
        }
    }

    fn mirrored(&self) -> Self {
        self.mapped_positions(|pos| HexIndex { q: -pos.s(), r: -pos.r, }).position_normalized()
    }

    fn draw(&self, id: &str) -> RawHtml<String> {
        let Self { atoms, bonds } = self.mirrored();
        let min_x = atoms.keys().map(|&HexIndex { q, r }| 2 * q + r).min().unwrap_or_default();
        let width = atoms.keys().map(|&HexIndex { q, r }| 2 * q + r + 2).max().unwrap_or_default() - min_x;
        let height = atoms.keys().map(|&HexIndex { r, .. }| r + 1).max().unwrap_or_default();
        let width = (41 * width + 10) * 3 / 4;
        let height = (71 * height + 20) * 3 / 4;
        html! {
            canvas(class = "molecule-canvas", id = id);
            script {
                : RawHtml(format!("
                    const productCanvas{id} = document.getElementById({id:?});
                    productCanvas{id}.width = {width} * window.devicePixelRatio;
                    productCanvas{id}.style.width = '{width}px';
                    productCanvas{id}.height = {height} * window.devicePixelRatio;
                    productCanvas{id}.style.height = '{height}px';
                    const pctx{id} = productCanvas{id}.getContext('2d');
                    pctx{id}.scale(window.devicePixelRatio, window.devicePixelRatio);
                    pctx{id}.scale(0.75, 0.75);
                    pctx{id}.fillStyle = '#223';
                    for (let shadow = 4; shadow >= 0; shadow -= 4) {{
                "));
                @for Bond { start, end, ty } in bonds {
                    @let _ = ty; //TODO
                    : RawHtml(format!("drawProductBond(pctx{id}, {}, {min_x}, {}, {}, {}/6, shadow);\n", match ty {
                        BondType::Normal => Cow::Borrowed("false, false, false"),
                        BondType::Triplex { red, black, yellow } => Cow::Owned(format!("{red}, {black}, {yellow}")),
                    }, start.q, start.r, match end - start {
                        HexIndex { q: 1, r: 0 } => 0,
                        HexIndex { q: 0, r: 1 } => 1,
                        HexIndex { q: -1, r: 1 } => 2,
                        HexIndex { q: -1, r: 0 } => 3,
                        HexIndex { q: 0, r: -1 } => 4,
                        HexIndex { q: 1, r: -1 } => 5,
                        _ => unimplemented!("quantum bond"),
                    }));
                }
                : RawHtml("if (!shadow) {\n");
                @for (coords, atom) in &atoms {
                    : RawHtml(format!("drawProductAtom(pctx{id}, '{}', {min_x}, {}, {}, 2);\n", format_atom(*atom), coords.q, coords.r));
                }
                : RawHtml("}\n");
                @for (coords, atom) in atoms {
                    : RawHtml(format!("drawProductAtom(pctx{id}, '{}', {min_x}, {}, {}, shadow);\n", format_atom(atom), coords.q, coords.r));
                }
                : RawHtml("}\n");
            }
        }
    }
}

fn parse_atom(s: &str) -> Option<Atom> {
    match s {
        "Salt" | "salt" => Some(Atom::Salt),
        "Air" | "air" => Some(Atom::Air),
        "Earth" | "earth" => Some(Atom::Earth),
        "Fire" | "fire" => Some(Atom::Fire),
        "Water" | "water" => Some(Atom::Water),
        "Quicksilver" | "quicksilver" => Some(Atom::Quicksilver),
        "Gold" | "gold" => Some(Atom::Gold),
        "Silver" | "silver" => Some(Atom::Silver),
        "Copper" | "copper" => Some(Atom::Copper),
        "Iron" | "iron" => Some(Atom::Iron),
        "Tin" | "tin" => Some(Atom::Tin),
        "Lead" | "lead" => Some(Atom::Lead),
        "Vitae" | "vitae" => Some(Atom::Vitae),
        "Mors" | "mors" => Some(Atom::Mors),
        "Repeat" | "repeat" => Some(Atom::Repeat),
        "Quintessence" | "quintessence" => Some(Atom::Quintessence),
        _ => None,
    }
}

fn format_atom(atom: Atom) -> &'static str {
    match atom {
        Atom::Salt => "salt",
        Atom::Air => "air",
        Atom::Earth => "earth",
        Atom::Fire => "fire",
        Atom::Water => "water",
        Atom::Quicksilver => "quicksilver",
        Atom::Gold => "gold",
        Atom::Silver => "silver",
        Atom::Copper => "copper",
        Atom::Iron => "iron",
        Atom::Tin => "tin",
        Atom::Lead => "lead",
        Atom::Vitae => "vitae",
        Atom::Mors => "mors",
        Atom::Repeat => "repeat",
        Atom::Quintessence => "quintessence",
    }
}

#[rocket::async_trait]
impl<'v> form::FromFormField<'v> for FormMolecule {
    fn from_value(field: form::ValueField<'v>) -> form::Result<'v, Self> {
        Ok(Self::read_sync(&mut &*BASE64.decode(field.value).map_err(|e| form::Error::validation(e.to_string()))?).map_err(|e| form::Error::validation(e.to_string()))?)
    }

    async fn from_data(field: form::DataField<'v, '_>) -> form::Result<'v, Self> {
        let limit = field.request.limits()
            .get("molecule")
            .unwrap_or(256.kibibytes());
        let bytes = field.data.open(limit).into_bytes().await?;
        if !bytes.is_complete() {
            Err((None, Some(limit)))?;
        }
        Ok(Self::read_sync(&mut &*bytes.into_inner()).map_err(|e| form::Error::validation(e.to_string()))?)
    }

    fn default() -> Option<Self> {
        Some(Self(Molecule { atoms: HashMap::default(), bonds: HashSet::default() }))
    }
}

impl UriDisplay<Query> for FormMolecule {
    fn fmt(&self, f: &mut Formatter<'_, Query>) -> fmt::Result {
        let mut buf = Vec::default();
        self.write_sync(&mut buf).map_err(|_| fmt::Error)?;
        f.write_raw(BASE64.encode(buf))
    }
}

impl_from_uri_param_identity!([Query] FormMolecule);

fn footer() -> RawHtml<String> { //TODO make this a const (requires const_html macro)
    html! {
        footer(class = "muted") {
            p {
                : "hosted by ";
                a(href = "https://fenhl.net/") : "Fenhl";
                : " • ";
                a(href = "https://fenhl.net/disc") : "disclaimer";
                : " • ";
                a(href = "https://status.fenhl.net/") : "status";
                : " • ";
                a(href = "https://github.com/fenhl/molecule-db") : "source code";
            }
            p {
                : "Special thanks to panic whose ";
                a(href = "http://critelli.technology/transmogrification.html") : "Tonic of Transmogrification reagent builder";
                : " served as the basis for parts of this website's code!";
            }
        }
    }
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
enum IndexError {
    #[error(transparent)] Json(#[from] serde_json::Error),
}

#[rocket::get("/?<m>&<b>")]
fn index(m: Option<FormMolecule>, b: Option<NonZero<u8>>) -> Result<RawHtml<String>, IndexError> {
    let (js_state, min_radius, radius, molecule_too_large) = if let Some(FormMolecule(molecule)) = m.clone() {
        match JsState::new(molecule, b) {
            Ok((js_state, min_radius)) => (Some(js_state), min_radius, b.unwrap_or_else(|| min_radius.max(NonZero::new(5).unwrap())), false),
            Err(MoleculeTooLarge { radius, min_radius }) => (None, min_radius, radius, true),
        }
    } else {
        (None, NonZero::<u8>::MIN, b.unwrap_or_else(|| NonZero::new(5).unwrap()), false)
    };
    Ok(html! {
        : Doctype;
        html {
            head {
                meta(charset = "utf-8");
                title : "Opus Magnum Molecule Database";
                meta(name = "viewport", content = "width=device-width, initial-scale=1, shrink-to-fit=no");
                link(rel = "icon", href = static_url!("favicon.svg"));
                link(rel = "stylesheet", href = static_url!("common.css"));
                script(src = static_url!("common.js"));
            }
            body {
                nav {
                    a(class = "button selected") : "Molecules by shape";
                    a(class = "button", href = uri!(molecules_list)) : "Molecules by name";
                }
                main(style = "flex-direction: column;") {
                    @if molecule_too_large {
                        div(class = "emphasized-section") : "molecule does not fit onto canvas";
                    }
                    h2 : "ENTER MOLECULE TO LOOK UP";
                    div(id = "canvas-wrapper") {
                        canvas(id = "current");
                        div(id = "clear", class = "canvas-button", style = "display: none;") {
                            a(href = uri!(index(_, if radius.get() == 5 { None } else { Some(radius) }))) : "Clear";
                        }
                        div(id = "radius", class = "canvas-button") {
                            a(id = "radius-down", href = uri!(index(m.clone(), radius.get().checked_sub(1).and_then(NonZero::new).filter(|new_radius| *new_radius != min_radius.max(NonZero::new(5).unwrap())))), style? = radius.get().checked_sub(1).and_then(NonZero::new).is_none_or(|new_radius| new_radius < min_radius).then_some("display: none;")) : "−";
                            : " B=";
                            : radius;
                            : " ";
                            a(id = "radius-up", href = uri!(index(m, radius.checked_add(1).filter(|new_radius| *new_radius != min_radius.max(NonZero::new(5).unwrap())))), style? = radius.checked_add(1).is_none().then_some("display: none;")) : "+";
                        }
                        div(id = "permalink", class = "canvas-button", style = "display: none;") {
                            a : "Copy Permalink";
                        }
                    }
                    div(id = "result", style = "display: none;");
                    p(id = "error");
                }
                canvas(id = "next", style = "display: none;");
                : footer();
                script : RawHtml(format!("const radius = {radius};"));
                script(src = static_url!("transmogrification.js"));
                @if let Some(js_state) = js_state {
                    script : RawHtml(format!("
                        async function updateFromQuery() {{
                            state = {0};
                            nextState = state;
                            redraw();
                            await updateDownload();
                        }}

                        updateFromQuery();
                    ", serde_json::to_value(js_state)?));
                }
            }
        }
    })
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsState {
    #[allow(unused)] selected_atom: Option<String>,
    #[allow(unused)] selected_bond: Option<String>,
    #[serde(flatten)]
    rest: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
#[error("molecule does not fit onto a canvas of size {radius}; minimum size is {min_radius}")]
struct MoleculeTooLarge {
    radius: NonZero<u8>,
    min_radius: NonZero<u8>,
}

impl JsState {
    fn new(molecule: Molecule, force_radius: Option<NonZero<u8>>) -> Result<(Self, NonZero<u8>), MoleculeTooLarge> {
        let mut molecule = molecule.normalized();
        let mut min_radius = NonZero::<u8>::MIN;
        if !molecule.atoms.is_empty() {
            // move molecule to try to fit onto canvas
            for rotation in [HexRotation::R0, HexRotation::R60, HexRotation::R120] {
                molecule = molecule.rotated(HexIndex::default(), rotation);
                let (min, max) = molecule.atoms.keys().minmax_by_key(|HexIndex { q, r }| q + r).into_option().expect("molecule has no atoms, checked above");
                let min_offset = min.q + min.r;
                let max_offset = max.q + max.r;
                let dir_min_radius = NonZero::new((max_offset + 2 - min_offset).div_ceil(2).try_into().map_err(|_| MoleculeTooLarge { radius: NonZero::<u8>::MAX, min_radius: NonZero::<u8>::MAX })?).expect("max_offset should always be ≥ min_offset so the division result should always be ≥ 1");
                min_radius = min_radius.max(dir_min_radius);
                let center_offset = (max_offset + min_offset) / 2;
                molecule = molecule.translated(HexIndex { q: -center_offset.div_euclid(2), r: -center_offset.div_ceil(2) });
                molecule = molecule.rotated(HexIndex::default(), (-i16::from(rotation.turns())).into());
            }
        }
        if let Some(radius) = force_radius && min_radius > radius {
            return Err(MoleculeTooLarge { radius, min_radius })
        }
        let Molecule { atoms, bonds } = molecule;
        Ok((Self {
            selected_atom: Some(format!("salt")),
            selected_bond: Some(format!("n")),
            rest: atoms.into_iter()
                .map(|(pos, atom)| (format!("{},{}", pos.q, pos.r), format_atom(atom).to_owned()))
                .chain(bonds.into_iter()
                    .map(|Bond { mut start, mut end, ty }| {
                        if end.q == start.q + 1 && end.r == start.r - 1 {
                            // JS expects this bond direction to be given in the opposite direction compared to MoleculeExt::normalized
                            mem::swap(&mut start, &mut end);
                        }
                        (format!("{},{}:{},{}", start.q, start.r, end.q, end.r), match ty {
                            BondType::Normal => format!("n"),
                            BondType::Triplex { red, black, yellow } => format!("{}{}{}", if red { "r" } else { "" }, if black { "k" } else { "" }, if yellow { "y" } else { "" }),
                        })
                    })
                )
                .collect(),
        }, min_radius))
    }
}

impl TryFrom<JsState> for Molecule {
    type Error = ();

    fn try_from(JsState { rest, .. }: JsState) -> Result<Self, ()> {
        let mut molecule = Self { atoms: HashMap::default(), bonds: HashSet::default() };
        for (key, value) in rest {
            if let Some((start, end)) = key.split_once(':') {
                let (q1, r1) = start.split_once(',').ok_or(())?;
                let (q2, r2) = end.split_once(',').ok_or(())?;
                molecule.bonds.insert(Bond {
                    start: HexIndex { q: q1.parse().map_err(|_| ())?, r: r1.parse().map_err(|_| ())? },
                    end: HexIndex { q: q2.parse().map_err(|_| ())?, r: r2.parse().map_err(|_| ())? },
                    ty: if value == "n" {
                        BondType::Normal
                    } else {
                        BondType::Triplex { red: value.contains('r'), black: value.contains('k'), yellow: value.contains('y') }
                    },
                });
            } else {
                let (q, r) = key.split_once(',').ok_or(())?;
                molecule.atoms.insert(HexIndex { q: q.parse().map_err(|_| ())?, r: r.parse().map_err(|_| ())? }, parse_atom(&value).ok_or(())?);
            }
        }
        Ok(molecule.normalized())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoleculeResponseV1 {
    appearances: Vec<(String, InOut, &'static str)>,
    permalink: String,
    rust_code: String,
}

#[rocket::post("/api/v1/molecule-from-state", format = "json", data = "<state>")]
fn molecule_from_state_v1(state: Json<JsState>) -> Result<Json<MoleculeResponseV1>, Status> {
    let molecule = Molecule::try_from(state.0).map_err(|()| Status::BadRequest)?;
    let mut permalink = Vec::default();
    FormMolecule(molecule.clone()).write_sync(&mut permalink).map_err(|_| Status::BadRequest)?;
    let mut response = MoleculeResponseV1 {
        appearances: Vec::default(),
        permalink: BASE64.encode(permalink),
        rust_code: format!("{:?}", Unparse(&molecule)),
    };
    for (iter_molecule, appearances) in molecules::molecules() {
        if iter_molecule == molecule {
            response.appearances = appearances.into_iter()
                .filter_map(|(puzzle, inout, name)| Some((puzzle, inout, name?)))
                .map(|(puzzle, inout, name)| (format!("{puzzle}{}", if puzzle.is_custom() { "*" } else { "" }), inout, name))
                .collect();
            break
        }
    }
    Ok(Json(response))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoleculeResponseV2 {
    appearances: Vec<Appearance>,
    permalink: String,
    rust_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Appearance {
    puzzle: &'static str,
    url: Option<Url>,
    inout: InOut,
    name: Option<&'static str>,
}

#[rocket::post("/api/v2/molecule-from-state", format = "json", data = "<state>")]
fn molecule_from_state_v2(state: Json<JsState>) -> Result<Json<MoleculeResponseV2>, Status> {
    let molecule = Molecule::try_from(state.0).map_err(|()| Status::BadRequest)?;
    let mut permalink = Vec::default();
    FormMolecule(molecule.clone()).write_sync(&mut permalink).map_err(|_| Status::BadRequest)?;
    let mut response = MoleculeResponseV2 {
        appearances: Vec::default(),
        permalink: BASE64.encode(permalink),
        rust_code: format!("{:?}", Unparse(&molecule)),
    };
    for (iter_molecule, appearances) in molecules::molecules() {
        if iter_molecule == molecule {
            response.appearances = appearances.into_iter()
                .filter_map(|(puzzle, inout, name)| Some((puzzle, inout, name?)))
                .map(|(puzzle, inout, name)| Appearance {
                    puzzle: puzzle.as_str(),
                    url: puzzle.source().url(),
                    name: Some(name),
                    inout,
                })
                .collect();
            break
        }
    }
    Ok(Json(response))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoleculeResponseV3 {
    appearances: Vec<Appearance>,
    min_radius: NonZero<u8>,
    permalink: String,
    rust_code: String,
}

#[rocket::post("/api/v3/molecule-from-state", format = "json", data = "<state>")]
fn molecule_from_state_v3(state: Json<JsState>) -> Result<Json<MoleculeResponseV3>, Status> {
    let molecule = Molecule::try_from(state.0).map_err(|()| Status::BadRequest)?;
    let mut permalink = Vec::default();
    FormMolecule(molecule.clone()).write_sync(&mut permalink).map_err(|_| Status::BadRequest)?;
    let mut response = MoleculeResponseV3 {
        appearances: Vec::default(),
        min_radius: molecule.min_radius().map_err(|_| Status::BadRequest)?,
        permalink: BASE64.encode(permalink),
        rust_code: format!("{:?}", Unparse(&molecule)),
    };
    for (iter_molecule, appearances) in molecules::molecules() {
        if iter_molecule == molecule {
            response.appearances = appearances.into_iter().map(|(puzzle, inout, name)| Appearance {
                puzzle: puzzle.as_str(),
                url: puzzle.source().url(),
                inout, name,
            }).collect();
            break
        }
    }
    Ok(Json(response))
}

#[rocket::get("/molecules")]
fn molecules_list() -> RawHtml<String> {
    html! {
        : Doctype;
        html {
            head {
                meta(charset = "utf-8");
                title : "Opus Magnum Molecule Database";
                meta(name = "viewport", content = "width=device-width, initial-scale=1, shrink-to-fit=no");
                link(rel = "icon", href = static_url!("favicon.svg"));
                link(rel = "stylesheet", href = static_url!("common.css"));
                script(src = static_url!("common.js"));
            }
            body {
                nav {
                    a(class = "button", href = uri!(index(_, _))) : "Molecules by shape";
                    a(class = "button selected") : "Molecules by name";
                }
                main {
                    @for (idx, (molecule, appearances)) in molecules::molecules().into_iter().sorted_by_key(|(_, appearances)| {
                        let mut names = appearances.iter().filter_map(|(_, _, name)| *name).collect_vec();
                        names.sort_unstable();
                        names.dedup();
                        (names.is_empty(), names)
                    }).enumerate() {
                        div {
                            h2 {
                                @if appearances.iter().all(|(_, _, name)| name.is_none()) {
                                    span(class = "muted") : "unnamed";
                                } else {
                                    : appearances.iter().filter_map(|(_, _, name)| *name).sorted_unstable().dedup().join("/");
                                }
                            }
                            a(href = uri!(index(Some(FormMolecule(molecule.clone())), _))) : molecule.draw(&format!("product{idx}"));
                        }
                    }
                }
                : footer();
            }
        }
    }
}

#[derive(clap::Parser)]
struct Args {
    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    Validate,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] GitCheckout(#[from] gix::clone::checkout::main_worktree::Error),
    #[error(transparent)] GitClone(#[from] gix::clone::Error),
    #[error(transparent)] GitCloneFetch(#[from] gix::clone::fetch::Error),
    #[error(transparent)] GitConnect(#[from] gix::remote::connect::Error),
    #[error(transparent)] GitFetch(#[from] gix::remote::fetch::Error),
    #[error(transparent)] GitFindRemote(#[from] gix::remote::find::existing::Error),
    #[error(transparent)] GitOpen(#[from] gix::open::Error),
    #[error(transparent)] GitPrepareFetch(#[from] gix::remote::fetch::prepare::Error),
    #[error(transparent)] Rocket(#[from] rocket::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    #[error("failed to locate user folder")]
    MissingHomeDir,
    #[error("no default remote configured for zlbb repo")]
    NoDefaultRemote,
    #[error("failed to parse puzzle: {0}")]
    ParsePuzzle(&'static str),
}

#[wheel::main(rocket)]
async fn main(Args { subcommand }: Args) -> Result<(), Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    if let Some(subcommand) = subcommand {
        match subcommand {
            Subcommand::Validate => {
                println!("downloading critelli Gopher index");
                let mut critelli_puzzles = HashMap::new();
                let mut tcp_client = BufReader::new(TcpStream::connect(("events.critelli.technology", 70)).await.at_unknown()?);
                tcp_client.write_all(b"/\r\n").await.at_unknown()?;
                let mut buf = String::default();
                while tcp_client.read_line(&mut buf).await.at_unknown()? > 0 {
                    if let Some(GopherEntry { item_type: gophermap::ItemType::Binary, selector, host, port, .. }) = GopherEntry::from(&buf) {
                        if let Some((_, url_part)) = regex_captures!("^/puzzle/(.+)/.+\\.puzzle$", selector) {
                            critelli_puzzles.insert(url_part.to_owned(), (host.to_owned(), port, selector.to_owned()));
                        }
                    }
                    buf.clear();
                }
                let zlbb_parent = {
                    #[cfg(any(target_os = "windows", target_os = "linux"))] { UserDirs::new().ok_or(Error::MissingHomeDir)?.home_dir().join("git").join("github.com").join("F43nd1r").join("zachtronics-leaderboard-bot") }
                    #[cfg(target_os = "macos")] { PathBuf::from("/opt/git/github.com/F43nd1r/zachtronics-leaderboard-bot") }
                };
                let zlbb_path = zlbb_parent.join("main");
                if fs::exists(&zlbb_path).await? {
                    println!("updating zlbb repo");
                    let repo = gix::open(&zlbb_path)?;
                    repo.find_default_remote(gix::remote::Direction::Fetch).ok_or(Error::NoDefaultRemote)??
                        .connect(gix::remote::Direction::Fetch)?
                        .prepare_fetch(gix::progress::Discard /*TODO show progress on command line? */, Default::default())?
                        .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(NonZero::<u32>::MIN))
                        .receive(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?;
                    Command::new("git").arg("reset").arg("--hard").arg("origin/HEAD").current_dir(&zlbb_path).check("git reset").await?; //TODO use gix, blocked on https://github.com/GitoxideLabs/gitoxide/issues/301
                } else {
                    println!("cloning zlbb repo");
                    fs::create_dir_all(zlbb_parent).await?;
                    gix::prepare_clone("https://github.com/F43nd1r/zachtronics-leaderboard-bot.git", &zlbb_path)?
                        .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(NonZero::<u32>::MIN))
                        .fetch_then_checkout(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?.0
                        .main_worktree(gix::progress::Discard /*TODO show progress on command line? */, &gix::interrupt::IS_INTERRUPTED)?;
                }
                wheel::print_flush!("validating puzzles")?;
                for puzzle in all::<Puzzle>() {
                    let omsim_rs::data::Puzzle { reagents, products, .. } = parse_puzzle(&match puzzle.source() {
                        puzzle::Source::Tutorial | puzzle::Source::Computation { .. } | puzzle::Source::CritelliComputation { .. } => continue, // nothing to validate against
                        puzzle::Source::Critelli { url_part } => {
                            let (host, port, selector) = critelli_puzzles.remove(url_part).expect(&format!("missing critelli puzzle: {url_part}"));
                            let mut tcp_client = TcpStream::connect((host, port)).await.at_unknown()?;
                            tcp_client.write_all(selector.as_ref()).await.at_unknown()?;
                            tcp_client.write_all(b"\r\n").await.at_unknown()?;
                            let mut buf = Vec::default();
                            tcp_client.read_to_end(&mut buf).await.at_unknown()?;
                            buf
                        }
                        puzzle::Source::CritelliPrivate { url_part, file_stem } => {
                            let mut tcp_client = TcpStream::connect(("events.critelli.technology", 70)).await.at_unknown()?;
                            tcp_client.write_all(b"/puzzle/").await.at_unknown()?;
                            tcp_client.write_all(url_part.as_ref()).await.at_unknown()?;
                            tcp_client.write_all(b"/").await.at_unknown()?;
                            tcp_client.write_all(file_stem.as_ref()).await.at_unknown()?;
                            tcp_client.write_all(b".puzzle\r\n").await.at_unknown()?;
                            let mut buf = Vec::default();
                            tcp_client.read_to_end(&mut buf).await.at_unknown()?;
                            buf
                        }
                        puzzle::Source::Official { zlbb_id } | puzzle::Source::Zlbb { zlbb_id, .. } => fs::read(zlbb_path.join(format!("src/main/resources/om/puzzle/{zlbb_id}.puzzle"))).await?,
                        puzzle::Source::Other { .. } => fs::read(format!("assets/puzzle/{}.puzzle", puzzle.url_part())).await?,
                    }).map_err(|e| Error::ParsePuzzle(e))?;
                    let mut found_reagents = Vec::default();
                    let mut found_products = Vec::default();
                    for (molecule, puzzles) in molecules::molecules() {
                        if let Some((_, _, name)) = puzzles.iter().find(|(iter_puzzle, in_out, _)| *iter_puzzle == puzzle && in_out.is_reagent()) {
                            assert!(reagents.iter().any(|iter_molecule| iter_molecule.normalized() == molecule), "{} ({molecule:?}) not found in upstream version of {puzzle}", if let Some(name) = name { format!("reagent {name:?}") } else { format!("unnamed reagent") });
                            if !found_reagents.iter().any(|iter_molecule| *iter_molecule == molecule) {
                                found_reagents.push(molecule.clone());
                            }
                        }
                        if let Some((_, _, name)) = puzzles.iter().find(|(iter_puzzle, in_out, _)| *iter_puzzle == puzzle && in_out.is_product()) {
                            assert!(products.iter().any(|iter_molecule| iter_molecule.normalized() == molecule), "{} ({molecule:?}) not found in upstream version of {puzzle}", if let Some(name) = name { format!("product {name:?}") } else { format!("unnamed product") });
                            if !found_products.iter().any(|iter_molecule| *iter_molecule == molecule) {
                                found_products.push(molecule);
                            }
                        }
                    }
                    for molecule in reagents {
                        assert!(found_reagents.contains(&molecule.normalized()), "upstream version of {puzzle} has an additional reagent: {molecule:?}");
                    }
                    for molecule in products {
                        assert!(found_products.contains(&molecule.normalized()), "upstream version of {puzzle} has an additional product: {molecule:?}");
                    }
                    wheel::print_flush!(".")?;
                }
                println!("\nvalid");
            }
        }
    } else {
        rocket::custom(rocket::Config {
            port: 24821,
            ..rocket::Config::default()
        })
        .mount("/", rocket::routes![
            index,
            molecule_from_state_v1,
            molecule_from_state_v2,
            molecule_from_state_v3,
            molecules_list,
        ])
        .mount("/static", FileServer::new("assets/static", rocket::fs::Options::None))
        .launch().await?;
    }
    Ok(())
}
