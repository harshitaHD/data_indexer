mod api;
mod config;
mod db;
mod indexer;
mod types;

use clap::{Parser, Subcommand};
use config::load;
use db::Db;
use indexer::Indexer;
use tokio::task;
use tracing_subscriber::EnvFilter;

/// Alfred/JARVIS CLI: run indexer, run API, or query netflow once.
#[derive(Parser)]
#[command(name="polygon-netflow-indexer", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run both the indexer (WS) and the HTTP API
    Run,
    /// Run only the HTTP API (assuming indexer elsewhere)
    Api,
    /// Print current netflow (stdout)
    Netflow,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cli = Cli::parse();
    let cfg = load()?;
    let db = Db::connect(&cfg.db_url).await?;
    db.migrate().await?;

    match cli.cmd {
        Cmd::Run => {
            let indexer_cfg = cfg.clone();
            let api_cfg = cfg.clone();
            let db2 = db.clone();

            let t_indexer = task::spawn(async move {
                let idx = Indexer::new(indexer_cfg, db).await?;
                idx.run().await
            });

            let t_api = task::spawn(async move {
                api::serve(api_cfg, db2).await
            });

            let (a,b) = tokio::join!(t_indexer, t_api);
            a??;
            b??;
        }
        Cmd::Api => {
            api::serve(cfg, db).await?;
        }
        Cmd::Netflow => {
            if let Some((cin, cout, cnet, bn)) = db.fetch_netflow(1, cfg.pol_token).await? {
                println!("Exchange : Binance");
                println!("Token    : {:?}", cfg.pol_token);
                println!("In       : {}", cin);
                println!("Out      : {}", cout);
                println!("Net      : {}", cnet);
                println!("Updated@ : block {}", bn);
            } else {
                println!("No netflow yet.");
            }
        }
    }
    Ok(())
}