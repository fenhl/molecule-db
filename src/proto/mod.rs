use {
    std::{
        io::prelude::*,
        pin::Pin,
    },
    async_proto::{
        ErrorContext,
        Protocol,
        ReadError,
        ReadErrorKind,
        WriteError,
        WriteErrorKind,
    },
    omsim_rs::data::*,
    tokio::io::{
        AsyncRead,
        AsyncWrite,
    },
};

pub(crate) mod v0;
// versions 1 through 61 are aliases of version 0 since the version byte doubles as the number of atoms in that version
pub(crate) mod v62;

#[derive(Clone)]
pub(crate) struct FormMolecule(pub(crate) Molecule);

impl Protocol for FormMolecule {
    fn read<'a, R: AsyncRead + Unpin + Send + 'a>(_: &'a mut R) -> Pin<Box<dyn Future<Output = Result<Self, ReadError>> + Send + 'a>> {
        unimplemented!()
    }

    fn write<'a, W: AsyncWrite + Unpin + Send + 'a>(&'a self, _: &'a mut W) -> Pin<Box<dyn Future<Output = Result<(), WriteError>> + Send + 'a>> {
        unimplemented!()
    }

    fn read_sync(stream: &mut impl Read) -> Result<Self, ReadError> {
        Ok(match u8::read_sync(stream)? {
            first_byte @ 0..=61 => v0::ProtocolMolecule::read_sync(&mut Read::chain(&[first_byte][..], stream))?.into(),
            62 => Self(v62::read(stream).map_err(|e| ReadError { context: ErrorContext::Custom("v62::read".to_owned()), kind: ReadErrorKind::Custom(e.to_string()) })?),
            version => return Err(ReadError { context: ErrorContext::Custom("FormMolecule::read_sync".to_owned()), kind: ReadErrorKind::UnknownVariant8(version) }),
        })
    }

    fn write_sync(&self, sink: &mut impl Write) -> Result<(), WriteError> {
        62u8.write_sync(sink)?;
        v62::write(&self.0, sink).map_err(|e| WriteError { context: ErrorContext::Custom("v62::write".to_owned()), kind: WriteErrorKind::Custom(e.to_string()) })?;
        Ok(())
    }
}
