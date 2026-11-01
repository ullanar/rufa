use std::path::Path;

use rufa_data::{Bundle, InfoSde};
use rufa_sim::attrs::AttrIds;
use rufa_sim::{Slot, State};

#[test]
fn reads_a_pyfa_database() {
    let Some(dir) = std::env::var_os("RUFA_DATA_DIR") else {
        return;
    };
    let bundle = Bundle::open(Path::new(&dir)).unwrap();
    let sde = bundle.sde().unwrap();
    let ids = AttrIds::resolve(&sde).unwrap();
    let info = InfoSde::new(&sde);
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pyfa-saveddata.db");
    let report = rufa_store::pyfa::read(&fixture, &info, &ids).unwrap();
    assert_eq!(report.skipped, 1, "the fit with an unknown hull is skipped");
    assert_eq!(report.fits.len(), 2);

    let rifter = &report.fits[0];
    assert_eq!(rifter.fit.name.as_deref(), Some("Pyfa Rifter"));
    assert_eq!(rifter.fit.ship.type_id, 587);
    let items = &rifter.fit.items;
    let lows: Vec<_> = items
        .iter()
        .filter(|it| matches!(it.slot, Slot::Low(_)))
        .collect();
    assert_eq!(lows.len(), 2);
    assert_eq!(lows[0].slot, Slot::Low(0));
    let highs: Vec<_> = items
        .iter()
        .filter(|it| matches!(it.slot, Slot::High(_)))
        .collect();
    assert_eq!(
        highs.len(),
        3,
        "the projected web and the dummy slot are left out"
    );
    assert_eq!(highs[0].state, State::Active);
    assert_eq!(highs[1].state, State::Overload);
    assert_eq!(highs[2].state, State::Offline);
    assert_eq!(highs[0].charge.as_ref().map(|c| c.type_id), Some(185));
    assert!(
        items
            .iter()
            .any(|it| it.slot == Slot::Medium(0) && it.type_id == 438)
    );
    let drones: Vec<_> = items
        .iter()
        .filter(|it| it.slot == Slot::DroneBay)
        .collect();
    assert_eq!(drones.len(), 2);
    assert_eq!((drones[0].quantity, drones[0].state), (2, State::Active));
    assert_eq!((drones[1].quantity, drones[1].state), (1, State::Offline));
    assert!(
        items
            .iter()
            .any(|it| it.slot == Slot::Cargo && it.quantity == 1000)
    );
    assert!(
        items
            .iter()
            .any(|it| matches!(it.slot, Slot::Implant(_)) && it.type_id == 19540)
    );
    assert!(
        items
            .iter()
            .any(|it| matches!(it.slot, Slot::Booster(_)) && it.type_id == 9950)
    );
    assert_eq!(rifter.unknown_types, 0);

    let kestrel = &report.fits[1];
    assert_eq!(kestrel.unknown_types, 1, "an unknown module id is counted");
    assert!(
        kestrel
            .fit
            .items
            .iter()
            .any(|it| matches!(it.slot, Slot::FighterTube(_)) && it.quantity == 6)
    );
}

#[test]
fn sqlite_reader_handles_the_schema_table() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pyfa-saveddata.db");
    let db = rufa_store::sqlite::Database::open(&fixture).unwrap();
    let fits = db.table("fits").unwrap();
    assert_eq!(fits.rows.len(), 3);
    assert_eq!(
        fits.get(&fits.rows[0], "name").as_str(),
        Some("Pyfa Rifter")
    );
    assert_eq!(fits.get(&fits.rows[0], "ID").as_i64(), Some(1));
    assert!(db.table("nothing").is_err());
}
