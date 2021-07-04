use async_std::{
    fs::{copy, create_dir_all},
    io::{self, prelude::WriteExt},
};
use std::env;

use anyhow::Result;
use async_std::task;
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Asset {
    name: String,
    hash: String,
}

fn main() -> Result<()> {
    let sql_path = env::args().nth(1).expect("Please specify meta file path.");

    let conn = Connection::open(&sql_path)?;

    let mut stmt = conn.prepare("SELECT n, h FROM a;")?;
    println!("found {} rows", stmt.column_count());
    let asset_iter = stmt.query_map([], |r| {
        Ok(Asset {
            name: r.get(0)?,
            hash: r.get(1)?,
        })
    })?;

    let dest = std::path::Path::new("dest");
    std::fs::create_dir_all(dest)?;

    let mut tasks = vec![];
    for asset in asset_iter {
        let asset = asset?;
        // Because I don't know how to use these files
        if asset.name.starts_with("//") {
            continue;
        }
        let sql_path = sql_path.clone();
        tasks.push(async_std::task::spawn(async move {
            let asset_dest_path = dest.join(&asset.name);
            let asset_dest_dir = asset_dest_path.parent().unwrap();
            let asset_dir_path = std::path::Path::new(&sql_path).parent().unwrap();

            create_dir_all(asset_dest_dir).await?;
            let asset_path = asset_dir_path
                .join("dat")
                .join(&asset.hash[..2])
                .join(&asset.hash);
            match copy(asset_path, asset_dest_path).await {
                Ok(_) => Ok(()),
                Err(e) => match e.kind() {
                    io::ErrorKind::NotFound => {
                        io::stdout()
                            .write(&format!("Not found asset: {:?}\n", &asset).as_bytes())
                            .await?;
                        Ok(())
                    }
                    _ => return Err(e),
                },
            }
        }));
    }

    println!("found {} resources", tasks.len());

    println!("ready...");
    task::block_on(async move {
        for task in tasks.into_iter() {
            task.await.unwrap();
        }
    });

    Ok(())
}
