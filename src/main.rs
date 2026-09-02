#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
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
        time::Duration,
    },
    async_proto::Protocol,
    base64::engine::{
        Engine as _,
        general_purpose::URL_SAFE_NO_PAD as BASE64,
    },
    enum_iterator::{
        Sequence,
        all,
    },
    gophermap::GopherEntry,
    itertools::Itertools as _,
    lazy_regex::regex_captures,
    omsim_rs::{
        data::*,
        parse::parse_puzzle,
    },
    rocket::{
        Responder,
        State,
        data::ToByteUnit as _,
        form,
        fs::FileServer,
        http::{
            Header,
            Status,
            impl_from_uri_param_identity,
            uri::fmt::{
                Formatter,
                Query,
                UriDisplay,
            },
        },
        outcome::Outcome,
        request::{
            self,
            FromRequest,
            Request,
        },
        response::content::RawHtml,
        serde::json::Json,
        uri,
    },
    rocket_util::{
        Doctype,
        ToHtml,
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
    xdg::BaseDirectories,
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
#[cfg(feature = "night")] use {
    std::iter,
    wheel::traits::ReqwestResponseExt as _,
};

include!(concat!(env!("OUT_DIR"), "/static_files.rs"));
include!(concat!(env!("OUT_DIR"), "/version.rs"));

mod metric;
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
    fn new(i: u8, o: u8) -> Self {
        match (i == 0, o == 0) {
            (false, false) => Self::Both,
            (false, true) => Self::Reagent,
            (true, false) => Self::Product,
            (true, true) => panic!(),
        }
    }
}

trait MoleculeExt {
    fn min_radius(&self) -> Result<NonZero<u8>, MoleculeTooLarge>;
    fn position_normalized(&self) -> Self;
    fn normalized(&self) -> Self;
    fn mirrored(&self) -> Self;
    fn draw_params(&self) -> String;
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

    fn draw_params(&self) -> String {
        let Self { atoms, bonds } = self.mirrored();
        let min_x = atoms.keys().map(|&HexIndex { q, r }| 2 * q + r).min().unwrap_or_default();
        let width = atoms.keys().map(|&HexIndex { q, r }| 2 * q + r + 2).max().unwrap_or_default() - min_x;
        let height = atoms.keys().map(|&HexIndex { r, .. }| r + 1).max().unwrap_or_default();
        let width = (41 * width + 10) * 3 / 4;
        let height = (71 * height + 20) * 3 / 4;
        format!("{min_x}, {width}, {height}, [{}], [{}]",
            atoms.into_iter().map(|(coords, kind)| format!("{{kind: {:?}, q: {}, r: {}}}", format_atom(kind), coords.q, coords.r)).join(", "),
            bonds.into_iter().map(|Bond { start, end, ty }| format!(
                "{{start: {{q: {}, r: {}}}, end: {{q: {}, r: {}}}, red: {}, black: {}, yellow: {}}}",
                start.q,
                start.r,
                end.q,
                end.r,
                match ty { BondType::Normal => false, BondType::Triplex { red, .. } => red },
                match ty { BondType::Normal => false, BondType::Triplex { black, .. } => black },
                match ty { BondType::Normal => false, BondType::Triplex { yellow, .. } => yellow },
            )).join(", "),
        )
    }

