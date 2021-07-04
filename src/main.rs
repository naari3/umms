use std::env;

use anyhow::Result;
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Asset {
    name: String,
    hash: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let sql_path = args.get(1).expect("Please specify meta file path.");
    println!("{}", sql_path);
    let conn = Connection::open(sql_path)?;

    let mut stmt = conn.prepare("SELECT n, h FROM a LIMIT 5;")?;
    let asset_iter = stmt.query_map([], |r| {
        Ok(Asset {
            name: r.get(0)?,
            hash: r.get(1)?,
        })
    })?;

    for asset in asset_iter {
        println!("{:?}", asset.unwrap());
    }

    Ok(())
}
