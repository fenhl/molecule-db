use {
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        io::{
            self,
            prelude::*,
        },
    },
    async_proto::Protocol as _,
    bitstream_io::{
        BigEndian,
        BitRead as _,
        BitReader,
        BitWrite as _,
        BitWriter,
    },
    itertools::Itertools as _,
    omsim_rs::data::*,
};

fn read_varint(stream: &mut BitReader<&mut impl Read, BigEndian>) -> io::Result<i32> {
    let mut n = 0u32;
    for i in 0..8 {
        n |= stream.read::<4, u32>()? << (4 * i);
        if i < 7 && !stream.read_bit()? { break }
    }
    Ok((n >> 1) as i32 ^ -((n & 1) as i32))
}

fn write_varint(sink: &mut BitWriter<&mut impl Write, BigEndian>, n: i32) -> io::Result<()> {
    let mut n = ((n << 1) ^ (n >> 31)) as u32;
    sink.write::<4, _>(n & 0b1111)?;
    for _ in 1..8 {
        n >>= 4;
        if n == 0 {
            sink.write_bit(false)?;
            break
        }
        sink.write_bit(true)?;
        sink.write::<4, _>(n & 0b1111)?;
    }
    Ok(())
}

#[test]
fn varint_roundtrip() {
    let mut buf = Vec::default();
    for n in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        buf.clear();
        let mut sink = BitWriter::<_, BigEndian>::new(&mut buf);
        write_varint(&mut sink, n).unwrap();
        sink.byte_align().unwrap();
        let mut buf = &buf[..];
        let mut stream = BitReader::<_, BigEndian>::new(&mut buf);
        let read = read_varint(&mut stream).unwrap();
        assert_eq!(read, n, "{read:#x} != {n:#x}");
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid partial triplex bond end offset")]
struct PartialTriplexOffset;

pub(super) fn read(stream: &mut impl Read) -> io::Result<Molecule> {
    let mut stream = BitReader::<_, BigEndian>::new(stream);
    let mut atoms = HashMap::default();
    let mut coords = match stream.read::<3, u8>()? {
        0 => HexIndex { q: 1, r: 0 },
        1 => HexIndex { q: 0, r: 1 },
        2 => HexIndex { q: -1, r: 1 },
        3 => HexIndex { q: -1, r: 0 },
        4 => HexIndex { q: 0, r: -1 },
        5 => HexIndex { q: 1, r: -1 },
        6 => HexIndex { q: read_varint(&mut stream)?, r: read_varint(&mut stream)? },
        7 => HexIndex { q: 0, r: 0 }, // special case: for the first atom, a value of 7 means a position of 0,0 instead of end of atom list
        _ => unreachable!(),
    };
    loop {
        atoms.insert(coords, super::v0::ProtocolAtom::read_sync(&mut &[stream.read::<4, u8>()?][..])?.into());
        coords = match stream.read::<3, u8>()? {
            0 => coords + HexIndex { q: 1, r: 0 },
            1 => coords + HexIndex { q: 0, r: 1 },
            2 => coords + HexIndex { q: -1, r: 1 },
            3 => coords + HexIndex { q: -1, r: 0 },
            4 => coords + HexIndex { q: 0, r: -1 },
            5 => coords + HexIndex { q: 1, r: -1 },
            6 => HexIndex { q: read_varint(&mut stream)?, r: read_varint(&mut stream)? },
            7 => break,
            _ => unreachable!(),
        };
    }
    let mut bonds = HashSet::default();
    let mut coords = HexIndex::default();
    loop {
        let mut ty = BondType::Normal;
        let end_offset = match stream.read::<3, u8>()? {
            0 => HexIndex { q: 1, r: 0 },
            1 => HexIndex { q: 0, r: 1 },
            2 => HexIndex { q: -1, r: 1 },
            3 => HexIndex { q: -1, r: 0 },
            4 => HexIndex { q: 0, r: -1 },
            5 => HexIndex { q: 1, r: -1 },
            6 => {
                ty = BondType::Triplex { red: true, black: true, yellow: true };
                match stream.read::<3, u8>()? {
                    0 => HexIndex { q: 1, r: 0 },
                    1 => HexIndex { q: 0, r: 1 },
                    2 => HexIndex { q: -1, r: 1 },
                    3 => HexIndex { q: -1, r: 0 },
                    4 => HexIndex { q: 0, r: -1 },
                    5 => HexIndex { q: 1, r: -1 },
                    6 => {
                        ty = BondType::Triplex {
                            red: false,
                            black: stream.read_bit()?,
                            yellow: stream.read_bit()?,
                        };
                        match stream.read::<3, u8>()? {
                            0 => HexIndex { q: 1, r: 0 },
                            1 => HexIndex { q: 0, r: 1 },
                            2 => HexIndex { q: -1, r: 1 },
                            3 => HexIndex { q: -1, r: 0 },
                            4 => HexIndex { q: 0, r: -1 },
                            5 => HexIndex { q: 1, r: -1 },
                            6 | 7 => return Err(io::Error::new(io::ErrorKind::InvalidData, PartialTriplexOffset)),
                            _ => unreachable!(),
                        }
                    }
                    7 => {
                        ty = BondType::Triplex {
                            red: true,
                            black: stream.read_bit()?,
                            yellow: stream.read_bit()?,
                        };
                        match stream.read::<3, u8>()? {
                            0 => HexIndex { q: 1, r: 0 },
                            1 => HexIndex { q: 0, r: 1 },
                            2 => HexIndex { q: -1, r: 1 },
                            3 => HexIndex { q: -1, r: 0 },
                            4 => HexIndex { q: 0, r: -1 },
                            5 => HexIndex { q: 1, r: -1 },
                            6 | 7 => return Err(io::Error::new(io::ErrorKind::InvalidData, PartialTriplexOffset)),
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                }
            }
            7 => break,
            _ => unreachable!(),
        };
        if stream.read_bit()? {
            coords = HexIndex { q: read_varint(&mut stream)?, r: read_varint(&mut stream)? };
        }
        bonds.insert(Bond {
            start: coords,
            end: coords + end_offset,
            ty,
        });
        coords += end_offset;
    }
    Ok(Molecule { atoms, bonds })
}

pub(crate) fn write(Molecule { atoms, bonds }: &Molecule, sink: &mut impl Write) -> io::Result<()> {
    let mut sink = BitWriter::<_, BigEndian>::new(sink);
    let mut atoms = atoms.iter().map(|(&pos, &atom)| (pos, atom)).collect_vec();
    atoms.sort_unstable_by_key(|(pos, _)| (pos.q.abs() + pos.r.abs(), pos.q, pos.r)); //TODO order atoms for more efficient encoding
    let mut atoms = atoms.into_iter();
    if let Some((mut coords, atom)) = atoms.next() {
        match coords {
            HexIndex { q: 1, r: 0 } => sink.write_const::<3, 0>()?,
            HexIndex { q: 0, r: 1 } => sink.write_const::<3, 1>()?,
            HexIndex { q: -1, r: 1 } => sink.write_const::<3, 2>()?,
            HexIndex { q: -1, r: 0 } => sink.write_const::<3, 3>()?,
            HexIndex { q: 0, r: -1 } => sink.write_const::<3, 4>()?,
            HexIndex { q: 1, r: -1 } => sink.write_const::<3, 5>()?,
            HexIndex { q: 0, r: 0 } => sink.write_const::<3, 7>()?,
            HexIndex { q, r } => {
                sink.write_const::<3, 6>()?;
                write_varint(&mut sink, q)?;
                write_varint(&mut sink, r)?;
            }
        }
        let mut buf = [0u8];
        super::v0::ProtocolAtom::from(atom).write_sync(&mut &mut buf[..])?;
        sink.write::<4, _>(buf[0])?;
        for (new_coords, atom) in atoms {
            match new_coords - coords {
                HexIndex { q: 1, r: 0 } => sink.write_const::<3, 0>()?,
                HexIndex { q: 0, r: 1 } => sink.write_const::<3, 1>()?,
                HexIndex { q: -1, r: 1 } => sink.write_const::<3, 2>()?,
                HexIndex { q: -1, r: 0 } => sink.write_const::<3, 3>()?,
                HexIndex { q: 0, r: -1 } => sink.write_const::<3, 4>()?,
                HexIndex { q: 1, r: -1 } => sink.write_const::<3, 5>()?,
                _ => {
                    sink.write_const::<3, 6>()?;
                    write_varint(&mut sink, new_coords.q)?;
                    write_varint(&mut sink, new_coords.r)?;
                }
            }
            let mut buf = [0u8];
            super::v0::ProtocolAtom::from(atom).write_sync(&mut &mut buf[..])?;
            sink.write::<4, _>(buf[0])?;
            coords = new_coords;
        }
        sink.write_const::<3, 7>()?;
    }
    let mut bonds = bonds.iter().copied().collect_vec();
    bonds.sort_unstable_by_key(|bond| (bond.start.q.abs() + bond.start.r.abs() + bond.end.q.abs() + bond.end.r.abs(), bond.start.q, bond.start.q, bond.end.q, bond.end.r)); //TODO order bonds for more efficient encoding
    let mut coords = HexIndex::default();
    for Bond { start, end, ty } in bonds {
        match ty {
            BondType::Normal => {}
            BondType::Triplex { red, black, yellow } => {
                sink.write_const::<3, 6>()?;
                if !red || !black || !yellow {
                    sink.write_const::<2, 3>()?;
                    sink.write_bit(red)?;
                    sink.write_bit(black)?;
                    sink.write_bit(yellow)?;
                }
            }
        }
        match (end.q - start.q, end.r - start.r) {
            (1, 0) => sink.write_const::<3, 0>()?,
            (0, 1) => sink.write_const::<3, 1>()?,
            (-1, 1) => sink.write_const::<3, 2>()?,
            (-1, 0) => sink.write_const::<3, 3>()?,
            (0, -1) => sink.write_const::<3, 4>()?,
            (1, -1) => sink.write_const::<3, 5>()?,
            _ => panic!("invalid bond"),
        }
        if start == coords {
            sink.write_bit(false)?;
        } else {
            sink.write_bit(true)?;
            write_varint(&mut sink, start.q)?;
            write_varint(&mut sink, start.r)?;
        }
        coords = end;
    }
    sink.write_const::<3, 7>()?;
    sink.byte_align()?;
    Ok(())
}

#[test]
fn test_roundtrip() {
    let mut buf = Vec::default();
    for (molecule, names) in crate::molecules::molecules() {
        buf.clear();
        write(&molecule, &mut buf).expect(&names.iter().map(|&(_, _, _, name)| name.unwrap_or("(unnamed)")).join("/"));
        let read = read(&mut &buf[..]).unwrap();
        if molecule != read {
            eprintln!("{}", names.iter().map(|&(_, _, _, name)| name.unwrap_or("(unnamed)")).format("/"));
        }
        assert_eq!(molecule, read, "{} (read: {:?})", names.iter().map(|&(_, _, _, name)| name.unwrap_or("(unnamed)")).join("/"), crate::unparse::Unparse(&read));
    }
}