    fn draw(&self, id: &str) -> RawHtml<String> {
        html! {
            canvas(class = "molecule-canvas", id = id);
            script : RawHtml(format!("drawProduct({id:?}, {});", self.draw_params()));
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
        Ok(Self::read_sync(&mut &*BASE64.decode(field.value).or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(field.value)).map_err(|e| form::Error::validation(e.to_string()))?).map_err(|e| form::Error::validation(e.to_string()))?)
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

#[derive(Clone, Deserialize)]
#[cfg_attr(not(feature = "night"), derive(Default))]
struct Config {
    #[cfg(feature = "night")]
    night_password: String,
}

impl Config {
    async fn load() -> Result<Self, Error> {
        if let Some(config) = BaseDirectories::new().find_config_file("fenhl/molecule-db.json") {
            Ok(fs::read_json(config).await?)
        } else {
            #[cfg(not(feature = "night"))] { Ok(Self::default()) }
            #[cfg(feature = "night")] { Err(Error::MissingConfig) }
        }
    }
}

#[cfg(feature = "night")]
async fn night_report(config: &Config, http_client: &reqwest::Client, path: &str, extra: Option<&str>) -> Result<(), Error> {
    http_client
        .post("https://night.fenhl.net/dev/dushanbe/report")
        .bearer_auth(&config.night_password)
        .form(&iter::once(("path", path)).chain(extra.map(|extra| ("extra", extra))).collect_vec())
        .send().await?
        .detailed_error_for_status().await?;
    Ok(())
}

#[cfg(feature = "night")]
fn night_report_sync(config: &Config, path: &str, extra: Option<&str>) -> Result<(), Error> {
    reqwest::blocking::Client::new()
        .post("https://night.fenhl.net/dev/dushanbe/report")
        .bearer_auth(&config.night_password)
        .form(&iter::once(("path", path)).chain(extra.map(|extra| ("extra", extra))).collect_vec())
        .send()?
        .error_for_status()?;
    Ok(())
}

async fn external_link(#[cfg_attr(not(feature = "night"), allow(unused))] config: &Config, #[cfg_attr(not(feature = "night"), allow(unused))] http_client: &reqwest::Client, url: &str, display: impl ToHtml) -> Result<RawHtml<String>, Error> {
    let url = Url::parse(url)?;
    Ok(html! {
        a(href = url) {
            @match url.host_str() {
                Some("discord.com") => img(class = "favicon", alt = "external link (discord.com)", src = static_url!("discord-favicon.ico"));
                Some("github.com") => picture(class = "favicon") {
                    source(srcset = "https://github.githubassets.com/favicons/favicon.svg", media = "(prefers-color-scheme: light)");
                    img(alt = "external link (github.com)", src = "https://github.githubassets.com/favicons/favicon-dark.svg");
                }
                Some("drive.google.com") => img(class = "favicon", alt = "external link (drive.google.com)", src = "https://www.gstatic.com/images/branding/productlogos/drive_2026/v1/web-32dp/logo_drive_2026_color_1x_web_32dp.png");
                Some("reddit.com") => img(class = "favicon", alt = "external link (reddit.com)", srcset = "https://www.redditstatic.com/shreddit/assets/favicon/64x64.png 64w, https://www.redditstatic.com/shreddit/assets/favicon/128x128.png 128w, https://www.redditstatic.com/shreddit/assets/favicon/192x192.png 192w");
                Some("fenhl.net" | "status.fenhl.net") => img(class = "favicon", alt = "external link (fenhl.net)", srcset = "https://fenhl.net/static/ava/pineapple/p-sq-16.png 16w, https://fenhl.net/static/ava/pineapple/p-sq-32.png 32w, https://fenhl.net/static/ava/pineapple/p-sq-64.png 64w, https://fenhl.net/static/ava/pineapple/p-sq-128.png 128w, https://fenhl.net/static/ava/pineapple/p-sq-256.png 256w");
                Some("critelli.technology") => svg(class = "favicon", xmlns = "http://www.w3.org/2000/svg", viewBox = "0 0 24 24") {
                    path(style = "fill: light-dark(black, white)", d = "M21.658 3.786l-3.658 3.318v-1.104c0-3.313-2.687-6-6-6s-6 2.687-6 6v4h-3v10.707l-2 1.813 1.346 1.48 20.654-18.734-1.342-1.48zm-5.658 5.132l-1.194 1.082h-6.806v-4c0-2.205 1.795-4 4-4s4 1.795 4 4v2.918zm5 1.082v14h-16.391l15.422-14h.969z");
                }
                Some("events.critelli.technology") => img(class = "favicon", alt = "external link (events.critelli.technology)", src = "https://events.critelli.technology/favicon.ico");
                Some(host) => {
                    @cfg(feature = "night") {
                        @let () = night_report(config, http_client, "/dev/dushanbe/mol/faviconError", Some(&format!("no favicon defined for host {host:?}"))).await?;
                    }
                    span(class = "favicon") : "🌐";
                }
                None => {
                    @cfg(feature = "night") {
                        @let () = night_report(config, http_client, "/dev/dushanbe/mol/faviconError", Some(&format!("URL {url:?} has no host"))).await?;
                    }
                    span(class = "favicon") : "🌐";
                }
            }
            : display;
        }
    })
}

#[derive(PartialEq, Eq, Sequence)]
enum Tab {
    MoleculeInput,
    MoleculeList,
    Puzzles,
}

impl Tab {
    fn uri(&self) -> rocket::http::uri::Origin<'static> {
        match self {
            Self::MoleculeInput => uri!(index(_, _)),
            Self::MoleculeList => uri!(molecules_list),
            Self::Puzzles => uri!(puzzle::index()),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::MoleculeInput => "Molecule input",
            Self::MoleculeList => "Molecule list",
            Self::Puzzles => "Puzzles",
        }
    }
}

struct Etag<'a> {
    #[allow(unused)]
    weak: bool,
    content: &'a str,
}

