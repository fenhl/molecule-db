use {
    std::collections::{
        HashMap,
        HashSet,
    },
    async_proto::Protocol,
    omsim_rs::data::*,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Protocol)]
struct ProtocolHexIndex(u8);

impl From<ProtocolHexIndex> for HexIndex {
    fn from(ProtocolHexIndex(packed): ProtocolHexIndex) -> Self {
        Self {
            q: ((packed & 0b1111_0000) >> 4).into(),
            r: ((packed & 0b0000_1111) >> 0).into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("hex index does not fit into u8")]
struct HexIndexEncodeError;

impl From<HexIndexEncodeError> for async_proto::WriteErrorKind {
    fn from(e: HexIndexEncodeError) -> Self {
        Self::Custom(e.to_string())
    }
}

impl TryFrom<HexIndex> for ProtocolHexIndex {
    type Error = HexIndexEncodeError;

    fn try_from(HexIndex { q, r }: HexIndex) -> Result<Self, Self::Error> {
        if q & 0b1111 != q || r & 0b1111 != r { return Err(HexIndexEncodeError) }
        Ok(Self((q as u8) << 4 | (r as u8) << 0))
    }
}

#[derive(Protocol)]
enum ProtocolAtom { Salt, Air, Earth, Fire, Water, Quicksilver, Vitae, Mors, Lead, Tin, Iron, Copper, Silver, Gold, Quintessence, Repeat }

impl From<ProtocolAtom> for Atom {
    fn from(value: ProtocolAtom) -> Self {
        match value {
            ProtocolAtom::Salt => Self::Salt,
            ProtocolAtom::Air => Self::Air,
            ProtocolAtom::Earth => Self::Earth,
            ProtocolAtom::Fire => Self::Fire,
            ProtocolAtom::Water => Self::Water,
            ProtocolAtom::Quicksilver => Self::Quicksilver,
            ProtocolAtom::Vitae => Self::Vitae,
            ProtocolAtom::Mors => Self::Mors,
            ProtocolAtom::Lead => Self::Lead,
            ProtocolAtom::Tin => Self::Tin,
            ProtocolAtom::Iron => Self::Iron,
            ProtocolAtom::Copper => Self::Copper,
            ProtocolAtom::Silver => Self::Silver,
            ProtocolAtom::Gold => Self::Gold,
            ProtocolAtom::Quintessence => Self::Quintessence,
            ProtocolAtom::Repeat => Self::Repeat,
        }
    }
}

impl From<Atom> for ProtocolAtom {
    fn from(value: Atom) -> Self {
        match value {
            Atom::Salt => Self::Salt,
            Atom::Air => Self::Air,
            Atom::Earth => Self::Earth,
            Atom::Fire => Self::Fire,
            Atom::Water => Self::Water,
            Atom::Quicksilver => Self::Quicksilver,
            Atom::Vitae => Self::Vitae,
            Atom::Mors => Self::Mors,
            Atom::Lead => Self::Lead,
            Atom::Tin => Self::Tin,
            Atom::Iron => Self::Iron,
            Atom::Copper => Self::Copper,
            Atom::Silver => Self::Silver,
            Atom::Gold => Self::Gold,
            Atom::Quintessence => Self::Quintessence,
            Atom::Repeat => Self::Repeat,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Protocol)]
struct ProtocolBond {
    start: ProtocolHexIndex,
    packed: u8,
}

impl From<ProtocolBond> for Bond {
    fn from(ProtocolBond { start, packed }: ProtocolBond) -> Self {
        let red = packed & 0b0000_0100 != 0;
        let black = packed & 0b0000_0010 != 0;
        let yellow = packed & 0b0000_0001 != 0;
        Self {
            start: start.into(),
            end: HexIndex::from(start) + HexIndex { q: 1, r: 0 }.rotated(HexIndex::default(), HexRotation::from_unsigned((packed & 0b0011_1000) >> 3)),
            ty: if !red && !black && !yellow {
                BondType::Normal
            } else {
                BondType::Triplex { red, black, yellow }
            },
        }
    }
}

impl<'a> TryFrom<&'a Bond> for ProtocolBond {
    type Error = HexIndexEncodeError;

    fn try_from(Bond { start, end, ty }: &'a Bond) -> Result<Self, Self::Error> {
        let rotation = match (end.q - start.q, end.r - start.r) {
            (1, 0) => 0,
            (0, 1) => 1,
            (-1, 1) => 2,
            (-1, 0) => 3,
            (0, -1) => 4,
            (1, -1) => 5,
            _ => panic!("invalid bond"),
        };
        let (red, black, yellow) = match *ty {
            BondType::Normal => (0, 0, 0),
            BondType::Triplex { red, black, yellow } => (red.into(), black.into(), yellow.into()),
        };
        Ok(Self {
            start: (*start).try_into()?,
            packed: rotation << 3 | red << 2 | black << 1 | yellow << 0,
        })
    }
}

#[derive(Protocol)]
struct ProtocolMolecule {
    #[async_proto(max_len = 61)]
    atoms: HashMap<ProtocolHexIndex, ProtocolAtom>,
    #[async_proto(max_len = 156)]
    bonds: HashSet<ProtocolBond>,
}

impl From<ProtocolMolecule> for FormMolecule {
    fn from(ProtocolMolecule { atoms, bonds }: ProtocolMolecule) -> Self {
        Self(Molecule {
            atoms: atoms.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
            bonds: bonds.into_iter().map(|bond| bond.into()).collect(),
        })
    }
}

impl<'a> TryFrom<&'a FormMolecule> for ProtocolMolecule {
    type Error = HexIndexEncodeError;

    fn try_from(FormMolecule(Molecule { atoms, bonds }): &'a FormMolecule) -> Result<Self, Self::Error> {
        Ok(Self {
            atoms: atoms.iter().map(|(k, v)| Ok(((*k).try_into()?, (*v).into()))).collect::<Result<_, _>>()?,
            bonds: bonds.iter().map(|bond| bond.try_into()).collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Protocol)]
#[async_proto(via = ProtocolMolecule)]
pub(crate) struct FormMolecule(pub(crate) Molecule);
