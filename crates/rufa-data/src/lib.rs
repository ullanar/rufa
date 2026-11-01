pub mod catalog;
pub mod esi;
mod game;
pub mod price;
pub mod update;

use std::path::{Path, PathBuf};

pub use catalog::{AttrLabel, Catalog, HullGroup, MarketNode, TypeRef};
pub use game::{Game, LoadError};

pub use esf_data::{Error, Info, InfoEsf, InfoName, InfoNameSde, InfoSde, Names, Sde, eve};

pub const SDE_FILE: &str = "sde.dat";
pub const NAMES_FILE: &str = "names.dat";
pub const TEXTS_FILE: &str = "texts.dat";

pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("RUFA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("rufa");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".local").join("share").join("rufa")
}

pub struct Bundle {
    sde: Box<[u8]>,
    names: Option<Box<[u8]>>,
}

impl Bundle {
    pub fn open(dir: &Path) -> std::io::Result<Bundle> {
        let sde = std::fs::read(dir.join(SDE_FILE))?.into_boxed_slice();
        let names = match std::fs::read(dir.join(NAMES_FILE)) {
            Ok(bytes) => Some(bytes.into_boxed_slice()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(Bundle { sde, names })
    }

    pub fn sde(&self) -> Result<Sde<'_>, Error> {
        Sde::new(&self.sde)
    }

    pub fn names(&self) -> Option<Result<Names<'_>, Error>> {
        self.names.as_deref().map(Names::new)
    }

    pub fn leak(self) -> (&'static [u8], Option<&'static [u8]>) {
        (Box::leak(self.sde), self.names.map(|b| &*Box::leak(b)))
    }
}