impl<'a> From<&'a str> for Etag<'a> {
    fn from(content: &'a str) -> Self {
        Self {
            weak: false,
            content,
        }
    }
}

enum IfNoneMatch<'a> {
    Any,
    Specific(Vec<Etag<'a>>),
}

impl IfNoneMatch<'_> {
    fn matches<'b>(&self, etag: impl Into<Etag<'b>>) -> bool {
        let etag = etag.into();
        match self {
            Self::Any => true,
            Self::Specific(etags) => etags.iter().any(|iter_etag| etag.content == iter_etag.content), // A recipient MUST use the weak comparison function when comparing entity tags for If-None-Match (https://httpwg.org/specs/rfc9110.html#rfc.section.13.1.2)
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum IfNoneMatchError {
    #[error("received both wildcard and specific If-None-Match values")]
    AnyAndSpecific,
    #[error("unexpected end of If-None-Match header")]
    EarlyEnd,
    #[error("unexpected character in If-None-Match header")]
    Parse(char),
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IfNoneMatch<'r> {
    type Error = IfNoneMatchError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let mut any = false;
        let mut buf = Vec::default();
        for mut rest in req.headers().get(http::header::IF_NONE_MATCH.as_str()) {
            if rest == "*" {
                any = true;
            } else {
                while let Some(c) = rest.chars().next() {
                    match c {
                        'W' => {
                            let mut chars = rest.chars().skip(1);
                            match chars.next() {
                                None => return Outcome::Error((Status::BadRequest, IfNoneMatchError::EarlyEnd)),
                                Some('/') => {}
                                Some(c) => return Outcome::Error((Status::BadRequest, IfNoneMatchError::Parse(c)))
                            }
                            match chars.next() {
                                None => return Outcome::Error((Status::BadRequest, IfNoneMatchError::EarlyEnd)),
                                Some('"') => {}
                                Some(c) => return Outcome::Error((Status::BadRequest, IfNoneMatchError::Parse(c)))
                            }
                            let Some((content, new_rest)) = rest[3..].split_once('"') else { return Outcome::Error((Status::BadRequest, IfNoneMatchError::EarlyEnd)) };
                            buf.push(Etag { weak: true, content });
                            rest = new_rest;
                        }
                        '"' => {
                            let Some((content, new_rest)) = rest[1..].split_once('"') else { return Outcome::Error((Status::BadRequest, IfNoneMatchError::EarlyEnd)) };
                            buf.push(Etag { weak: true, content });
                            rest = new_rest;
                        }
                        ' ' | '\t' | ',' => rest = &rest[1..],
                        _ => return Outcome::Error((Status::BadRequest, IfNoneMatchError::Parse(c))),
                    }
                }
            }
        }
        match (any, buf.is_empty()) {
            (false, _) => Outcome::Success(Self::Specific(buf)),
            (true, false) => Outcome::Success(Self::Any),
            (true, true) => Outcome::Error((Status::BadRequest, IfNoneMatchError::AnyAndSpecific)),
        }
    }
}

#[derive(Responder)]
enum StaticPageResponse {
    Fresh((Status, ())),
    Stale {
        body: RawHtml<String>,
        cache_control: Header<'static>,
        etag: Header<'static>,
    },
    Untagged(RawHtml<String>),
}

async fn static_page(config: &Config, http_client: &reqwest::Client, if_none_match: IfNoneMatch<'_>, tab: Tab, is_subpage: bool, title: impl ToHtml, content: impl ToHtml, scripts: impl ToHtml) -> StaticPageResponse {
    if GIT_COMMIT_HASH.is_some_and(|git_commit_hash| if_none_match.matches(&*git_commit_hash.to_string())) {
        StaticPageResponse::Fresh((Status::NotModified, ()))
    } else {
        let body = html! {
            : Doctype;
            html {
                head {
                    meta(charset = "utf-8");
                    title : title;
                    meta(name = "viewport", content = "width=device-width, initial-scale=1, shrink-to-fit=no");
                    link(rel = "icon", href = static_url!("favicon.svg"));
                    link(rel = "stylesheet", href = static_url!("common.css"));
                    script(src = static_url!("common.js"));
                }
                body {
                    nav {
                        @for iter_tab in all::<Tab>() {
                            a(class = if tab == iter_tab { "button selected" } else { "button" }, href? = (tab != iter_tab || is_subpage).then(|| iter_tab.uri())) : iter_tab.label();
                        }
                    }
                    : content;
                    footer(class = "muted") {
                        hr;
                        p {
                            : "Opus Magnum molecule database hosted by ";
                            : external_link(config, http_client, "https://fenhl.net/", "Fenhl").await.unwrap();
                            : " • ";
                            : external_link(config, http_client, "https://fenhl.net/disc", "disclaimer").await.unwrap();
                            : " • ";
                            : external_link(config, http_client, "https://status.fenhl.net/", "status").await.unwrap();
                            : " • ";
                            : external_link(config, http_client, "https://github.com/fenhl/molecule-db", "source code").await.unwrap();
                        }
                        p {
                            : "Special thanks to panic whose ";
                            : external_link(config, http_client, "http://critelli.technology/transmogrification.html", "Tonic of Transmogrification reagent builder").await.unwrap();
                            : " served as the basis for parts of this website's code!";
                        }
                    }
                    : scripts;
                }
            }
        };
        if let Some(git_commit_hash) = GIT_COMMIT_HASH {
            StaticPageResponse::Stale {
                cache_control: Header::new(http::header::CACHE_CONTROL.as_str(), "no-cache"), // ensure etag is validated on each request
                etag: Header::new(http::header::ETAG.as_str(), format!("\"{git_commit_hash}\"")),
                body,
            }
        } else {
            StaticPageResponse::Untagged(body)
        }
    }
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
enum IndexError {
    #[error(transparent)] Json(#[from] serde_json::Error),
}

#[rocket::get("/?<m>&<b>")]
async fn index(config: &State<Config>, http_client: &State<reqwest::Client>, if_none_match: IfNoneMatch<'_>, m: form::Result<'_, FormMolecule>, b: Option<NonZero<u8>>) -> Result<(Status, StaticPageResponse), IndexError> {
    let (js_state, min_radius, radius, molecule_too_large, errors) = match m.clone() {
        Ok(FormMolecule(molecule)) => match JsState::new(molecule, b) {
            Ok((js_state, min_radius)) => (Some(js_state), min_radius, b.unwrap_or_else(|| min_radius.max(NonZero::new(5).unwrap())), false, form::Errors::default()),
            Err(MoleculeTooLarge { radius, min_radius }) => (None, min_radius, radius, true, form::Errors::default()),
        },
        Err(errors) => (None, NonZero::<u8>::MIN, b.unwrap_or_else(|| NonZero::new(5).unwrap()), false, errors),
    };
    Ok((if errors.is_empty() { Status::Ok } else { errors.status() }, static_page(config, http_client, if_none_match, Tab::MoleculeInput, false, "Opus Magnum Molecule Database", html! {
        main(style = "flex-direction: column;") {
            @if molecule_too_large {
                div(class = "emphasized-section") : "molecule does not fit onto canvas";
            }
            @for e in errors {
                @if e.kind != form::error::ErrorKind::Missing {
                    div(class = "emphasized-section") : e;
                }
            }
            h3 : "ENTER MOLECULE TO LOOK UP";
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
    }, html! {
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
    }).await))
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
                .filter_map(|(puzzle, i, o, name)| Some((puzzle, InOut::new(i, o), name?)))
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
    appearances: Vec<AppearanceV2>,
    permalink: String,
    rust_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceV2 {
    puzzle: &'static str,
    url: rocket::http::uri::Origin<'static>,
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
                .filter_map(|(puzzle, i, o, name)| Some((puzzle, InOut::new(i, o), name?)))
                .map(|(puzzle, inout, name)| AppearanceV2 {
                    puzzle: puzzle.as_str(),
                    url: uri!(puzzle::get(puzzle)),
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
    appearances: Vec<AppearanceV2>,
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
            response.appearances = appearances.into_iter().map(|(puzzle, i, o, name)| AppearanceV2 {
                puzzle: puzzle.as_str(),
                url: uri!(puzzle::get(puzzle)),
                inout: InOut::new(i, o),
                name,
            }).collect();
            break
        }
    }
    Ok(Json(response))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoleculeResponseV4 {
    appearances: Vec<AppearanceV4>,
    min_radius: NonZero<u8>,
    permalink: String,
    rust_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceV4 {
    puzzle: &'static str,
    url: rocket::http::uri::Origin<'static>,
    i: u8,
    o: u8,
    name: Option<&'static str>,
}

#[rocket::post("/api/v4/molecule-from-state", format = "json", data = "<state>")]
fn molecule_from_state_v4(state: Json<JsState>) -> Result<Json<MoleculeResponseV4>, Status> {
    let molecule = Molecule::try_from(state.0).map_err(|()| Status::BadRequest)?;
    let mut permalink = Vec::default();
    FormMolecule(molecule.clone()).write_sync(&mut permalink).map_err(|_| Status::BadRequest)?;
    let mut response = MoleculeResponseV4 {
        appearances: Vec::default(),
        min_radius: molecule.min_radius().map_err(|_| Status::BadRequest)?,
        permalink: BASE64.encode(permalink),
        rust_code: format!("{:?}", Unparse(&molecule)),
    };
    for (iter_molecule, appearances) in molecules::molecules() {
        if iter_molecule == molecule {
            response.appearances = appearances.into_iter().map(|(puzzle, i, o, name)| AppearanceV4 {
                puzzle: puzzle.as_str(),
                url: uri!(puzzle::get(puzzle)),
                i, o, name,
            }).collect();
            break
        }
    }
    Ok(Json(response))
}

#[rocket::get("/molecules")]
async fn molecules_list(config: &State<Config>, http_client: &State<reqwest::Client>, if_none_match: IfNoneMatch<'_>) -> StaticPageResponse {
    static_page(config, http_client, if_none_match, Tab::MoleculeList, false, "Opus Magnum Molecule Database", html! {
        main {
            @for (idx, (molecule, appearances)) in molecules::molecules().into_iter().sorted_by_key(|(_, appearances)| {
                let mut names = appearances.iter().filter_map(|(_, _, _, name)| *name).collect_vec();
                names.sort_unstable();
                names.dedup();
                (names.is_empty(), names)
            }).enumerate() {
                div {
                    h3 {
                        @if appearances.iter().all(|(_, _, _, name)| name.is_none()) {
                            span(class = "muted") : "unnamed";
                        } else {
                            : appearances.iter().filter_map(|(_, _, _, name)| *name).sorted_unstable().dedup().join("/");
                        }
                    }
                    a(href = uri!(index(Ok(FormMolecule(molecule.clone())), _))) : molecule.draw(&format!("product{idx}"));
                }
            }
        }
    }, html! {}).await
}

#[derive(clap::Parser)]
struct Args {
    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    EncodingStats,
    Validate,
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
enum Error {
    #[error(transparent)] GitCheckout(#[from] gix::clone::checkout::main_worktree::Error),
    #[error(transparent)] GitClone(#[from] gix::clone::Error),
    #[error(transparent)] GitCloneFetch(#[from] gix::clone::fetch::Error),
    #[error(transparent)] GitConnect(#[from] gix::remote::connect::Error),
    #[error(transparent)] GitFetch(#[from] gix::remote::fetch::Error),
    #[error(transparent)] GitFindRemote(#[from] gix::remote::find::existing::Error),
    #[error(transparent)] GitOpen(#[from] gix::open::Error),
    #[error(transparent)] GitPrepareFetch(#[from] gix::remote::fetch::prepare::Error),
    #[error(transparent)] HexIndexEncode(#[from] proto::v0::HexIndexEncodeError),
    #[error(transparent)] Http(#[from] reqwest::Error),
    #[error(transparent)] Rocket(#[from] rocket::Error),
    #[error(transparent)] Url(#[from] url::ParseError),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Write(#[from] async_proto::WriteError),
    #[cfg(feature = "night")]
    #[error("failed to locate config file")]
    MissingConfig,
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
            Subcommand::EncodingStats => {
                let mut buf = Vec::default();
                let molecules = molecules::molecules();
                let num_molecules = molecules.len();
                let mut min_v0_len = usize::MAX;
                let mut max_v0_len = 0;
                let mut total_v0_lens = 0;
                let mut min_v62_len = usize::MAX;
                let mut max_v62_len = 0;
                let mut total_v62_lens = 0;
                for (molecule, _) in molecules {
                    buf.clear();
                    proto::v62::write(&molecule, &mut buf).at_unknown()?;
                    min_v62_len = min_v62_len.min(buf.len() + 1);
                    max_v62_len = max_v62_len.max(buf.len() + 1);
                    total_v62_lens += buf.len() + 1;
                    buf.clear();
                    proto::v0::ProtocolMolecule::try_from(&FormMolecule(molecule))?.write_sync(&mut buf)?;
                    min_v0_len = min_v0_len.min(buf.len());
                    max_v0_len = max_v0_len.max(buf.len());
                    total_v0_lens += buf.len();
                }
                println!("v0: min = {min_v0_len}, avg = {}, max = {max_v0_len}", total_v0_lens as f64 / num_molecules as f64);
                println!("v62: min = {min_v62_len}, avg = {}, max = {max_v62_len}", total_v62_lens as f64 / num_molecules as f64);
            }
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
                'puzzles: for puzzle in all::<Puzzle>() {
                    let omsim_rs::data::Puzzle { reagents, products, .. } = parse_puzzle(&match puzzle.source() {
                        puzzle::Source::Tutorial | puzzle::Source::OfficialNonLb { .. } | puzzle::Source::Computation { .. } | puzzle::Source::CritelliComputation { .. } => continue, // nothing to validate against
                        puzzle::Source::Critelli { url_part, .. } => {
                            let (host, port, selector) = critelli_puzzles.remove(url_part).expect(&format!("missing critelli puzzle: {url_part}"));
                            let mut tcp_client = TcpStream::connect((host, port)).await.at_unknown()?;
                            tcp_client.write_all(selector.as_ref()).await.at_unknown()?;
                            tcp_client.write_all(b"\r\n").await.at_unknown()?;
                            let mut buf = Vec::default();
                            tcp_client.read_to_end(&mut buf).await.at_unknown()?;
                            buf
                        }
                        puzzle::Source::Official { zlbb_id, .. } | puzzle::Source::Zlbb { zlbb_id, .. } => fs::read(zlbb_path.join(format!("src/main/resources/om/puzzle/{zlbb_id}.puzzle"))).await?,
                        puzzle::Source::Other { .. } => fs::read(format!("assets/puzzle/{}.puzzle", puzzle.url_part())).await?,
                    }).map_err(|e| Error::ParsePuzzle(e))?;
                    let mut found_reagents = Vec::default();
                    let mut found_products = Vec::default();
                    for (molecule, puzzles) in molecules::molecules() {
                        if let Some((_, _, _, name)) = puzzles.iter().find(|(iter_puzzle, i, _, _)| *iter_puzzle == puzzle && *i > 0) {
                            if molecule.atoms.values().filter(|atom| **atom == Atom::Repeat).count() > 1 { continue 'puzzles } //TODO update omsim molecule decoder for polymers with multiple repeat atoms?
                            assert!(reagents.iter().any(|iter_molecule| iter_molecule.normalized() == molecule), "{} ({molecule:?}) not found in upstream version of {puzzle}", if let Some(name) = name { format!("reagent {name:?}") } else { format!("unnamed reagent") });
                            if !found_reagents.iter().any(|iter_molecule| *iter_molecule == molecule) {
                                found_reagents.push(molecule.clone());
                            }
                        }
                        if let Some((_, _, _, name)) = puzzles.iter().find(|(iter_puzzle, _, o, _)| *iter_puzzle == puzzle && *o > 0) {
                            if molecule.atoms.values().filter(|atom| **atom == Atom::Repeat).count() > 1 { continue 'puzzles } //TODO update omsim molecule decoder for polymers with multiple repeat atoms?
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
        let config = Config::load().await?;
        #[cfg(feature = "night")] {
            let panic_config = config.clone();
            let default_panic_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = night_report_sync(&panic_config, &format!("/dev/dushanbe/mol/panic"), Some("thread panic"));
                default_panic_hook(info)
            }));
        }
        let http_client = reqwest::Client::builder()
            .user_agent(concat!("MoleculeDb/", env!("CARGO_PKG_VERSION"), " (https://github.com/fenhl/molecule-db)"))
            .timeout(Duration::from_secs(30))
            .use_rustls_tls()
            .hickory_dns(true)
            .https_only(true)
            .build()?;
        rocket::custom(rocket::Config::figment().merge(rocket::Config {
            log_level: Some(rocket::config::Level::ERROR),
            ..rocket::Config::default()
        }).merge(("port", 24821))) //TODO report issue for lack of typed interface to set port, see https://github.com/rwf2/Rocket/commit/fd294049c784cb52680a423616fadc29d57fa25b
        .mount("/", rocket::routes![
            index,
            molecule_from_state_v1,
            molecule_from_state_v2,
            molecule_from_state_v3,
            molecule_from_state_v4,
            molecules_list,
            puzzle::index,
            puzzle::get,
        ])
        .mount("/static", FileServer::without_index("assets/static"))
        .manage(config)
        .manage(http_client)
        .launch().await?;
    }
    Ok(())
}
