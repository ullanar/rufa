use rufa_data::{Bundle, eve};

fn main() {
    let dir = rufa_data::data_dir();
    let bundle = Bundle::open(&dir).expect("sde.dat in data dir");
    let (bytes, _) = bundle.leak();
    let raw = eve::root_as_sde(bytes).expect("valid sde.dat");
    let what = std::env::args().nth(1).unwrap_or_default();
    match what.as_str() {
        "attrs" => {
            for a in raw.dogma_attributes().into_iter().flatten() {
                println!(
                    "{}\t{}\t{}\tunit={}\tcat={}",
                    a.id(),
                    a.name(),
                    a.display_name().unwrap_or(""),
                    a.unit_id(),
                    a.category_id()
                );
            }
        }
        "effects" => {
            for e in raw.dogma_effects().into_iter().flatten() {
                println!("{}\t{}\t{:?}", e.id(), e.name(), e.effect_category());
            }
        }
        "type" => {
            let id: i32 = std::env::args().nth(2).unwrap().parse().unwrap();
            let sde = rufa_data::Sde::new(bytes).unwrap();
            let t = sde.get_type(id).unwrap();
            println!(
                "{} group={} cat={} market={}",
                t.name(),
                t.group_id(),
                t.category_id(),
                t.market_group_id()
            );
            for a in t.dogma_attributes().into_iter().flatten() {
                let name = sde
                    .get_dogma_attribute(a.attribute_id())
                    .map(|x| x.name().to_string())
                    .unwrap_or_default();
                println!("  {}\t{}\t{}", a.attribute_id(), name, a.value());
            }
            for e in t.dogma_effects().into_iter().flatten() {
                let name = sde
                    .get_dogma_effect(e.effect_id())
                    .map(|x| x.name().to_string())
                    .unwrap_or_default();
                println!(
                    "  effect {}\t{}\tdefault={}",
                    e.effect_id(),
                    name,
                    e.is_default()
                );
            }
        }
        "groups" => {
            for c in raw.categories().into_iter().flatten() {
                println!("cat {}\t{}\t{}", c.id(), c.name(), c.published());
            }
            for g in raw.groups().into_iter().flatten() {
                println!(
                    "grp {}\t{}\tcat={}\tn={}",
                    g.id(),
                    g.name(),
                    g.category_id(),
                    g.type_ids().map(|v| v.len()).unwrap_or(0)
                );
            }
        }
        "market" => {
            for g in raw.market_groups().into_iter().flatten() {
                println!(
                    "{}\t{}\tparent={}\tn={}",
                    g.id(),
                    g.name(),
                    g.parent_group_id(),
                    g.type_ids().map(|v| v.len()).unwrap_or(0)
                );
            }
        }
        "id" => {
            let sde = rufa_data::Sde::new(bytes).unwrap();
            let name: Vec<String> = std::env::args().skip(2).collect();
            let name = name.join(" ");
            match sde.type_name_to_id(&name) {
                Some(id) => println!("{id}\t{name}"),
                None => println!("not found: {name}"),
            }
        }
        "group" => {
            let id: i32 = std::env::args().nth(2).unwrap().parse().unwrap();
            for g in raw.groups().into_iter().flatten().filter(|g| g.id() == id) {
                for tid in g.type_ids().into_iter().flatten() {
                    let sde = rufa_data::Sde::new(bytes).unwrap();
                    if let Some(t) = sde.get_type(tid) {
                        println!("{}\t{}\tpublished={}", t.id(), t.name(), t.published());
                    }
                }
            }
        }
        _ => println!("usage: dump attrs|effects|type ID|group ID|groups|market"),
    }
}
