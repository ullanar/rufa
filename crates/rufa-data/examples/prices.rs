fn main() {
    let types: Vec<i32> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let start = std::time::Instant::now();
    match rufa_data::price::fetch(rufa_data::price::REGION_THE_FORGE, &types) {
        Ok(prices) => {
            for (id, price) in prices {
                println!("{id}\t{price:.2}");
            }
            println!("in {:?}", start.elapsed());
        }
        Err(e) => println!("error: {e}"),
    }
}
