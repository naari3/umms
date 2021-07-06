use async_process::Command;
use async_std::{
    fs::{copy, create_dir_all},
    io::{self, prelude::WriteExt},
    path::Path,
};
use indicatif::ProgressBar;
use regex::Regex;
use std::env;

use anyhow::Result;
use async_std::task;
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Asset {
    name: String,
    hash: String,
}

#[async_std::main]
async fn main() -> Result<()> {
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

    let dest = Path::new("dest");
    create_dir_all(dest).await?;

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
            let asset_dir_path = Path::new(&sql_path).parent().unwrap();

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
    let pb = ProgressBar::new(tasks.len() as u64);

    println!("ready...");
    let pb = task::block_on(async move {
        for task in tasks.into_iter() {
            task.await.unwrap();
            pb.inc(1);
        }
        pb
    });
    pb.finish_with_message("done");

    // TODO: refactor
    println!("awb -> wav");
    let mut tasks = vec![];
    create_dir_all(dest.join("wav")).await?;

    println!("collect");
    let epb = ProgressBar::new(tasks.len() as u64);
    for entry in glob::glob("./dest/**/*.awb").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                create_dir_all(
                    dest.join("wav")
                        .join(path.clone().parent().unwrap().strip_prefix("dest").unwrap()),
                )
                .await?;
                let output = Command::new(".\\vgmstream-win\\test.exe")
                    .args(&["-m", path.to_str().unwrap()])
                    .output()
                    .await?;
                let metainfo_string = String::from_utf8_lossy(&output.stdout).to_string();
                let re = Regex::new(r"stream count: (\d+)")?;
                let caps = match re.captures(&metainfo_string) {
                    Some(caps) => caps,
                    None => {
                        tasks.push(async_std::task::spawn(async move {
                            Command::new(".\\vgmstream-win\\test.exe")
                                .args(&[
                                    path.clone().to_str().unwrap(),
                                    "-o",
                                    &format!(
                                        "dest/wav/{}/?n.wav",
                                        path.clone()
                                            .parent()
                                            .unwrap()
                                            .strip_prefix("dest")
                                            .unwrap()
                                            .to_string_lossy()
                                    ),
                                ])
                                .output()
                                .await
                                .unwrap();
                        }));
                        continue;
                    }
                };
                // multi stream strip
                let stream_count: u32 = caps.get(1).unwrap().as_str().parse()?;

                for i in 1..stream_count + 1 {
                    let path = path.clone();
                    tasks.push(async_std::task::spawn(async move {
                        Command::new(".\\vgmstream-win\\test.exe")
                            .args(&[
                                "-s",
                                &i.to_string(),
                                path.clone().to_str().unwrap(),
                                "-o",
                                &format!(
                                    "dest/wav/{}/?n_?s.wav",
                                    path.clone()
                                        .parent()
                                        .unwrap()
                                        .strip_prefix("dest")
                                        .unwrap()
                                        .to_string_lossy()
                                ),
                            ])
                            .output()
                            .await
                            .unwrap();
                    }));
                }
            }
            Err(e) => println!("{:?}", e),
        }
        epb.inc(1);
    }
    epb.finish_with_message("done");

    println!("found {} resources", tasks.len());
    let vgm_pb = ProgressBar::new(tasks.len() as u64);

    let vgm_pb = task::block_on(async move {
        for task in tasks.into_iter() {
            task.await;
            vgm_pb.inc(1);
        }
        vgm_pb
    });
    vgm_pb.finish_with_message("done");

    Ok(())
}
