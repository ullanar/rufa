use std::path::{Path, PathBuf};

use rufa_data::{Bundle, InfoSde, Sde};
use rufa_sim::attrs::AttrIds;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("rufa-dedup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn game() -> Option<(Bundle, AttrIds)> {
    let dir = std::env::var_os("RUFA_DATA_DIR")?;
    let bundle = Bundle::open(Path::new(&dir)).ok()?;
    let ids = {
        let sde: Sde<'_> = bundle.sde().ok()?;
        AttrIds::resolve(&sde).ok()?
    };
    Some((bundle, ids))
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pyfa-saveddata.db")
}

/// Imports every fit of the report the way the app does, skipping what the
/// index or the saved identities already hold. Returns how many were written.
fn import(info: &InfoSde, ids: &AttrIds, dir: &Path, db: &Path) -> usize {
    let report = rufa_store::pyfa::read(db, info, ids).unwrap();
    let mut known = rufa_store::dedup::imported_ids(dir, db);
    let mut here = rufa_store::dedup::saved_identities(info, dir).unwrap_or_default();
    let mut saved = 0;
    for imported in &report.fits {
        let identity = rufa_store::dedup::identity(info, &imported.fit).ok();
        if known.contains(&imported.pyfa_id)
            || identity.as_ref().is_some_and(|id| here.contains(id))
        {
            continue;
        }
        let path = rufa_store::save_fit(info, dir, &imported.fit, None, "hull").unwrap();
        rufa_store::dedup::remember_import(dir, db, imported.pyfa_id, &path).unwrap();
        known.insert(imported.pyfa_id);
        if let Some(id) = identity {
            here.insert(id);
        }
        saved += 1;
    }
    saved
}

#[test]
fn three_imports_in_a_row_leave_one_copy() {
    let Some((bundle, ids)) = game() else { return };
    let sde = bundle.sde().unwrap();
    let info = InfoSde::new(&sde);
    let scratch = Scratch::new("three");
    let db = fixture();

    assert_eq!(import(&info, &ids, &scratch.0, &db), 2, "the first import");
    assert_eq!(import(&info, &ids, &scratch.0, &db), 0, "the second import");
    assert_eq!(import(&info, &ids, &scratch.0, &db), 0, "the third import");
    assert_eq!(rufa_store::list_fits(&scratch.0).unwrap().len(), 2);
}

#[test]
fn an_edited_fit_is_not_imported_again() {
    let Some((bundle, ids)) = game() else { return };
    let sde = bundle.sde().unwrap();
    let info = InfoSde::new(&sde);
    let scratch = Scratch::new("edited");
    let db = fixture();
    import(&info, &ids, &scratch.0, &db);

    // Change a saved fit, so only the written-down Pyfa id can match it.
    let path = rufa_store::list_fits(&scratch.0).unwrap()[0].clone();
    let mut fit = rufa_store::load_fit(&info, &path).unwrap();
    fit.items.pop();
    rufa_store::save_fit(&info, &scratch.0, &fit, Some(&path), "hull").unwrap();

    assert_eq!(import(&info, &ids, &scratch.0, &db), 0);
    assert_eq!(rufa_store::list_fits(&scratch.0).unwrap().len(), 2);
}

#[test]
fn a_deleted_fit_comes_back() {
    let Some((bundle, ids)) = game() else { return };
    let sde = bundle.sde().unwrap();
    let info = InfoSde::new(&sde);
    let scratch = Scratch::new("deleted");
    let db = fixture();
    import(&info, &ids, &scratch.0, &db);

    let path = rufa_store::list_fits(&scratch.0).unwrap()[0].clone();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(import(&info, &ids, &scratch.0, &db), 1);
    assert_eq!(rufa_store::list_fits(&scratch.0).unwrap().len(), 2);
}

#[test]
fn the_original_outlives_its_numbered_copies() {
    let Some((bundle, ids)) = game() else { return };
    let sde = bundle.sde().unwrap();
    let info = InfoSde::new(&sde);
    let scratch = Scratch::new("copies");
    import(&info, &ids, &scratch.0, &fixture());

    // What three presses of the import button used to leave behind.
    let first = rufa_store::list_fits(&scratch.0).unwrap()[0].clone();
    let stem = first.file_stem().unwrap().to_string_lossy().into_owned();
    let dir = first.parent().unwrap();
    for n in [2, 3] {
        std::fs::copy(&first, dir.join(format!("{stem}-{n}.esf"))).unwrap();
    }

    let groups = rufa_store::dedup::duplicates(&info, &scratch.0).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].keep, first);
    assert_eq!(groups[0].remove.len(), 2);

    assert_eq!(
        rufa_store::dedup::remove_duplicates(&scratch.0, &groups).unwrap(),
        2
    );
    let left = rufa_store::list_fits(&scratch.0).unwrap();
    assert_eq!(left.len(), 2, "the other imported fit stays");
    assert!(left.contains(&first));
}

#[test]
fn two_names_over_the_same_items_are_two_fits() {
    let Some((bundle, ids)) = game() else { return };
    let sde = bundle.sde().unwrap();
    let info = InfoSde::new(&sde);
    let scratch = Scratch::new("names");
    import(&info, &ids, &scratch.0, &fixture());

    let first = rufa_store::list_fits(&scratch.0).unwrap()[0].clone();
    let mut fit = rufa_store::load_fit(&info, &first).unwrap();
    fit.name = Some("the spare one".to_string());
    rufa_store::save_fit(&info, &scratch.0, &fit, None, "hull").unwrap();

    assert!(
        rufa_store::dedup::duplicates(&info, &scratch.0)
            .unwrap()
            .is_empty(),
        "same items, different names, two fits"
    );
}
