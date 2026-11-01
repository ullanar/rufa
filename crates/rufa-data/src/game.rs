use std::path::Path;

use crate::catalog::Catalog;
use crate::{Bundle, InfoSde, Names, Sde, eve};

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Sde(crate::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "{e}"),
            LoadError::Sde(e) => write!(f, "{e}"),
        }
    }
}

pub struct Game {
    pub sde: Sde<'static>,
    pub names: Option<Names<'static>>,
    pub catalog: Catalog,
}

impl Game {
    pub fn load(dir: &Path) -> Result<&'static Game, LoadError> {
        let bundle = Bundle::open(dir).map_err(LoadError::Io)?;
        let (sde_bytes, names_bytes) = bundle.leak();
        let sde = Sde::new(sde_bytes).map_err(LoadError::Sde)?;
        let names = match names_bytes {
            Some(bytes) => Some(Names::new(bytes).map_err(LoadError::Sde)?),
            None => None,
        };
        let raw =
            eve::root_as_sde(sde_bytes).map_err(|e| LoadError::Sde(crate::Error::InvalidSde(e)))?;
        let catalog = Catalog::build(&raw);
        Ok(Box::leak(Box::new(Game {
            sde,
            names,
            catalog,
        })))
    }

    pub fn info(&'static self) -> InfoSde<'static> {
        InfoSde::new(&self.sde)
    }

    pub fn build_number(&self) -> i32 {
        self.sde.build_number()
    }

    pub fn type_name(&self, type_id: i32) -> &str {
        self.sde.get_type(type_id).map(|t| t.name()).unwrap_or("")
    }
}
