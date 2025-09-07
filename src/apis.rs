use crate::{config::AppConfig, db::Db};
use axum::{routing::get, Router, Json};
use ethers::types::Address;
use serde::Serialize;
use std::{net::SocketAddr, str::FromStr};

#[derive(Serialize)]
struct NetflowResp {
    exchange: &'static str,
    token: String,
    cumulative_in: f64,
    cumulative_out: f64,
    cumulative_net: f64,
    last_updated_block: i64,
}

pub async fn serve(cfg: AppConfig, db: Db) -> anyhow::Result<()> {
    let token = cfg.pol_token;
    let app = Router::new().route("/netflow", get({
        let db = db.clone();
        move || {
            let db = db.clone();
            async move {
                let row = db.fetch_netflow(1, token).await.ok().flatten();
                if let Some((cin, cout, cnet, bn)) = row {
                    Json(NetflowResp {
                        exchange: "binance",
                        token: format!("{:?}", token),
                        cumulative_in: cin,
                        cumulative_out: cout,
                        cumulative_net: cnet,
                        last_updated_block: bn,
                    })
                } else {
                    Json(NetflowResp {
                        exchange: "binance",
                        token: format!("{:?}", token),
                        cumulative_in: 0.0,
                        cumulative_out: 0.0,
                        cumulative_net: 0.0,
                        last_updated_block: 0,
                    })
                }
            }
        }
    }));

    let addr: SocketAddr = cfg.http_listen_addr.parse()?;
    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}
