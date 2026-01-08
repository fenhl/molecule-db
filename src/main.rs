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
    },
    async_proto::Protocol,
    base64::engine::{
        Engine as _,
        general_purpose::URL_SAFE as BASE64,
    },
    enum_iterator::all,
    itertools::Itertools as _,
    omsim_rs::data::*,
    rocket::{
        data::ToByteUnit as _,
        form::{
            self,
            FromFormField,
        },
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
    crate::{
        proto::FormMolecule,
        unparse::Unparse,
        util::{
            IntExt as _,
            IteratorExt as _,
        },
    },
};

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

trait MoleculeExt {
    fn position_normalized(&self) -> Self;
    fn normalized(&self) -> Self;
    fn mirrored(&self) -> Self;
    fn draw(&self, id: &str) -> RawHtml<String>;
}

impl MoleculeExt for Molecule {
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
impl<'v> FromFormField<'v> for FormMolecule {
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

#[rocket::get("/?<m>")]
fn index(m: Option<FormMolecule>) -> Result<RawHtml<String>, IndexError> {
    let (js_state, molecule_too_large) = if let Some(FormMolecule(molecule)) = m {
        match JsState::try_from(molecule) {
            Ok(js_state) => (Some(js_state), false),
            Err(MoleculeTooLarge) => (None, true),
        }
    } else {
        (None, false)
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
                main(style = "flex-direction: column;") {
                    @if molecule_too_large {
                        div(class = "emphasized-section") : "molecule does not fit onto canvas";
                    }
                    div {
                        h2 : "ENTER MOLECULE TO LOOK UP";
                        div(id = "canvas-wrapper") {
                            canvas(id = "current");
                            div(id = "clear", class = "canvas-button", style = "display: none;") {
                                a(href = uri!(index(_))) : "Clear";
                            }
                            div(id = "permalink", class = "canvas-button", style = "display: none;") {
                                a : "Copy Permalink";
                            }
                        }
                    }
                    div(id = "result", style = "display: none;");
                    p(id = "error");
                    p(id = "default") {
                        a(href = uri!(molecules_list)) : "List of all molecules";
                    }
                }
                canvas(id = "next", style = "display: none;");
                : footer();
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
#[error("molecule does not fit onto canvas")]
struct MoleculeTooLarge;

impl TryFrom<Molecule> for JsState {
    type Error = MoleculeTooLarge;

    fn try_from(molecule: Molecule) -> Result<Self, Self::Error> {
        let mut molecule = molecule.normalized();
        if !molecule.atoms.is_empty() {
            // move molecule to try to fit onto canvas
            for rotation in [HexRotation::R0, HexRotation::R60, HexRotation::R120] {
                molecule = molecule.rotated(HexIndex::default(), rotation);
                let (min, max) = molecule.atoms.keys().minmax_by_key(|HexIndex { q, r }| q + r).into_option().expect("molecule has no atoms, checked above");
                let min_offset = min.q + min.r;
                let max_offset = max.q + max.r;
                if max_offset - min_offset > 8 {
                    return Err(MoleculeTooLarge)
                }
                let center_offset = (max_offset + min_offset) / 2;
                molecule = molecule.translated(HexIndex { q: -center_offset.div_euclid(2), r: -center_offset.div_ceil(2) });
                molecule = molecule.rotated(HexIndex::default(), (-i16::from(rotation.turns())).into());
            }
        }
        let Molecule { atoms, bonds } = molecule;
        Ok(Self {
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
        })
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
    url: Option<&'static str>,
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
                    url: puzzle.url(),
                    name: Some(name),
                    inout,
                })
                .collect();
            break
        }
    }
    Ok(Json(response))
}

#[rocket::post("/api/v3/molecule-from-state", format = "json", data = "<state>")]
fn molecule_from_state_v3(state: Json<JsState>) -> Result<Json<MoleculeResponseV2>, Status> {
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
            response.appearances = appearances.into_iter().map(|(puzzle, inout, name)| Appearance {
                puzzle: puzzle.as_str(),
                url: puzzle.url(),
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
                            a(href = uri!(index(Some(FormMolecule(molecule.clone()))))) : molecule.draw(&format!("product{idx}"));
                        }
                    }
                }
                : footer();
            }
        }
    }
}

#[rocket::launch]
fn rocket() -> _ {
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
}
